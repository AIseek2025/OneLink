//! Profile API + dev-only event ingestion.

use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use axum::{
    body::Bytes,
    extract::State,
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{SecondsFormat, Utc};
use onelink_event_envelope::EventEnvelope;
use onelink_internal_auth::{verify_internal_token, INTERNAL_TOKEN_HEADER};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::Config;
use crate::projection::{
    facts_from_resolved_items, merge_facts_dedupe, refresh_derived_profile_fields,
    MemoryResolveInput, StoredFact, TraitSnapshot,
};
use crate::store::postgres::PostgresStore;

fn empty_as_none(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

pub fn router(state: Arc<ProfileState>) -> Router {
    Router::new()
        .route("/api/v1/profile/me", get(get_me).patch(patch_me))
        .route("/api/v1/profile/me/completion", get(get_completion))
        .route("/internal/events/receive", post(receive_event))
        .with_state(state)
}

#[derive(Debug)]
pub struct ProfileState {
    pub config: Config,
    pub client: reqwest::Client,
    profiles: Mutex<std::collections::HashMap<String, ProfileDoc>>,
    pub pg: Option<Arc<PostgresStore>>,
}

#[derive(Debug, Clone)]
pub struct ProfileDoc {
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

struct PgSnapshot {
    user_id: String,
    display_name: Option<String>,
    avatar_url: Option<String>,
    headline: Option<String>,
    bio: Option<String>,
    city_level_location: Option<String>,
    languages: Vec<String>,
    is_searchable: bool,
    allow_discovery: bool,
    facts: Vec<StoredFact>,
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
        return Err((
            StatusCode::UNAUTHORIZED,
            "invalid or expired token".to_string(),
        ));
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

    let base = state.read_profile(&user_id).await;

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

const DISPLAY_NAME_MAX_LEN: usize = 128;
const AVATAR_URL_MAX_LEN: usize = 2048;
const CITY_LEVEL_LOCATION_MAX_LEN: usize = 256;
const LANGUAGES_MAX_COUNT: usize = 20;
const LANGUAGE_TAG_MAX_LEN: usize = 64;

#[derive(Debug, Deserialize)]
struct PatchMeRequest {
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    avatar_url: Option<String>,
    #[serde(default)]
    city_level_location: Option<String>,
    #[serde(default)]
    languages: Option<Vec<String>>,
    #[serde(default)]
    is_searchable: Option<bool>,
    #[serde(default)]
    allow_discovery: Option<bool>,
}

impl PatchMeRequest {
    fn validate(&self) -> Result<(), (StatusCode, String)> {
        if let Some(ref v) = self.display_name {
            if v.len() > DISPLAY_NAME_MAX_LEN {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!("display_name exceeds {DISPLAY_NAME_MAX_LEN} characters"),
                ));
            }
        }
        if let Some(ref v) = self.avatar_url {
            if v.len() > AVATAR_URL_MAX_LEN {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!("avatar_url exceeds {AVATAR_URL_MAX_LEN} characters"),
                ));
            }
        }
        if let Some(ref v) = self.city_level_location {
            if v.len() > CITY_LEVEL_LOCATION_MAX_LEN {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!("city_level_location exceeds {CITY_LEVEL_LOCATION_MAX_LEN} characters"),
                ));
            }
        }
        if let Some(ref langs) = self.languages {
            if langs.len() > LANGUAGES_MAX_COUNT {
                return Err((
                    StatusCode::BAD_REQUEST,
                    format!("languages exceeds {LANGUAGES_MAX_COUNT} items"),
                ));
            }
            for tag in langs {
                if tag.len() > LANGUAGE_TAG_MAX_LEN {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        format!("language tag exceeds {LANGUAGE_TAG_MAX_LEN} characters"),
                    ));
                }
            }
        }
        Ok(())
    }
}

async fn patch_me(
    State(state): State<Arc<ProfileState>>,
    headers: HeaderMap,
    Json(body): Json<PatchMeRequest>,
) -> Result<Json<MeResponse>, (StatusCode, String)> {
    body.validate()?;

    let me = identity_me_value(&state, &headers).await?;
    let user_id = user_id_from_me(&me)?;

    {
        let mut g = state.profiles.lock().expect("profiles mutex poisoned");
        let entry = g.entry(user_id.clone()).or_insert_with(ProfileDoc::default);
        entry.user_id = user_id.clone();
        if let Some(ref v) = body.display_name {
            entry.display_name = v.clone();
        }
        if let Some(ref v) = body.avatar_url {
            entry.avatar_url = v.clone();
        }
        if let Some(ref v) = body.city_level_location {
            entry.city_level_location = v.clone();
        }
        if let Some(ref v) = body.languages {
            entry.languages = v.clone();
        }
        if let Some(v) = body.is_searchable {
            entry.is_searchable = v;
        }
        if let Some(v) = body.allow_discovery {
            entry.allow_discovery = v;
        }
        entry.updated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    }

    if let Some(pg) = &state.pg {
        let snapshot = {
            let g = state.profiles.lock().expect("profiles mutex poisoned");
            g.get(&user_id).map(|e| {
                (
                    if e.display_name.is_empty() {
                        None
                    } else {
                        Some(e.display_name.clone())
                    },
                    if e.avatar_url.is_empty() {
                        None
                    } else {
                        Some(e.avatar_url.clone())
                    },
                    if e.headline.is_empty() {
                        None
                    } else {
                        Some(e.headline.clone())
                    },
                    if e.bio.is_empty() {
                        None
                    } else {
                        Some(e.bio.clone())
                    },
                    if e.city_level_location.is_empty() {
                        None
                    } else {
                        Some(e.city_level_location.clone())
                    },
                    e.languages.clone(),
                    e.is_searchable,
                    e.allow_discovery,
                )
            })
        };
        if let Some((dn, av, hl, bi, cl, langs, searchable, discoverable)) = snapshot {
            if let Err(e) = pg
                .upsert_profile(
                    &user_id,
                    dn.as_deref(),
                    av.as_deref(),
                    hl.as_deref(),
                    bi.as_deref(),
                    cl.as_deref(),
                    &langs,
                    searchable,
                    discoverable,
                )
                .await
            {
                tracing::error!(error = %e, user_id = %user_id, "postgres upsert_profile (patch_me) failed — data will not persist");
            }
        }
    }

    let base = state.read_profile(&user_id).await;

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

    let d = state.read_profile(&user_id).await;

    let required = vec![
        "display_name".to_string(),
        "interest_tags".to_string(),
        "connection_goals".to_string(),
        "current_location".to_string(),
        "communication_preferences".to_string(),
    ];
    let mut filled = vec![];
    let mut missing = vec![];
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
    entry
        .applied_projection_ids
        .insert(projection_id.to_string());
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
    body: Bytes,
) -> impl IntoResponse {
    if let Err(code) = verify_internal_token(&headers, &state.config.internal_shared_secret) {
        return code;
    }

    let envelope: EventEnvelope = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "invalid event envelope JSON");
            return StatusCode::BAD_REQUEST;
        }
    };

    tracing::info!(
        event_name = %envelope.event_name,
        producer = %envelope.producer,
        full_envelope = %serde_json::to_string(&envelope).unwrap_or_default(),
        "profile-service POST /internal/events/receive"
    );

    if envelope.event_name == "profile.fact.created.v1" {
        let payload = &envelope.payload;
        let user_id = payload
            .get("user_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if user_id.is_empty() {
            tracing::warn!("profile.fact.created payload missing user_id");
            return StatusCode::BAD_REQUEST;
        }
        let fact_type = payload
            .get("fact_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let fact_key = payload
            .get("fact_key")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let fact_value_json = payload.get("fact_value_json").cloned().unwrap_or(json!({}));
        let source_type = payload
            .get("source_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let confidence = payload.get("confidence").and_then(|v| v.as_f64());
        let status = payload
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("active");

        if fact_type.is_empty() || fact_key.is_empty() {
            tracing::warn!(
                fact_type = %fact_type,
                fact_key = %fact_key,
                "profile.fact.created payload missing fact_type or fact_key"
            );
            return StatusCode::BAD_REQUEST;
        }

        {
            let mut g = state.profiles.lock().expect("profiles mutex poisoned");
            let entry = g.entry(user_id.clone()).or_insert_with(ProfileDoc::default);
            entry.user_id = user_id.clone();
            entry.facts.push(StoredFact {
                fact_type: fact_type.to_string(),
                value: fact_key.to_string(),
                source_memory_id: String::new(),
                source_message_id: None,
                confidence: confidence.unwrap_or(0.5),
            });
            refresh_derived_profile_fields(
                &entry.facts,
                &mut entry.city_level_location,
                &mut entry.headline,
                &mut entry.bio,
                &mut entry.memory_highlights,
                &mut entry.traits,
            );
            entry.updated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        }

        if let Some(pg) = &state.pg {
            if let Err(e) = pg.ensure_user_exists(&user_id).await {
                tracing::error!(error = %e, user_id = %user_id, "postgres ensure_user_exists (fact-created) failed — data will not persist");
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
            if let Err(e) = pg
                .upsert_profile(&user_id, None, None, None, None, None, &[], true, true)
                .await
            {
                tracing::error!(error = %e, user_id = %user_id, "postgres upsert_profile (fact-created) failed — data will not persist");
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
            if let Err(e) = pg
                .insert_fact(
                    &user_id,
                    fact_type,
                    fact_key,
                    &fact_value_json,
                    source_type,
                    None,
                    confidence,
                    status,
                    None,
                )
                .await
            {
                tracing::error!(error = %e, user_id = %user_id, "postgres insert_fact (fact-created) failed — data will not persist");
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
        }

        let trait_snap = {
            let g = state.profiles.lock().expect("profiles mutex poisoned");
            g.get(&user_id).map(|e| TraitSnapshot {
                interest_tags: e.traits.interest_tags.clone(),
                connection_goal_tags: e.traits.connection_goal_tags.clone(),
                location_label: e.traits.location_label.clone(),
                communication_preferences: e.traits.communication_preferences.clone(),
            })
        };
        if let (Some(ts), Some(pg)) = (trait_snap, &state.pg) {
            for t in &ts.interest_tags {
                if let Err(e) = pg.upsert_trait(&user_id, "interest", t, None, None).await {
                    tracing::error!(error = %e, user_id = %user_id, "postgres upsert_trait (interest) failed");
                }
            }
            for t in &ts.connection_goal_tags {
                if let Err(e) = pg.upsert_trait(&user_id, "goal", t, None, None).await {
                    tracing::error!(error = %e, user_id = %user_id, "postgres upsert_trait (goal) failed");
                }
            }
            if let Some(ref loc) = ts.location_label {
                if let Err(e) = pg.upsert_trait(&user_id, "location", loc, None, None).await {
                    tracing::error!(error = %e, user_id = %user_id, "postgres upsert_trait (location) failed");
                }
            }
            for t in &ts.communication_preferences {
                if let Err(e) = pg
                    .upsert_trait(&user_id, "communication_preference", t, None, None)
                    .await
                {
                    tracing::error!(error = %e, user_id = %user_id, "postgres upsert_trait (communication_preference) failed");
                }
            }
        }

        tracing::info!(
            user_id = %user_id,
            fact_type = %fact_type,
            "profile fact created from direct event"
        );
        return StatusCode::ACCEPTED;
    }

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

    // Write to in-memory HashMap first (write-through cache for projection logic),
    // then persist to PG. The in-memory HashMap is NOT the primary read source when
    // PG is active — reads go through `read_profile` which prefers PG.
    let pg_snapshot = {
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

        if state.pg.is_some() {
            Some(PgSnapshot {
                user_id: entry.user_id.clone(),
                display_name: if entry.display_name.is_empty() {
                    None
                } else {
                    Some(entry.display_name.clone())
                },
                avatar_url: if entry.avatar_url.is_empty() {
                    None
                } else {
                    Some(entry.avatar_url.clone())
                },
                headline: if entry.headline.is_empty() {
                    None
                } else {
                    Some(entry.headline.clone())
                },
                bio: if entry.bio.is_empty() {
                    None
                } else {
                    Some(entry.bio.clone())
                },
                city_level_location: if entry.city_level_location.is_empty() {
                    None
                } else {
                    Some(entry.city_level_location.clone())
                },
                languages: entry.languages.clone(),
                is_searchable: entry.is_searchable,
                allow_discovery: entry.allow_discovery,
                facts: entry.facts.clone(),
            })
        } else {
            None
        }
    };

    if let (Some(pg), Some(snap)) = (&state.pg, pg_snapshot) {
        if let Err(e) = pg.ensure_user_exists(&snap.user_id).await {
            tracing::error!(error = %e, user_id = %snap.user_id, "postgres ensure_user_exists (projection) failed — data will not persist");
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
        if let Err(e) = pg
            .upsert_profile(
                &snap.user_id,
                snap.display_name.as_deref(),
                snap.avatar_url.as_deref(),
                snap.headline.as_deref(),
                snap.bio.as_deref(),
                snap.city_level_location.as_deref(),
                &snap.languages,
                snap.is_searchable,
                snap.allow_discovery,
            )
            .await
        {
            tracing::error!(error = %e, user_id = %snap.user_id, "postgres upsert_profile (projection) failed — data will not persist");
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
        for f in &snap.facts {
            let fv = serde_json::json!({ "value": f.value, "confidence": f.confidence, "source_memory_id": f.source_memory_id });
            if let Err(e) = pg
                .insert_fact(
                    &snap.user_id,
                    &f.fact_type,
                    &f.value,
                    &fv,
                    "memory_projection",
                    None,
                    Some(f.confidence),
                    "active",
                    None,
                )
                .await
            {
                tracing::error!(error = %e, user_id = %snap.user_id, fact_type = %f.fact_type, "postgres insert_fact (projection) failed");
            }
        }

        let trait_snap = {
            let g = state.profiles.lock().expect("profiles mutex poisoned");
            g.get(&snap.user_id).map(|e| TraitSnapshot {
                interest_tags: e.traits.interest_tags.clone(),
                connection_goal_tags: e.traits.connection_goal_tags.clone(),
                location_label: e.traits.location_label.clone(),
                communication_preferences: e.traits.communication_preferences.clone(),
            })
        };
        if let Some(ts) = trait_snap {
            for t in &ts.interest_tags {
                if let Err(e) = pg
                    .upsert_trait(&snap.user_id, "interest", t, None, None)
                    .await
                {
                    tracing::error!(error = %e, user_id = %snap.user_id, "postgres upsert_trait (interest/projection) failed");
                }
            }
            for t in &ts.connection_goal_tags {
                if let Err(e) = pg.upsert_trait(&snap.user_id, "goal", t, None, None).await {
                    tracing::error!(error = %e, user_id = %snap.user_id, "postgres upsert_trait (goal/projection) failed");
                }
            }
            if let Some(ref loc) = ts.location_label {
                if let Err(e) = pg
                    .upsert_trait(&snap.user_id, "location", loc, None, None)
                    .await
                {
                    tracing::error!(error = %e, user_id = %snap.user_id, "postgres upsert_trait (location/projection) failed");
                }
            }
            for t in &ts.communication_preferences {
                if let Err(e) = pg
                    .upsert_trait(&snap.user_id, "communication_preference", t, None, None)
                    .await
                {
                    tracing::error!(error = %e, user_id = %snap.user_id, "postgres upsert_trait (communication_preference/projection) failed");
                }
            }
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
    pub fn new(config: Config, pg: Option<Arc<PostgresStore>>) -> Arc<Self> {
        Arc::new(Self {
            config,
            client: reqwest::Client::new(),
            profiles: Mutex::new(std::collections::HashMap::new()),
            pg,
        })
    }

    /// Read a profile using the authoritative source:
    /// PG first (if available), then in-memory HashMap (write-through cache),
    /// then default empty ProfileDoc.
    pub async fn read_profile(&self, user_id: &str) -> ProfileDoc {
        if let Some(pg) = &self.pg {
            let pg_profile = pg.find_profile(user_id).await;
            let pg_facts = pg.list_facts(user_id).await;
            let pg_traits = pg.list_traits(user_id).await;

            match (&pg_profile, &pg_facts, &pg_traits) {
                (Ok(Some(row)), Ok(facts), Ok(traits)) => {
                    let stored_facts: Vec<StoredFact> = facts
                        .iter()
                        .map(|f| {
                            let source_memory_id = f
                                .fact_value_json
                                .get("source_memory_id")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let source_message_id = f
                                .fact_value_json
                                .get("source_message_id")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                            StoredFact {
                                fact_type: f.fact_type.clone(),
                                value: f.fact_key.clone(),
                                source_memory_id,
                                source_message_id,
                                confidence: f.confidence.unwrap_or(0.5),
                            }
                        })
                        .collect();

                    let mut ts = TraitSnapshot::default();
                    for t in traits {
                        match t.trait_type.as_str() {
                            "interest" => ts.interest_tags.push(t.trait_key.clone()),
                            "goal" => ts.connection_goal_tags.push(t.trait_key.clone()),
                            "location" if ts.location_label.is_none() => {
                                ts.location_label = Some(t.trait_key.clone());
                            }
                            "communication_preference" => {
                                ts.communication_preferences.push(t.trait_key.clone())
                            }
                            _ => {}
                        }
                    }

                    let mut doc = ProfileDoc {
                        user_id: row.user_id.clone(),
                        display_name: row.display_name.clone().unwrap_or_default(),
                        avatar_url: row.avatar_url.clone().unwrap_or_default(),
                        headline: row.headline.clone().unwrap_or_default(),
                        bio: row.bio.clone().unwrap_or_default(),
                        city_level_location: row.city_level_location.clone().unwrap_or_default(),
                        languages: row.languages.clone().unwrap_or_default(),
                        is_searchable: row.is_searchable,
                        allow_discovery: row.allow_discovery,
                        updated_at: row.updated_at.clone(),
                        memory_highlights: vec![],
                        applied_projection_ids: HashSet::new(),
                        facts: stored_facts,
                        traits: ts,
                    };
                    refresh_derived_profile_fields(
                        &doc.facts,
                        &mut doc.city_level_location,
                        &mut doc.headline,
                        &mut doc.bio,
                        &mut doc.memory_highlights,
                        &mut doc.traits,
                    );
                    return doc;
                }
                (Ok(None), _, _) => {}
                (Err(e), _, _) => {
                    tracing::error!(error = %e, user_id = user_id, "PG find_profile failed, returning empty profile instead of stale in-memory data");
                    return ProfileDoc {
                        user_id: user_id.to_string(),
                        ..Default::default()
                    };
                }
                _ => {}
            }
        }

        let g = self.profiles.lock().expect("profiles mutex poisoned");
        g.get(user_id).cloned().unwrap_or_default()
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
        assert_eq!(
            entry.facts.len(),
            1,
            "same fact key deduped across projections"
        );
    }
}
