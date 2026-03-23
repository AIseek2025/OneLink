//! BFF — chat init aggregates identity + ai-chat (+ empty questions).

use std::sync::Arc;

use axum::{
    extract::State,
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use serde_json::{json, Value};

use crate::config::Config;

pub fn router(state: Arc<BffState>) -> Router {
    Router::new()
        .route("/api/v1/placeholder", get(|| async { "bff skeleton" }))
        .route("/api/v1/bff/chat/init", get(chat_init))
        .with_state(state)
}

#[derive(Debug)]
pub struct BffState {
    pub config: Config,
    pub client: reqwest::Client,
}

impl BffState {
    pub fn new(config: Config) -> Arc<Self> {
        Arc::new(Self {
            config,
            client: reqwest::Client::new(),
        })
    }
}

async fn chat_init(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "missing Authorization".to_string(),
        ))?;

    let me_url = format!(
        "{}/api/v1/identity/me",
        state.config.identity_service_base_url
    );
    let me_resp = state
        .client
        .get(me_url)
        .header(AUTHORIZATION, auth)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("identity: {e}")))?;
    if !me_resp.status().is_success() {
        return Err((StatusCode::UNAUTHORIZED, "invalid token".to_string()));
    }
    let user: Value = me_resp
        .json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("identity json: {e}")))?;

    let conv_url = format!(
        "{}/api/v1/chat/conversations",
        state.config.ai_chat_service_base_url
    );
    let conv_resp = state
        .client
        .post(conv_url)
        .header(AUTHORIZATION, auth)
        .header("Content-Type", "application/json")
        .json(&json!({ "idempotency_key": null }))
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("ai-chat: {e}")))?;
    if !conv_resp.status().is_success() {
        return Err((
            StatusCode::BAD_GATEWAY,
            format!("ai-chat conversations status {}", conv_resp.status()),
        ));
    }
    let conversation: Value = conv_resp
        .json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("ai-chat json: {e}")))?;

    Ok(Json(json!({
        "user": user,
        "conversation": conversation,
        "pending_questions": []
    })))
}
