//! BFF — aggregation layer for App/Web.
//!
//! Implemented endpoints:
//! - `POST /api/v1/bff/auth/register` — register proxy (identity-service)
//! - `POST /api/v1/bff/auth/login` — login proxy (identity-service)
//! - `GET /api/v1/bff/chat/init` — chat page init (identity + conversation + pending questions)
//! - `POST /api/v1/bff/chat/messages` — chat message send proxy (ai-chat-service)
//! - `GET /api/v1/bff/onboarding` — cold-start onboarding (identity + pending + questionnaire progress)
//! - `GET /api/v1/bff/home` — home page (identity + profile + completion)
//! - `GET /api/v1/bff/profile/{userId}` — profile view (identity + profile)
//! - `PATCH /api/v1/bff/profile/me` — profile edit proxy (passes through to profile-service)
//! - `POST /api/v1/bff/find/intent` — find-person intent (identity + match-service find-request)
//! - `GET /api/v1/bff/find/results` — find-person results (identity + match-service candidates)
//! - `POST /api/v1/bff/analytics/events` — analytics event ingestion (fire-and-forget)
//! - `POST /api/v1/bff/questions/answers` — question answer proxy (question-service)
//! - `GET /api/v1/bff/dm/list` — DM thread list (identity + dm-service threads)
//! - `POST /api/v1/bff/dm/send` — DM send message (identity + safety review + dm-service)
//! - `POST /api/v1/bff/safety/report` — report user (identity + safety-service)
//! - `POST /api/v1/bff/safety/block` — block user (identity + safety-service)

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    routing::{get, patch, post},
    Json, Router,
};

use onelink_internal_auth::INTERNAL_TOKEN_HEADER;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::Config;
use crate::registry::LocaleRegistry;

pub fn router(state: Arc<BffState>) -> Router {
    Router::new()
        .route("/api/v1/bff/auth/register", post(auth_register))
        .route("/api/v1/bff/auth/login", post(auth_login))
        .route("/api/v1/bff/auth/session/refresh", post(session_refresh))
        .route("/api/v1/bff/chat/init", get(chat_init))
        .route("/api/v1/bff/chat/messages", post(chat_messages))
        .route("/api/v1/bff/onboarding", get(onboarding))
        .route("/api/v1/bff/home", get(home))
        .route("/api/v1/bff/profile/{userId}", get(profile))
        .route("/api/v1/bff/profile/me", patch(profile_me_patch))
        .route("/api/v1/bff/find/intent", post(find_intent))
        .route("/api/v1/bff/find/results", get(find_results))
        .route("/api/v1/bff/find/requests", post(find_requests))
        .route(
            "/api/v1/bff/find/requests/{requestId}",
            get(find_request_detail),
        )
        .route(
            "/api/v1/bff/find/requests/{requestId}/clarifications",
            post(clarification_answer),
        )
        .route("/api/v1/bff/recommendations", get(recommendation_list))
        .route(
            "/api/v1/bff/recommendations/{recId}",
            get(recommendation_detail),
        )
        .route(
            "/api/v1/bff/recommendations/{recId}/feedback",
            post(recommendation_feedback),
        )
        .route("/api/v1/bff/analytics/events", post(analytics_events))
        .route("/api/v1/bff/questions/answers", post(question_answer))
        .route("/api/v1/bff/dm/list", get(dm_list))
        .route("/api/v1/bff/dm/send", post(dm_send))
        .route("/api/v1/bff/dm/threads/draft", post(dm_draft))
        .route(
            "/api/v1/bff/dm/threads/first-message",
            post(dm_first_message),
        )
        .route("/api/v1/bff/dm/threads/{threadId}", get(dm_thread_detail))
        .route("/api/v1/bff/safety/report", post(safety_report))
        .route("/api/v1/bff/safety/block", post(safety_block))
        .route("/api/v1/bff/safety/reports", post(safety_reports))
        .route("/api/v1/bff/safety/blocks", post(safety_blocks))
        .route(
            "/api/v1/bff/safety/appeals/{appealId}",
            get(safety_appeal_status),
        )
        .route("/api/v1/bff/settings/locale", get(locale_registry))
        .route("/api/v1/bff/compliance/summary", get(compliance_summary))
        .route("/api/v1/bff/compliance/export", post(compliance_export))
        .route("/api/v1/bff/compliance/delete", post(compliance_delete))
        .route(
            "/api/v1/bff/compliance/correction",
            post(compliance_correction),
        )
        .route("/api/v1/bff/admin/reports", get(admin_reports))
        .route(
            "/api/v1/bff/admin/reports/{reportId}",
            get(admin_report_detail),
        )
        .route(
            "/api/v1/bff/admin/reports/{reportId}/action",
            post(admin_report_action),
        )
        .route("/api/v1/bff/admin/appeals", get(admin_appeals))
        .route("/api/v1/bff/admin/metrics", get(admin_metrics))
        .route("/api/v1/bff/users/me", get(users_me))
        .route("/api/v1/bff/users/me", patch(users_me_patch))
        .route("/api/v1/bff/conversations", get(dm_conversations))
        .route("/api/v1/bff/compliance/data", get(compliance_data))
        .route("/api/v1/bff/region/gate", get(region_gate))
        .route("/api/v1/bff/i18n/registry", get(i18n_registry))
        .route("/api/v1/bff/i18n/translate", get(i18n_translate))
        .with_state(state)
}

#[derive(Debug)]
pub struct BffState {
    pub config: Config,
    pub client: reqwest::Client,
    pub locale_registry: LocaleRegistry,
}

impl BffState {
    pub fn new(config: Config) -> Arc<Self> {
        let locale_registry = LocaleRegistry::from_config(&config);
        Arc::new(Self {
            config,
            client: reqwest::Client::new(),
            locale_registry,
        })
    }
}

/// Auth register proxy: POST /api/v1/bff/auth/register → identity-service POST /api/v1/identity/register.
async fn auth_register(
    State(state): State<Arc<BffState>>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let url = format!(
        "{}/api/v1/identity/register",
        state.config.identity_service_base_url
    );
    let resp = state
        .client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("identity register: {e}")))?;

    let status = resp.status();
    if !status.is_success() {
        let body_text = resp.text().await.unwrap_or_default();
        return Err((status, format!("identity register failed: {body_text}")));
    }

    let result: Value = resp.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("identity register json: {e}"),
        )
    })?;

    tracing::info!(
        user_id = %result.get("user_id").and_then(|v| v.as_str()).unwrap_or(""),
        "bff: auth register proxied"
    );

    Ok(Json(result))
}

/// Auth login proxy: POST /api/v1/bff/auth/login → identity-service POST /api/v1/identity/login.
async fn auth_login(
    State(state): State<Arc<BffState>>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let url = format!(
        "{}/api/v1/identity/login",
        state.config.identity_service_base_url
    );
    let resp = state
        .client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("identity login: {e}")))?;

    let status = resp.status();
    if !status.is_success() {
        let body_text = resp.text().await.unwrap_or_default();
        return Err((status, format!("identity login failed: {body_text}")));
    }

    let result: Value = resp
        .json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("identity login json: {e}")))?;

    tracing::info!(
        user_id = %result.get("user_id").and_then(|v| v.as_str()).unwrap_or(""),
        "bff: auth login proxied"
    );

    Ok(Json(result))
}

async fn identity_user_json(
    state: &Arc<BffState>,
    auth: &str,
) -> Result<Value, (StatusCode, String)> {
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
    me_resp
        .json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("identity json: {e}")))
}

async fn profile_me_json(state: &Arc<BffState>, auth: &str) -> Option<Value> {
    let url = format!(
        "{}/api/v1/profile/me",
        state.config.profile_service_base_url
    );
    match state
        .client
        .get(&url)
        .header(AUTHORIZATION, auth)
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => r.json::<Value>().await.ok(),
        Ok(r) => {
            tracing::warn!(
                status = %r.status(),
                "bff: profile.me non-success, degrading to null"
            );
            None
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "bff: profile.me request failed, degrading to null"
            );
            None
        }
    }
}

async fn profile_completion_json(state: &Arc<BffState>, auth: &str) -> Option<Value> {
    let url = format!(
        "{}/api/v1/profile/me/completion",
        state.config.profile_service_base_url
    );
    match state
        .client
        .get(&url)
        .header(AUTHORIZATION, auth)
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => r.json::<Value>().await.ok(),
        Ok(r) => {
            tracing::warn!(
                status = %r.status(),
                "bff: profile.completion non-success, degrading to null"
            );
            None
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "bff: profile.completion request failed, degrading to null"
            );
            None
        }
    }
}

/// Pending question `items[]` only; degrades to `[]` on failure (same semantics as `chat/init`).
async fn question_pending_items_json(state: &Arc<BffState>, auth: &str) -> Value {
    let q_url = format!(
        "{}/api/v1/questions/pending?channel=ai_chat&limit=10",
        state.config.question_service_base_url
    );
    match state
        .client
        .get(&q_url)
        .header(AUTHORIZATION, auth)
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => match r.json::<Value>().await {
            Ok(v) => v.get("items").cloned().unwrap_or_else(|| json!([])),
            Err(_) => json!([]),
        },
        Ok(r) => {
            tracing::warn!(
                status = %r.status(),
                "bff: question pending non-success, degrading to empty items"
            );
            json!([])
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "bff: question pending request failed, degrading to empty items"
            );
            json!([])
        }
    }
}

/// Questionnaire-domain progress from `question-service` `GET /questions/completion` (not profile).
/// On failure returns a minimal object with `degraded: true` — BFF does not compute progress.
async fn questionnaire_progress_json(state: &Arc<BffState>, auth: &str) -> Value {
    let url = format!(
        "{}/api/v1/questions/completion",
        state.config.question_service_base_url
    );
    match state
        .client
        .get(&url)
        .header(AUTHORIZATION, auth)
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => match r.json::<Value>().await {
            Ok(v) => v,
            Err(_) => json!({
                "degraded": true,
                "reason": "invalid_json"
            }),
        },
        Ok(r) => {
            tracing::warn!(
                status = %r.status(),
                "bff: question completion non-success, returning degraded progress"
            );
            json!({
                "degraded": true,
                "reason": "non_success",
                "http_status": r.status().as_u16()
            })
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "bff: question completion request failed, returning degraded progress"
            );
            json!({
                "degraded": true,
                "reason": "request_failed"
            })
        }
    }
}

fn extract_auth(headers: &HeaderMap) -> Result<&str, (StatusCode, String)> {
    headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "missing Authorization".to_string(),
        ))
}

async fn chat_init(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let user = identity_user_json(&state, auth).await?;

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

    let pending_questions = question_pending_items_json(&state, auth).await;

    Ok(Json(json!({
        "user": user,
        "conversation": conversation,
        "pending_questions": pending_questions
    })))
}

/// Chat message send proxy: POST /api/v1/bff/chat/messages → ai-chat-service POST /api/v1/chat/conversations/{id}/messages.
/// Requires Bearer auth. Validates identity first, then forwards the message.
#[derive(Debug, Deserialize)]
struct ChatMessageRequest {
    conversation_id: String,
    content_type: Option<String>,
    content_text: String,
}

async fn chat_messages(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
    Json(body): Json<ChatMessageRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let _user = identity_user_json(&state, auth).await?;

    if body.content_text.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "content_text must not be empty".to_string(),
        ));
    }

    let url = format!(
        "{}/api/v1/chat/conversations/{}/messages",
        state.config.ai_chat_service_base_url, body.conversation_id
    );
    let resp = state
        .client
        .post(&url)
        .header(AUTHORIZATION, auth)
        .header("Content-Type", "application/json")
        .json(&json!({
            "content_type": body.content_type.as_deref().unwrap_or("text"),
            "content_text": body.content_text,
        }))
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("ai-chat message: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err((status, format!("ai-chat message failed: {body_text}")));
    }

    let result: Value = resp.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("ai-chat message json: {e}"),
        )
    })?;

    Ok(Json(result))
}

/// Cold-start style aggregate: identity + questionnaire pending + questionnaire progress only.
async fn onboarding(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;

    let user = identity_user_json(&state, auth).await?;
    let pending_questions = question_pending_items_json(&state, auth).await;
    let progress = questionnaire_progress_json(&state, auth).await;

    Ok(Json(json!({
        "user": user,
        "pending_questions": pending_questions,
        "progress": progress
    })))
}

/// Home page aggregate: identity.me + profile.me + profile.completion.
/// profile-service degradation: profile → null, completion → null.
async fn home(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let user = identity_user_json(&state, auth).await?;
    let profile = profile_me_json(&state, auth).await;
    let completion = profile_completion_json(&state, auth).await;

    Ok(Json(json!({
        "user": user,
        "profile": profile,
        "completion": completion
    })))
}

/// Profile view aggregate: identity.me + profile.me.
/// Only supports viewing own profile (userId must match token user).
async fn profile(
    State(state): State<Arc<BffState>>,
    Path(user_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let user = identity_user_json(&state, auth).await?;

    let token_user_id = user.get("user_id").and_then(|v| v.as_str()).unwrap_or("");
    if token_user_id != user_id {
        return Err((
            StatusCode::FORBIDDEN,
            "cannot view other user's profile".to_string(),
        ));
    }

    let profile = profile_me_json(&state, auth).await;
    let completion = profile_completion_json(&state, auth).await;

    Ok(Json(json!({
        "user": user,
        "profile": profile,
        "completion": completion
    })))
}

/// Profile edit proxy: PATCH /api/v1/bff/profile/me → profile-service PATCH /api/v1/profile/me.
/// Validates identity first, then forwards the request body.
async fn profile_me_patch(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let user = identity_user_json(&state, auth).await?;

    let url = format!(
        "{}/api/v1/profile/me",
        state.config.profile_service_base_url
    );
    let resp = state
        .client
        .patch(&url)
        .header(AUTHORIZATION, auth)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("profile patch: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err((status, format!("profile-service patch failed: {body_text}")));
    }

    let result: Value = resp
        .json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("profile patch json: {e}")))?;

    tracing::info!(
        user_id = %user.get("user_id").and_then(|v| v.as_str()).unwrap_or(""),
        "bff: profile.me patched"
    );

    Ok(Json(result))
}

/// Analytics event ingestion: accepts a batch or single event from the web client,
/// logs it server-side, and returns 202.
///
/// **Security**: Requires Bearer auth. The server injects `user_id` from the
/// identity-service token validation, overriding any client-supplied `user_id`.
/// This prevents user_id spoofing in analytics data.
async fn analytics_events(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let user = identity_user_json(&state, auth).await?;
    let server_user_id = user.get("user_id").and_then(|v| v.as_str()).unwrap_or("");

    let enriched = if let Some(arr) = body.as_array() {
        let items: Vec<Value> = arr
            .iter()
            .map(|ev| {
                let mut obj = ev.clone();
                if let Some(m) = obj.as_object_mut() {
                    m.insert("user_id".to_string(), json!(server_user_id));
                }
                obj
            })
            .collect();
        json!(items)
    } else {
        let mut obj = body.clone();
        if let Some(m) = obj.as_object_mut() {
            m.insert("user_id".to_string(), json!(server_user_id));
        }
        obj
    };

    let event_count = if enriched.is_array() {
        enriched.as_array().unwrap().len()
    } else {
        1
    };

    tracing::info!(
        user_id = %server_user_id,
        event_count = event_count,
        "bff: analytics events received (server-injected user_id)"
    );

    let _ = &enriched;

    Ok(Json(json!({
        "accepted": event_count,
        "status": "logged"
    })))
}

/// Question answer proxy: POST /api/v1/bff/questions/answers → question-service POST /api/v1/questions/answers.
/// Requires Bearer auth. Validates identity first, then forwards the answer.
async fn question_answer(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let user = identity_user_json(&state, auth).await?;

    let url = format!(
        "{}/api/v1/questions/answers",
        state.config.question_service_base_url
    );
    let resp = state
        .client
        .post(&url)
        .header(AUTHORIZATION, auth)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("question-service answer: {e}"),
            )
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err((
            status,
            format!("question-service answer failed: {body_text}"),
        ));
    }

    let result: Value = resp.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("question-service answer json: {e}"),
        )
    })?;

    tracing::info!(
        user_id = %user.get("user_id").and_then(|v| v.as_str()).unwrap_or(""),
        "bff: question answer proxied"
    );

    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
struct FindIntentRequest {
    raw_query: String,
    #[serde(default)]
    intent_tags: Vec<String>,
}

#[derive(Debug, Serialize)]
struct FindIntentResponse {
    find_request_id: String,
    status: String,
}

async fn find_intent(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
    Json(body): Json<FindIntentRequest>,
) -> Result<Json<FindIntentResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let user = identity_user_json(&state, auth).await?;

    let user_id = user.get("user_id").and_then(|v| v.as_str()).unwrap_or("");

    if body.raw_query.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "raw_query must not be empty".to_string(),
        ));
    }

    let url = format!(
        "{}/api/v1/match/find-requests",
        state.config.match_service_base_url
    );
    let resp = state
        .client
        .post(&url)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .header("Content-Type", "application/json")
        .json(&json!({
            "user_id": user_id,
            "raw_query": body.raw_query,
            "intent_tags": body.intent_tags,
        }))
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("match-service: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err((
            status,
            format!("match-service find-request failed: {body_text}"),
        ));
    }

    let match_resp: Value = resp
        .json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("match-service json: {e}")))?;

    let find_request_id = match_resp
        .get("find_request_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let status = match_resp
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("pending")
        .to_string();

    tracing::info!(
        find_request_id = %find_request_id,
        user_id = %user_id,
        raw_query = %body.raw_query,
        "bff: find intent submitted to match-service"
    );

    Ok(Json(FindIntentResponse {
        find_request_id,
        status,
    }))
}

/// Find-person results: proxies to match-service candidates endpoint.
async fn find_results(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let user = identity_user_json(&state, auth).await?;

    let user_id = user.get("user_id").and_then(|v| v.as_str()).unwrap_or("");

    let url = format!(
        "{}/api/v1/match/find-requests?user_id={}",
        state.config.match_service_base_url, user_id,
    );

    let resp = state
        .client
        .get(&url)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("match-service: {e}")))?;

    if !resp.status().is_success() {
        tracing::warn!(status = %resp.status(), "bff: match-service find-results non-success");
        return Ok(Json(json!({
            "find_requests": [],
            "candidates": [],
            "degraded": true,
            "reason": "match_service_unavailable"
        })));
    }

    let result: Value = resp
        .json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("match-service json: {e}")))?;

    Ok(Json(result))
}

/// DM list: proxies to dm-service threads endpoint.
async fn dm_list(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let user = identity_user_json(&state, auth).await?;

    let user_id = user.get("user_id").and_then(|v| v.as_str()).unwrap_or("");

    let url = format!(
        "{}/api/v1/dm/threads?user_id={}",
        state.config.dm_service_base_url, user_id,
    );

    let resp = state
        .client
        .get(&url)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("dm-service: {e}")))?;

    if !resp.status().is_success() {
        tracing::warn!(status = %resp.status(), "bff: dm-service threads non-success");
        return Ok(Json(json!({
            "threads": [],
            "degraded": true,
            "reason": "dm_service_unavailable"
        })));
    }

    let result: Value = resp
        .json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("dm-service json: {e}")))?;

    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
struct DmSendRequest {
    recipient_user_id: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct DmSendResponse {
    thread_id: String,
    message_id: String,
    safety_review_passed: bool,
}

/// DM send: creates thread if needed, reviews first message via safety-service, then sends.
async fn dm_send(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
    Json(body): Json<DmSendRequest>,
) -> Result<Json<DmSendResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let user = identity_user_json(&state, auth).await?;

    let user_id = user
        .get("user_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if body.content.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "content must not be empty".to_string(),
        ));
    }

    if user_id == body.recipient_user_id {
        return Err((
            StatusCode::BAD_REQUEST,
            "cannot send DM to yourself".to_string(),
        ));
    }

    let thread_url = format!("{}/api/v1/dm/threads", state.config.dm_service_base_url);
    let thread_resp = state
        .client
        .post(&thread_url)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .header("Content-Type", "application/json")
        .json(&json!({
            "initiator_user_id": user_id,
            "recipient_user_id": body.recipient_user_id,
        }))
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("dm-service thread: {e}")))?;

    if !thread_resp.status().is_success() {
        let status = thread_resp.status();
        let body_text = thread_resp.text().await.unwrap_or_default();
        return Err((
            status,
            format!("dm-service thread create failed: {body_text}"),
        ));
    }

    let thread_result: Value = thread_resp.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("dm-service thread json: {e}"),
        )
    })?;

    let thread_id = thread_result
        .get("thread_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let is_new_thread = thread_result
        .get("created")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if is_new_thread {
        let review_url = format!(
            "{}/api/v1/safety/dm-first-message-review",
            state.config.safety_service_base_url
        );
        let review_resp = state
            .client
            .post(&review_url)
            .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
            .header("Content-Type", "application/json")
            .json(&json!({
                "sender_user_id": user_id,
                "recipient_user_id": body.recipient_user_id,
                "message_content": body.content,
            }))
            .send()
            .await
            .map_err(|e| (StatusCode::BAD_GATEWAY, format!("safety-service: {e}")))?;

        if review_resp.status().is_success() {
            let review_result: Value = review_resp
                .json()
                .await
                .map_err(|e| (StatusCode::BAD_GATEWAY, format!("safety-service json: {e}")))?;

            let allowed = review_result
                .get("allowed")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            if !allowed {
                let reason = review_result
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                return Err((
                    StatusCode::FORBIDDEN,
                    format!("message blocked by safety review: {reason}"),
                ));
            }
        }
    }

    let msg_url = format!(
        "{}/api/v1/dm/threads/{}/messages",
        state.config.dm_service_base_url, thread_id,
    );
    let msg_resp = state
        .client
        .post(&msg_url)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .header("Content-Type", "application/json")
        .json(&json!({
            "sender_user_id": user_id,
            "content": body.content,
        }))
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("dm-service send: {e}")))?;

    if !msg_resp.status().is_success() {
        let status = msg_resp.status();
        let body_text = msg_resp.text().await.unwrap_or_default();
        return Err((status, format!("dm-service send failed: {body_text}")));
    }

    let msg_result: Value = msg_resp.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("dm-service send json: {e}"),
        )
    })?;

    let message_id = msg_result
        .get("message_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    tracing::info!(
        thread_id = %thread_id,
        message_id = %message_id,
        sender = %user_id,
        "bff: DM sent"
    );

    Ok(Json(DmSendResponse {
        thread_id,
        message_id,
        safety_review_passed: true,
    }))
}

#[derive(Debug, Deserialize)]
struct SafetyReportRequest {
    reported_user_id: String,
    reason: String,
    #[serde(default)]
    description: String,
}

#[derive(Debug, Serialize)]
struct SafetyReportResponse {
    report_ticket_id: String,
    status: String,
}

/// Report user: proxies to safety-service.
async fn safety_report(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
    Json(body): Json<SafetyReportRequest>,
) -> Result<Json<SafetyReportResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let user = identity_user_json(&state, auth).await?;

    let user_id = user.get("user_id").and_then(|v| v.as_str()).unwrap_or("");

    let url = format!(
        "{}/api/v1/safety/reports",
        state.config.safety_service_base_url
    );
    let resp = state
        .client
        .post(&url)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .header("Content-Type", "application/json")
        .json(&json!({
            "reporter_user_id": user_id,
            "reported_user_id": body.reported_user_id,
            "reason": body.reason,
            "description": body.description,
        }))
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("safety-service: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err((status, format!("safety-service report failed: {body_text}")));
    }

    let result: Value = resp
        .json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("safety-service json: {e}")))?;

    tracing::info!(
        reporter = %user_id,
        reported = %body.reported_user_id,
        "bff: safety report submitted"
    );

    Ok(Json(SafetyReportResponse {
        report_ticket_id: result
            .get("report_ticket_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        status: result
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("submitted")
            .to_string(),
    }))
}

#[derive(Debug, Deserialize)]
struct SafetyBlockRequest {
    blocked_user_id: String,
}

#[derive(Debug, Serialize)]
struct SafetyBlockResponse {
    blocked_user_id: String,
    created_at: String,
}

/// Block user: proxies to safety-service.
async fn safety_block(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
    Json(body): Json<SafetyBlockRequest>,
) -> Result<Json<SafetyBlockResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let user = identity_user_json(&state, auth).await?;

    let user_id = user.get("user_id").and_then(|v| v.as_str()).unwrap_or("");

    let url = format!(
        "{}/api/v1/safety/blocks",
        state.config.safety_service_base_url
    );
    let resp = state
        .client
        .post(&url)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .header("Content-Type", "application/json")
        .json(&json!({
            "user_id": user_id,
            "blocked_user_id": body.blocked_user_id,
        }))
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("safety-service: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err((status, format!("safety-service block failed: {body_text}")));
    }

    let result: Value = resp
        .json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("safety-service json: {e}")))?;

    tracing::info!(
        user_id = %user_id,
        blocked_user_id = %body.blocked_user_id,
        "bff: user blocked"
    );

    Ok(Json(SafetyBlockResponse {
        blocked_user_id: result
            .get("blocked_user_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        created_at: result
            .get("created_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    }))
}

async fn session_refresh(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let _user = identity_user_json(&state, auth).await?;

    let url = format!(
        "{}/api/v1/identity/session/refresh",
        state.config.identity_service_base_url
    );
    let resp = state
        .client
        .post(&url)
        .header(AUTHORIZATION, auth)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("identity session refresh: {e}"),
            )
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err((
            status,
            format!("identity session refresh failed: {body_text}"),
        ));
    }

    let result: Value = resp.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("identity session refresh json: {e}"),
        )
    })?;

    tracing::info!("bff: session refresh proxied");
    Ok(Json(result))
}

async fn find_requests(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let user = identity_user_json(&state, auth).await?;
    let user_id = user.get("user_id").and_then(|v| v.as_str()).unwrap_or("");

    let url = format!(
        "{}/api/v1/match/find-requests",
        state.config.match_service_base_url
    );
    let resp = state
        .client
        .post(&url)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .header("Content-Type", "application/json")
        .json(&json!({
            "user_id": user_id,
            "payload": body,
        }))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("match-service find-requests: {e}"),
            )
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err((
            status,
            format!("match-service find-requests failed: {body_text}"),
        ));
    }

    let result: Value = resp.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("match-service find-requests json: {e}"),
        )
    })?;

    tracing::info!(user_id = %user_id, "bff: find requests proxied");
    Ok(Json(result))
}

async fn find_request_detail(
    State(state): State<Arc<BffState>>,
    Path(request_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let _user = identity_user_json(&state, auth).await?;

    let url = format!(
        "{}/api/v1/match/find-requests/{}",
        state.config.match_service_base_url, request_id
    );
    let resp = state
        .client
        .get(&url)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("match-service find-request detail: {e}"),
            )
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err((
            status,
            format!("match-service find-request detail failed: {body_text}"),
        ));
    }

    let result: Value = resp.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("match-service find-request detail json: {e}"),
        )
    })?;

    Ok(Json(result))
}

async fn clarification_answer(
    State(state): State<Arc<BffState>>,
    Path(request_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let _user = identity_user_json(&state, auth).await?;

    let url = format!(
        "{}/api/v1/match/find-requests/{}/clarifications",
        state.config.match_service_base_url, request_id
    );
    let resp = state
        .client
        .post(&url)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("match-service clarification: {e}"),
            )
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err((
            status,
            format!("match-service clarification failed: {body_text}"),
        ));
    }

    let result: Value = resp.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("match-service clarification json: {e}"),
        )
    })?;

    tracing::info!(request_id = %request_id, "bff: clarification answer submitted");
    Ok(Json(result))
}

async fn recommendation_list(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let user = identity_user_json(&state, auth).await?;
    let user_id = user.get("user_id").and_then(|v| v.as_str()).unwrap_or("");

    let url = format!(
        "{}/api/v1/match/recommendations?user_id={}",
        state.config.match_service_base_url, user_id
    );
    let resp = state
        .client
        .get(&url)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("match-service recommendations: {e}"),
            )
        })?;

    if !resp.status().is_success() {
        tracing::warn!(status = %resp.status(), "bff: match-service recommendations non-success");
        return Ok(Json(json!({
            "request_id": "",
            "state": "failed",
            "recommendations": [],
            "degraded": true,
            "reason": "match_service_unavailable"
        })));
    }

    let result: Value = resp.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("match-service recommendations json: {e}"),
        )
    })?;

    Ok(Json(result))
}

async fn recommendation_detail(
    State(state): State<Arc<BffState>>,
    Path(rec_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let _user = identity_user_json(&state, auth).await?;

    let url = format!(
        "{}/api/v1/match/recommendations/{}",
        state.config.match_service_base_url, rec_id
    );
    let resp = state
        .client
        .get(&url)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("match-service recommendation detail: {e}"),
            )
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err((
            status,
            format!("match-service recommendation detail failed: {body_text}"),
        ));
    }

    let result: Value = resp.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("match-service recommendation detail json: {e}"),
        )
    })?;

    Ok(Json(result))
}

async fn recommendation_feedback(
    State(state): State<Arc<BffState>>,
    Path(rec_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let _user = identity_user_json(&state, auth).await?;

    let url = format!(
        "{}/api/v1/match/recommendations/{}/feedback",
        state.config.match_service_base_url, rec_id
    );
    let resp = state
        .client
        .post(&url)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("match-service recommendation feedback: {e}"),
            )
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err((
            status,
            format!("match-service recommendation feedback failed: {body_text}"),
        ));
    }

    let result: Value = resp.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("match-service recommendation feedback json: {e}"),
        )
    })?;

    tracing::info!(rec_id = %rec_id, "bff: recommendation feedback submitted");
    Ok(Json(result))
}

async fn dm_draft(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let user = identity_user_json(&state, auth).await?;
    let user_id = user.get("user_id").and_then(|v| v.as_str()).unwrap_or("");

    let initial_message = body
        .get("initial_message")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if initial_message.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "initial_message must not be empty".to_string(),
        ));
    }

    let url = format!(
        "{}/api/v1/dm/threads/draft",
        state.config.dm_service_base_url
    );
    let resp = state
        .client
        .post(&url)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .header("Content-Type", "application/json")
        .json(&json!({
            "user_id": user_id,
            "recommendation_id": body.get("recommendation_id").and_then(|v| v.as_str()).unwrap_or(""),
            "initial_message": initial_message,
        }))
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("dm-service draft: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err((status, format!("dm-service draft failed: {body_text}")));
    }

    let result: Value = resp.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("dm-service draft json: {e}"),
        )
    })?;

    tracing::info!(user_id = %user_id, "bff: dm draft created");
    Ok(Json(result))
}

async fn dm_first_message(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let user = identity_user_json(&state, auth).await?;
    let user_id = user.get("user_id").and_then(|v| v.as_str()).unwrap_or("");

    let message = body.get("message").and_then(|v| v.as_str()).unwrap_or("");

    if message.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "message must not be empty".to_string(),
        ));
    }

    let thread_id = body.get("thread_id").and_then(|v| v.as_str()).unwrap_or("");

    let review_url = format!(
        "{}/api/v1/safety/dm-first-message-review",
        state.config.safety_service_base_url
    );
    let review_resp = state
        .client
        .post(&review_url)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .header("Content-Type", "application/json")
        .json(&json!({
            "sender_user_id": user_id,
            "message_content": message,
        }))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("safety-service review: {e}"),
            )
        })?;

    if review_resp.status().is_success() {
        let review_result: Value = review_resp.json().await.map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("safety-service review json: {e}"),
            )
        })?;

        let allowed = review_result
            .get("allowed")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        if !allowed {
            let reason = review_result
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            return Err((
                StatusCode::FORBIDDEN,
                format!("message blocked by safety review: {reason}"),
            ));
        }
    }

    let msg_url = format!(
        "{}/api/v1/dm/threads/{}/messages",
        state.config.dm_service_base_url, thread_id
    );
    let msg_resp = state
        .client
        .post(&msg_url)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .header("Content-Type", "application/json")
        .json(&json!({
            "sender_user_id": user_id,
            "content": message,
        }))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("dm-service first message: {e}"),
            )
        })?;

    if !msg_resp.status().is_success() {
        let status = msg_resp.status();
        let body_text = msg_resp.text().await.unwrap_or_default();
        return Err((
            status,
            format!("dm-service first message failed: {body_text}"),
        ));
    }

    let result: Value = msg_resp.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("dm-service first message json: {e}"),
        )
    })?;

    tracing::info!(thread_id = %thread_id, user_id = %user_id, "bff: dm first message submitted");
    Ok(Json(result))
}

async fn dm_thread_detail(
    State(state): State<Arc<BffState>>,
    Path(thread_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let _user = identity_user_json(&state, auth).await?;

    let url = format!(
        "{}/api/v1/dm/threads/{}",
        state.config.dm_service_base_url, thread_id
    );
    let resp = state
        .client
        .get(&url)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("dm-service thread detail: {e}"),
            )
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err((
            status,
            format!("dm-service thread detail failed: {body_text}"),
        ));
    }

    let result: Value = resp.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("dm-service thread detail json: {e}"),
        )
    })?;

    Ok(Json(result))
}

async fn safety_reports(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let user = identity_user_json(&state, auth).await?;
    let user_id = user.get("user_id").and_then(|v| v.as_str()).unwrap_or("");

    let url = format!(
        "{}/api/v1/safety/reports",
        state.config.safety_service_base_url
    );
    let resp = state
        .client
        .post(&url)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .header("Content-Type", "application/json")
        .json(&json!({
            "reporter_user_id": user_id,
            "target_user_id": body.get("target_user_id").and_then(|v| v.as_str()).unwrap_or(""),
            "category": body.get("category").and_then(|v| v.as_str()).unwrap_or("other"),
            "description": body.get("description").and_then(|v| v.as_str()).unwrap_or(""),
        }))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("safety-service reports: {e}"),
            )
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err((
            status,
            format!("safety-service reports failed: {body_text}"),
        ));
    }

    let result: Value = resp.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("safety-service reports json: {e}"),
        )
    })?;

    tracing::info!(user_id = %user_id, "bff: safety report submitted");
    Ok(Json(result))
}

async fn safety_blocks(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let user = identity_user_json(&state, auth).await?;
    let user_id = user.get("user_id").and_then(|v| v.as_str()).unwrap_or("");

    let url = format!(
        "{}/api/v1/safety/blocks",
        state.config.safety_service_base_url
    );
    let resp = state
        .client
        .post(&url)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .header("Content-Type", "application/json")
        .json(&json!({
            "user_id": user_id,
            "target_user_id": body.get("target_user_id").and_then(|v| v.as_str()).unwrap_or(""),
            "reason": body.get("reason").and_then(|v| v.as_str()).unwrap_or(""),
        }))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("safety-service blocks: {e}"),
            )
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err((status, format!("safety-service blocks failed: {body_text}")));
    }

    let result: Value = resp.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("safety-service blocks json: {e}"),
        )
    })?;

    tracing::info!(user_id = %user_id, "bff: user blocked");
    Ok(Json(result))
}

async fn safety_appeal_status(
    State(state): State<Arc<BffState>>,
    Path(appeal_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let _user = identity_user_json(&state, auth).await?;

    let url = format!(
        "{}/api/v1/safety/appeals/{}",
        state.config.safety_service_base_url, appeal_id
    );
    let resp = state
        .client
        .get(&url)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("safety-service appeal: {e}"),
            )
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err((status, format!("safety-service appeal failed: {body_text}")));
    }

    let result: Value = resp.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("safety-service appeal json: {e}"),
        )
    })?;

    Ok(Json(result))
}

async fn locale_registry(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let _user = identity_user_json(&state, auth).await?;

    Ok(Json(
        state
            .locale_registry
            .locale_settings_snapshot(&state.config.env_mode),
    ))
}

async fn compliance_summary(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let user = identity_user_json(&state, auth).await?;
    let user_id = user.get("user_id").and_then(|v| v.as_str()).unwrap_or("");

    let url = format!(
        "{}/api/v1/profile/me/compliance",
        state.config.profile_service_base_url
    );
    let resp = state
        .client
        .get(&url)
        .header(AUTHORIZATION, auth)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .send()
        .await;

    let compliance_data = match resp {
        Ok(r) if r.status().is_success() => r.json::<Value>().await.unwrap_or_else(|_| json!({})),
        _ => {
            tracing::warn!("bff: profile compliance non-success, degrading to default");
            json!({})
        }
    };

    let export_available = compliance_data
        .get("data_export_available")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let delete_available = compliance_data
        .get("data_delete_available")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let correction_available = compliance_data
        .get("data_correction_available")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    Ok(Json(json!({
        "user_id": user_id,
        "data_export_available": export_available,
        "data_delete_available": delete_available,
        "data_correction_available": correction_available,
        "pending_requests": compliance_data.get("pending_requests").cloned().unwrap_or(json!([]))
    })))
}

async fn compliance_export(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let user = identity_user_json(&state, auth).await?;
    let user_id = user.get("user_id").and_then(|v| v.as_str()).unwrap_or("");

    let url = format!(
        "{}/api/v1/profile/me/compliance/export",
        state.config.profile_service_base_url
    );
    let resp = state
        .client
        .post(&url)
        .header(AUTHORIZATION, auth)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .header("Content-Type", "application/json")
        .json(&json!({
            "user_id": user_id,
            "export_format": body.get("export_format").and_then(|v| v.as_str()).unwrap_or("json"),
        }))
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("compliance export: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err((status, format!("compliance export failed: {body_text}")));
    }

    let result: Value = resp.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("compliance export json: {e}"),
        )
    })?;

    tracing::info!(user_id = %user_id, "bff: data export requested");
    Ok(Json(result))
}

async fn compliance_delete(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let user = identity_user_json(&state, auth).await?;
    let user_id = user.get("user_id").and_then(|v| v.as_str()).unwrap_or("");

    let url = format!(
        "{}/api/v1/profile/me/compliance/delete",
        state.config.profile_service_base_url
    );
    let resp = state
        .client
        .post(&url)
        .header(AUTHORIZATION, auth)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .header("Content-Type", "application/json")
        .json(&json!({
            "user_id": user_id,
            "scope": body.get("scope").and_then(|v| v.as_str()).unwrap_or("all"),
        }))
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("compliance delete: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err((status, format!("compliance delete failed: {body_text}")));
    }

    let result: Value = resp.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("compliance delete json: {e}"),
        )
    })?;

    tracing::info!(user_id = %user_id, "bff: data delete requested");
    Ok(Json(result))
}

async fn compliance_correction(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let user = identity_user_json(&state, auth).await?;
    let user_id = user.get("user_id").and_then(|v| v.as_str()).unwrap_or("");

    let url = format!(
        "{}/api/v1/profile/me/compliance/correction",
        state.config.profile_service_base_url
    );
    let resp = state
        .client
        .post(&url)
        .header(AUTHORIZATION, auth)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .header("Content-Type", "application/json")
        .json(&json!({
            "user_id": user_id,
            "field_name": body.get("field_name").and_then(|v| v.as_str()).unwrap_or(""),
        }))
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("compliance correction: {e}"),
            )
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err((status, format!("compliance correction failed: {body_text}")));
    }

    let result: Value = resp.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("compliance correction json: {e}"),
        )
    })?;

    tracing::info!(user_id = %user_id, "bff: data correction requested");
    Ok(Json(result))
}

async fn admin_reports(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let _user = identity_user_json(&state, auth).await?;

    let url = format!(
        "{}/api/v1/admin/reports",
        state.config.admin_service_base_url
    );
    let resp = state
        .client
        .get(&url)
        .header(AUTHORIZATION, auth)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("admin reports: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err((status, format!("admin reports failed: {body_text}")));
    }

    let result: Value = resp
        .json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("admin reports json: {e}")))?;
    Ok(Json(result))
}

async fn admin_report_detail(
    State(state): State<Arc<BffState>>,
    Path(report_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let _user = identity_user_json(&state, auth).await?;

    let url = format!(
        "{}/api/v1/admin/reports/{}",
        state.config.admin_service_base_url, report_id
    );
    let resp = state
        .client
        .get(&url)
        .header(AUTHORIZATION, auth)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("admin report detail: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err((status, format!("admin report detail failed: {body_text}")));
    }

    let result: Value = resp.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("admin report detail json: {e}"),
        )
    })?;
    Ok(Json(result))
}

async fn admin_report_action(
    State(state): State<Arc<BffState>>,
    Path(report_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let _user = identity_user_json(&state, auth).await?;

    let url = format!(
        "{}/api/v1/admin/reports/{}/action",
        state.config.admin_service_base_url, report_id
    );
    let resp = state
        .client
        .post(&url)
        .header(AUTHORIZATION, auth)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("admin report action: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err((status, format!("admin report action failed: {body_text}")));
    }

    let result: Value = resp.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("admin report action json: {e}"),
        )
    })?;
    Ok(Json(result))
}

async fn admin_appeals(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let _user = identity_user_json(&state, auth).await?;

    let url = format!(
        "{}/api/v1/admin/appeals",
        state.config.admin_service_base_url
    );
    let resp = state
        .client
        .get(&url)
        .header(AUTHORIZATION, auth)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("admin appeals: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err((status, format!("admin appeals failed: {body_text}")));
    }

    let result: Value = resp
        .json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("admin appeals json: {e}")))?;
    Ok(Json(result))
}

async fn admin_metrics(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let _user = identity_user_json(&state, auth).await?;

    let url = format!(
        "{}/api/v1/admin/metrics",
        state.config.admin_service_base_url
    );
    let resp = state
        .client
        .get(&url)
        .header(AUTHORIZATION, auth)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("admin metrics: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err((status, format!("admin metrics failed: {body_text}")));
    }

    let result: Value = resp
        .json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("admin metrics json: {e}")))?;
    Ok(Json(result))
}

async fn users_me(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let user = identity_user_json(&state, auth).await?;
    Ok(Json(user))
}

async fn users_me_patch(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let _user = identity_user_json(&state, auth).await?;

    let url = format!(
        "{}/api/v1/profile/me",
        state.config.profile_service_base_url
    );
    let resp = state
        .client
        .patch(&url)
        .header(AUTHORIZATION, auth)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("users me patch: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err((status, format!("users me patch failed: {body_text}")));
    }

    let result: Value = resp
        .json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("users me patch json: {e}")))?;
    Ok(Json(result))
}

async fn dm_conversations(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let user = identity_user_json(&state, auth).await?;
    let user_id = user.get("user_id").and_then(|v| v.as_str()).unwrap_or("");

    let url = format!(
        "{}/api/v1/dm/conversations?user_id={}",
        state.config.dm_service_base_url, user_id
    );
    let resp = state
        .client
        .get(&url)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("dm conversations: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err((status, format!("dm conversations failed: {body_text}")));
    }

    let result: Value = resp.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("dm conversations json: {e}"),
        )
    })?;
    Ok(Json(result))
}

async fn compliance_data(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let user = identity_user_json(&state, auth).await?;
    let user_id = user.get("user_id").and_then(|v| v.as_str()).unwrap_or("");

    let url = format!(
        "{}/api/v1/profile/me/compliance",
        state.config.profile_service_base_url
    );
    let resp = state
        .client
        .get(&url)
        .header(AUTHORIZATION, auth)
        .header(INTERNAL_TOKEN_HEADER, &state.config.internal_shared_secret)
        .send()
        .await;

    let compliance = match resp {
        Ok(r) if r.status().is_success() => r.json::<Value>().await.unwrap_or_else(|_| json!({})),
        _ => {
            tracing::warn!("bff: compliance data non-success, degrading to default");
            json!({})
        }
    };

    Ok(Json(json!({
        "user_id": user_id,
        "data": compliance
    })))
}

async fn region_gate(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let _user = identity_user_json(&state, auth).await?;

    Ok(Json(
        state
            .locale_registry
            .region_gate_snapshot(&state.config.env_mode),
    ))
}

async fn i18n_registry(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let _user = identity_user_json(&state, auth).await?;

    Ok(Json(
        state
            .locale_registry
            .i18n_registry_snapshot(&state.config.env_mode),
    ))
}

#[derive(Debug, Deserialize)]
struct I18nTranslateQuery {
    key: String,
    locale: Option<String>,
}

async fn i18n_translate(
    State(state): State<Arc<BffState>>,
    headers: HeaderMap,
    Query(params): Query<I18nTranslateQuery>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let _user = identity_user_json(&state, auth).await?;

    if params.key.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "key must not be empty".to_string()));
    }

    Ok(Json(state.locale_registry.i18n_translate_lookup(
        &params.key,
        params.locale.as_deref(),
        &state.config.env_mode,
    )))
}
