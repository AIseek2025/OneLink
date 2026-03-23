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

const INTERNAL_TOKEN_HEADER: &str = "x-internal-token";

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
    /// 由记忆投影写入的可见摘要行（dev MVP）
    memory_highlights: Vec<String>,
    applied_projection_ids: HashSet<String>,
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
    });

    Ok(Json(MeResponse {
        user_id: base.user_id,
        display_name: if base.display_name.is_empty() {
            "OneLink 用户".to_string()
        } else {
            base.display_name
        },
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
        "headline".to_string(),
        "bio".to_string(),
        "chat_memory".to_string(),
    ];
    let mut filled = vec![];
    let mut missing = vec![];

    let d = doc.unwrap_or_default();
    if !d.display_name.is_empty() {
        filled.push("display_name".to_string());
    } else {
        missing.push("display_name".to_string());
    }
    if !d.headline.is_empty() {
        filled.push("headline".to_string());
    } else {
        missing.push("headline".to_string());
    }
    if !d.bio.is_empty() {
        filled.push("bio".to_string());
    } else {
        missing.push("bio".to_string());
    }
    if !d.memory_highlights.is_empty() {
        filled.push("chat_memory".to_string());
    } else {
        missing.push("chat_memory".to_string());
    }

    let rate = (filled.len() as f64 / required.len() as f64).clamp(0.0, 1.0);

    Ok(Json(CompletionResponse {
        completion_rate: (rate * 100.0).round() / 100.0,
        required_dimensions: required.clone(),
        filled_dimensions: filled.clone(),
        missing_dimensions: missing,
    }))
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

    let mut lines: Vec<String> = resolved
        .items
        .iter()
        .map(|i| format!("[{}] {}", i.network_type, i.content))
        .collect();

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
        });

        if entry.applied_projection_ids.contains(&projection_id) {
            tracing::info!(projection_id = %projection_id, "duplicate projection skipped");
            return StatusCode::ACCEPTED;
        }
        entry.applied_projection_ids.insert(projection_id);

        for line in lines.drain(..) {
            entry.memory_highlights.push(line.clone());
        }

        let joined = entry.memory_highlights.join("；");
        entry.headline = format!("记忆已同步 · {} 条要点", entry.memory_highlights.len());
        entry.bio = format!("来自聊天的记忆投影：{}", joined);
        entry.updated_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
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
