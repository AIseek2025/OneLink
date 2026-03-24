//! Profile API + dev-only event ingestion.

use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use axum::{
    extract::State,
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use chrono::{SecondsFormat, Utc};
use onelink_event_envelope::EventEnvelope;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::Config;
use crate::projection::{
    facts_from_resolved_items, merge_facts_dedupe, refresh_derived_profile_fields, MemoryResolveInput,
    StoredFact, TraitSnapshot,
};

const INTERNAL_TOKEN_HEADER: &str = "x-internal-token";

fn empty_as_none(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

fn verify_internal_token(headers: &HeaderMap, expected: &str) -> Result<(), StatusCode> {
    if expected.is_empty() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    let ok = headers
        .get(INTERNAL_TOKEN_HEADER)
        .and_then(|v| v.to_str().ok())
        == Some(expected);
    if ok {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

pub fn router(state: Arc<ProfileState>) -> Router {
    Router::new()
        .route("/api/v1/profile/me", get(get_me))
        .route("/api/v1/profile/me/completion", get(get_completion))
        .route("/internal/events/receive", post(receive_event))
        .with_state(state)
}

#[derive(Debug)]
pub struct ProfileState {
    pub config: Config,
    pub client: reqwest::Client,
    profiles: Mutex<std::collections::HashMap<String, ProfileDoc>>,
}

#[derive(Debug, Clone)]
struct ProfileDoc {
    user_id: String,
    display_name: String,
    avatar_url: String,
    headline: String,
    bio: String,
    city_level_location: String,
    languages: Vec<String>,
    is_searchable: bool,
    allow_discovery: bool,
    updated_at: String,
    /// 由记忆投影写入的可见摘要行（dev MVP；与 facts 派生一致）
    memory_highlights: Vec<String>,
    applied_projection_ids: HashSet<String>,
    /// Phase A 结构化事实层（唯一来源：profile.memory_projection 消费链）
    facts: Vec<StoredFact>,
    /// 由 facts 聚合的 trait 层（与 facts 同步刷新）
    traits: TraitSnapshot,
}

#[derive(Debug, Serialize)]
struct FactView {
    fact_type: String,
    value: String,
    /// Phase B：启发式置信度（0~1）
    confidence: f64,
    /// Phase B：溯源 memory 行 id
    source_memory_id: String,
    /// Phase B：溯源消息 id（有则返回）
    #[serde(skip_serializing_if = "Option::is_none")]
    source_message_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct MeResponse {
    user_id: String,
    display_name: String,
    avatar_url: String,
    headline: String,
    bio: String,
    city_level_location: String,
    languages: Vec<String>,
    is_searchable: bool,
    allow_discovery: bool,
    updated_at: String,
    /// Phase A：结构化事实（只增字段，旧客户端可忽略）
    facts: Vec<FactView>,
    /// Phase A：trait 聚合
    traits: TraitSnapshot,
}

#[derive(Debug, Serialize)]
struct CompletionResponse {
    completion_rate: f64,
    required_dimensions: Vec<String>,
    filled_dimensions: Vec<String>,
    missing_dimensions: Vec<String>,
}

async fn identity_me_value(
    state: &ProfileState,
    headers: &HeaderMap,
) -> Result<Value, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "missing Authorization".to_string(),
        ))?;
    let url = format!(
        "{}/api/v1/identity/me",
        state.config.identity_service_base_url
    );
    let response = state
        .client
        .get(url)
        .header(AUTHORIZATION, auth)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("identity: {e}")))?;
    if response.status().is_server_error() {
        return Err((
            StatusCode::BAD_GATEWAY,
            format!("identity-service status {}", response.status()),
        ));
    }
    if !response.status().is_success() {
        return Err((StatusCode::UNAUTHORIZED, "invalid or expired token".to_string()));
    }
    response
        .json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("identity json: {e}")))
}

fn user_id_from_me(me: &Value) -> Result<String, (StatusCode, String)> {
    me.get("user_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or((
            StatusCode::BAD_GATEWAY,
            "identity.me missing user_id".to_string(),
        ))
}

async fn get_me(
    State(state): State<Arc<ProfileState>>,
    headers: HeaderMap,
) -> Result<Json<MeResponse>, (StatusCode, String)> {
    let me = identity_me_value(&state, &headers).await?;
    let user_id = user_id_from_me(&me)?;

    let doc = {
        let g = state.profiles.lock().expect("profiles mutex poisoned");
        g.get(&user_id).cloned()
    };

    let base = doc.unwrap_or_else(|| ProfileDoc {
        user_id: user_id.clone(),
        display_name: String::new(),
        avatar_url: String::new(),
        headline: String::new(),
        bio: String::new(),
        city_level_location: String::new(),
        languages: vec![],
        is_searchable: true,
        allow_discovery: true,
        updated_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
        memory_highlights: vec![],
        applied_projection_ids: HashSet::new(),
        facts: vec![],
        traits: TraitSnapshot::default(),
    });

    let facts_view: Vec<FactView> = base
        .facts
        .iter()
        .map(|f| FactView {
            fact_type: f.fact_type.clone(),
            value: f.value.clone(),
            confidence: f.confidence,
            source_memory_id: f.source_memory_id.clone(),
            source_message_id: f.source_message_id.clone(),
        })
        .collect();

    Ok(Json(MeResponse {
        user_id: base.user_id,
        // 返回真实资料值，避免与 completion 的“是否已填写”口径冲突。
        display_name: base.display_name,
        avatar_url: base.avatar_url,
        headline: base.headline,
        bio: base.bio,
        city_level_location: base.city_level_location,
        languages: if base.languages.is_empty() {
            vec!["zh".to_string()]
        } else {
            base.languages
        },
        is_searchable: base.is_searchable,
        allow_discovery: base.allow_discovery,
        updated_at: base.updated_at,
        facts: facts_view,
        traits: base.traits,
    }))
}

async fn get_completion(
    State(state): State<Arc<ProfileState>>,
    headers: HeaderMap,
) -> Result<Json<CompletionResponse>, (StatusCode, String)> {
    let me = identity_me_value(&state, &headers).await?;
    let user_id = user_id_from_me(&me)?;

    let doc = {
        let g = state.profiles.lock().expect("profiles mutex poisoned");
        g.get(&user_id).cloned()
    };

    let required = vec![
        "display_name".to_string(),
        "interest_tags".to_string(),
        "connection_goals".to_string(),
        "current_location".to_string(),
        "communication_preferences".to_string(),
    ];
    let mut filled = vec![];
    let mut missing = vec![];

    let d = doc.unwrap_or_default();
    if !d.display_name.is_empty() {
        filled.push("display_name".to_string());
    } else {
        missing.push("display_name".to_string());
    }
    if !d.traits.interest_tags.is_empty() {
        filled.push("interest_tags".to_string());
    } else {
        missing.push("interest_tags".to_string());
    }
    if !d.traits.connection_goal_tags.is_empty() {
        filled.push("connection_goals".to_string());
    } else {
        missing.push("connection_goals".to_string());
    }
    let has_location = !d.city_level_location.is_empty() || d.traits.location_label.is_some();
    if has_location {
        filled.push("current_location".to_string());
    } else {
        missing.push("current_location".to_string());
    }
    if !d.traits.communication_preferences.is_empty() {
        filled.push("communication_preferences".to_string());
    } else {
        missing.push("communication_preferences".to_string());
    }

    let rate = (filled.len() as f64 / required.len() as f64).clamp(0.0, 1.0);

    Ok(Json(CompletionResponse {
        completion_rate: (rate * 100.0).round() / 100.0,
        required_dimensions: required.clone(),
        filled_dimensions: filled.clone(),
        missing_dimensions: missing,
    }))
}

/// 若 `projection_id` 已处理过，返回 `false` 且不合并事实（幂等）。
fn apply_projection_batch(
    entry: &mut ProfileDoc,
    projection_id: &str,
    batch_facts: Vec<StoredFact>,
) -> bool {
    if entry.applied_projection_ids.contains(projection_id) {
        return false;
    }
    entry.applied_projection_ids.insert(projection_id.to_string());
    merge_facts_dedupe(&mut entry.facts, batch_facts);
    refresh_derived_profile_fields(
        &entry.facts,
        &mut entry.city_level_location,
        &mut entry.headline,
        &mut entry.bio,
        &mut entry.memory_highlights,
        &mut entry.traits,
    );
    entry.updated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    true
}

async fn receive_event(
    State(state): State<Arc<ProfileState>>,
    headers: HeaderMap,
    Json(envelope): Json<EventEnvelope>,
) -> StatusCode {
    if let Err(code) = verify_internal_token(&headers, &state.config.internal_shared_secret) {
        return code;
    }

    tracing::info!(
        event_name = %envelope.event_name,
        producer = %envelope.producer,
        full_envelope = %serde_json::to_string(&envelope).unwrap_or_default(),
        "profile-service POST /internal/events/receive"
    );

    if envelope.event_name != "profile.memory_projection.requested.v1" {
        return StatusCode::ACCEPTED;
    }

    let payload = &envelope.payload;
    let user_id = payload
        .get("user_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let projection_id = payload
        .get("projection_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let memory_ids: Vec<String> = payload
        .get("memory_ids")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    if user_id.is_empty() || projection_id.is_empty() {
        tracing::warn!("projection payload missing user_id or projection_id");
        return StatusCode::BAD_REQUEST;
    }

    let resolve_url = format!(
        "{}/internal/memory/resolve",
        state.config.context_service_base_url
    );
    let resolved: MemoryResolveResponse = match state
        .client
        .post(resolve_url)
        .header(
            INTERNAL_TOKEN_HEADER,
            state.config.internal_shared_secret.as_str(),
        )
        .json(&json!({ "memory_ids": memory_ids }))
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => match r.json().await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, "memory resolve decode failed");
                return StatusCode::BAD_GATEWAY;
            }
        },
        Ok(r) => {
            tracing::warn!(status = %r.status(), "memory resolve failed");
            return StatusCode::BAD_GATEWAY;
        }
        Err(e) => {
            tracing::warn!(error = %e, "memory resolve request failed");
            return StatusCode::BAD_GATEWAY;
        }
    };

    let inputs: Vec<MemoryResolveInput> = resolved
        .items
        .iter()
        .map(|i| MemoryResolveInput {
            memory_id: i.memory_id.clone(),
            content: i.content.clone(),
            network_type: i.network_type.clone(),
            keywords: i.keywords.clone(),
            temporal_state: empty_as_none(&i.temporal_state),
            preference_polarity: empty_as_none(&i.preference_polarity),
            source_message_id: empty_as_none(&i.source_message_id),
        })
        .collect();

    let batch_facts = facts_from_resolved_items(&inputs);

    {
        let mut g = state.profiles.lock().expect("profiles mutex poisoned");
        let entry = g.entry(user_id.clone()).or_insert_with(|| ProfileDoc {
            user_id: user_id.clone(),
            display_name: String::new(),
            avatar_url: String::new(),
            headline: String::new(),
            bio: String::new(),
            city_level_location: String::new(),
            languages: vec![],
            is_searchable: true,
            allow_discovery: true,
            updated_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            memory_highlights: vec![],
            applied_projection_ids: HashSet::new(),
            facts: vec![],
            traits: TraitSnapshot::default(),
        });

        if !apply_projection_batch(entry, &projection_id, batch_facts) {
            tracing::info!(projection_id = %projection_id, "duplicate projection skipped");
            return StatusCode::ACCEPTED;
        }
    }

    tracing::info!(user_id = %user_id, "profile updated from memory projection");
    StatusCode::ACCEPTED
}

#[derive(Debug, Deserialize)]
struct MemoryResolveResponse {
    items: Vec<MemoryResolveItem>,
}

#[derive(Debug, Deserialize)]
struct MemoryResolveItem {
    #[allow(dead_code)]
    memory_id: String,
    content: String,
    network_type: String,
    #[serde(default)]
    keywords: Vec<String>,
    #[serde(default)]
    temporal_state: String,
    #[serde(default)]
    preference_polarity: String,
    #[allow(dead_code)]
    #[serde(default)]
    source_message_id: String,
}

impl Default for ProfileDoc {
    fn default() -> Self {
        Self {
            user_id: String::new(),
            display_name: String::new(),
            avatar_url: String::new(),
            headline: String::new(),
            bio: String::new(),
            city_level_location: String::new(),
            languages: vec![],
            is_searchable: true,
            allow_discovery: true,
            updated_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            memory_highlights: vec![],
            applied_projection_ids: HashSet::new(),
            facts: vec![],
            traits: TraitSnapshot::default(),
        }
    }
}

impl ProfileState {
    pub fn new(config: Config) -> Arc<Self> {
        Arc::new(Self {
            config,
            client: reqwest::Client::new(),
            profiles: Mutex::new(std::collections::HashMap::new()),
        })
    }
}

#[cfg(test)]
mod projection_apply_tests {
    use super::*;

    use crate::projection::{facts_from_resolved_items, MemoryResolveInput, StoredFact, FACT_GOAL};

    #[test]
    fn duplicate_projection_id_does_not_merge_twice() {
        let mut entry = ProfileDoc {
            user_id: "u1".into(),
            display_name: String::new(),
            avatar_url: String::new(),
            headline: String::new(),
            bio: String::new(),
            city_level_location: String::new(),
            languages: vec![],
            is_searchable: true,
            allow_discovery: true,
            updated_at: String::new(),
            memory_highlights: vec![],
            applied_projection_ids: HashSet::new(),
            facts: vec![],
            traits: TraitSnapshot::default(),
        };
        let batch = vec![StoredFact {
            fact_type: FACT_GOAL.to_string(),
            value: "希望认识合伙人".to_string(),
            source_memory_id: "mem-1".to_string(),
            source_message_id: None,
            confidence: 0.77,
        }];
        assert!(apply_projection_batch(&mut entry, "proj-a", batch.clone()));
        assert_eq!(entry.facts.len(), 1);
        assert!(!apply_projection_batch(&mut entry, "proj-a", batch));
        assert_eq!(entry.facts.len(), 1);
    }

    #[test]
    fn distinct_projection_ids_both_merge_with_dedupe() {
        let mut entry = ProfileDoc {
            user_id: "u1".into(),
            display_name: String::new(),
            avatar_url: String::new(),
            headline: String::new(),
            bio: String::new(),
            city_level_location: String::new(),
            languages: vec![],
            is_searchable: true,
            allow_discovery: true,
            updated_at: String::new(),
            memory_highlights: vec![],
            applied_projection_ids: HashSet::new(),
            facts: vec![],
            traits: TraitSnapshot::default(),
        };
        let inputs = vec![MemoryResolveInput {
            memory_id: "m1".into(),
            content: "希望认识投资人".into(),
            network_type: "experience".into(),
            keywords: vec![],
            temporal_state: None,
            preference_polarity: None,
            source_message_id: None,
        }];
        let f1 = facts_from_resolved_items(&inputs);
        assert!(apply_projection_batch(&mut entry, "p1", f1.clone()));
        assert!(apply_projection_batch(&mut entry, "p2", f1));
        assert_eq!(entry.facts.len(), 1, "same fact key deduped across projections");
    }
}
