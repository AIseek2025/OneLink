use std::sync::Arc;

use axum::{
    extract::State,
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    routing::{get, patch, post},
    Json, Router,
};
use serde_json::{json, Value};

use crate::i18n::{self, I18nRegistry, Locale};
use crate::state::AppState;

pub fn router(state: Arc<AppState>) -> Router {
    let bff = Router::new()
        .route("/boot", get(boot))
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/screens", get(screens_index))
        .route("/design-tokens", get(design_tokens_index))
        .route("/contract-freeze", get(contract_freeze_index))
        .route("/analytics/events", post(analytics_events))
        .route("/i18n/locales", get(i18n_locales))
        .route("/i18n/translate/:key", get(i18n_translate))
        .route("/auth/login", post(auth_login))
        .route("/auth/register", post(auth_register))
        .route("/auth/session/refresh", post(session_refresh))
        .route("/home", get(home))
        .route("/find/intent", post(find_intent))
        .route("/find/results", get(find_results))
        .route("/find/requests/:id", get(find_request_detail))
        .route(
            "/find/requests/:id/clarifications",
            post(clarification_submit),
        )
        .route("/recommendations", get(recommendation_list))
        .route("/recommendations/:id", get(recommendation_detail))
        .route(
            "/recommendations/:id/feedback",
            post(recommendation_feedback),
        )
        .route("/dm/threads/draft", post(dm_draft))
        .route("/dm/threads/first-message", post(dm_first_message))
        .route("/dm/threads/:id", get(dm_thread_detail))
        .route("/conversations", get(conversation_list))
        .route("/safety/reports", post(safety_report))
        .route("/safety/blocks", post(safety_block))
        .route("/safety/appeals/:id", get(appeal_status))
        .route("/compliance/summary", get(compliance_summary))
        .route("/compliance/export", post(compliance_export))
        .route("/compliance/correction", post(compliance_correction))
        .route("/compliance/delete", post(compliance_delete))
        .route("/admin/reports", get(admin_reports))
        .route("/admin/reports/:id", get(admin_report_detail))
        .route("/admin/reports/:id/action", post(admin_report_action))
        .route("/admin/appeals", get(admin_appeals))
        .route("/admin/metrics", get(admin_metrics))
        .route("/settings/locale", get(settings_locale))
        .route("/settings/locale/update", patch(settings_locale_update))
        .route("/profile/me", get(profile_me))
        .route("/users/me", get(users_me))
        .route("/region/gate", get(region_gate))
        .with_state(state);

    Router::new().nest("/api/v1/bff", bff)
}

fn extract_locale(headers: &HeaderMap) -> &str {
    headers
        .get("Accept-Language")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("zh-CN")
}

async fn boot(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth_opt = headers.get(AUTHORIZATION).and_then(|v| v.to_str().ok());
    let locale = extract_locale(&headers);

    let (has_session, user) = match auth_opt {
        Some(auth) if !auth.is_empty() => {
            match bff_get(&state, auth, "/api/v1/bff/auth/session/refresh", locale).await {
                Ok(_) => {
                    let me = bff_get(&state, auth, "/api/v1/bff/home", locale).await.ok();
                    (true, me)
                }
                Err(_) => (false, None),
            }
        }
        _ => (false, None),
    };

    Ok(Json(json!({
        "boot_state": {
            "state": if has_session { "session_restored" } else { "no_session" },
            "has_session": has_session,
            "user": user,
        }
    })))
}

async fn health() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "onelink-app",
        "version": "0.1.0"
    }))
}

async fn ready(State(state): State<Arc<AppState>>) -> Json<Value> {
    let bff_reachable = bff_get(&state, "", "/api/v1/bff/health", "zh-CN")
        .await
        .is_ok();

    Json(json!({
        "ready": bff_reachable,
        "checks": {
            "bff_upstream": if bff_reachable { "ok" } else { "unavailable" }
        }
    }))
}

async fn screens_index() -> Json<Value> {
    let screens = onelink_app_server::screens::a0_a1_screens();
    let mut all = screens;
    all.extend(onelink_app_server::screens::a2_screens());
    all.extend(onelink_app_server::screens::a3_screens());
    all.extend(onelink_app_server::screens::a4_screens());
    Json(serde_json::to_value(&all).unwrap_or_default())
}

async fn design_tokens_index() -> Json<Value> {
    Json(
        serde_json::to_value(onelink_app_server::design_tokens::frozen_design_tokens())
            .unwrap_or_default(),
    )
}

async fn contract_freeze_index() -> Json<Value> {
    Json(
        serde_json::to_value(onelink_app_server::contract_freeze::frozen_bff_contract_manifest())
            .unwrap_or_default(),
    )
}

async fn analytics_events(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<AnalyticsBatchRequest>,
) -> Json<Value> {
    let auth_opt = headers.get(AUTHORIZATION).and_then(|v| v.to_str().ok());
    let locale = extract_locale(&headers);
    let event_count = body.events.len();

    let _ = bff_post(
        &state,
        auth_opt.unwrap_or(""),
        "/api/v1/bff/analytics/events",
        &json!({ "events": body.events }),
        locale,
    )
    .await;

    Json(json!({
        "accepted": event_count,
        "status": "logged"
    }))
}

#[derive(serde::Deserialize)]
struct AnalyticsBatchRequest {
    events: Vec<Value>,
}

#[derive(serde::Deserialize)]
struct AuthLoginRequest {
    phone: String,
    code: String,
}

#[derive(serde::Deserialize)]
struct AuthRegisterRequest {
    phone: String,
    nickname: Option<String>,
    avatar_url: Option<String>,
}

#[derive(serde::Deserialize)]
struct SessionRefreshRequest {
    refresh_token: String,
}

#[derive(serde::Deserialize)]
struct FindIntentRequest {
    raw_query: String,
    intent_tags: Option<Vec<String>>,
}

#[derive(serde::Deserialize)]
struct ClarificationSubmitRequest {
    answers: std::collections::HashMap<String, String>,
}

#[derive(serde::Deserialize)]
struct RecommendationFeedbackRequest {
    feedback_type: String,
    comment: Option<String>,
}

#[derive(serde::Deserialize)]
struct DmDraftRequest {
    recommendation_id: String,
    initial_message: String,
}

#[derive(serde::Deserialize)]
struct DmFirstMessageRequest {
    thread_id: String,
    message: String,
}

#[derive(serde::Deserialize)]
struct SafetyReportRequest {
    target_user_id: String,
    category: String,
    description: Option<String>,
}

#[derive(serde::Deserialize)]
struct SafetyBlockRequest {
    target_user_id: String,
    reason: Option<String>,
}

#[derive(serde::Deserialize)]
struct ComplianceExportRequest {
    export_format: Option<String>,
}

#[derive(serde::Deserialize)]
struct ComplianceCorrectionRequest {
    field_name: Option<String>,
}

#[derive(serde::Deserialize)]
struct ComplianceDeleteRequest {
    scope: Option<String>,
}

#[derive(serde::Deserialize)]
struct AdminReportActionRequest {
    action_type: String,
    reason: Option<String>,
}

#[derive(serde::Deserialize)]
struct SettingsLocaleUpdateRequest {
    locale: Option<String>,
    region: Option<String>,
    timezone: Option<String>,
    content_language: Option<String>,
    notification_language: Option<String>,
    notifications_enabled: Option<bool>,
}

async fn i18n_locales() -> Json<Value> {
    let locales = i18n::supported_locales();
    Json(json!({
        "default_locale": i18n::default_locale(),
        "supported_locales": locales,
    }))
}

async fn auth_login(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<AuthLoginRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let locale = extract_locale(&headers);
    let resp = bff_post(
        &state,
        "",
        "/api/v1/bff/auth/login",
        &json!({ "phone": body.phone, "code": body.code }),
        locale,
    )
    .await?;
    Ok(Json(resp))
}

async fn auth_register(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<AuthRegisterRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let locale = extract_locale(&headers);
    let mut payload = json!({ "phone": body.phone });
    if let Some(nick) = body.nickname {
        payload["nickname"] = json!(nick);
    }
    if let Some(avatar) = body.avatar_url {
        payload["avatar_url"] = json!(avatar);
    }
    let resp = bff_post(&state, "", "/api/v1/bff/auth/register", &payload, locale).await?;
    Ok(Json(resp))
}

async fn session_refresh(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SessionRefreshRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let locale = extract_locale(&headers);
    let resp = bff_post(
        &state,
        "",
        "/api/v1/bff/auth/session/refresh",
        &json!({ "refresh_token": body.refresh_token }),
        locale,
    )
    .await?;
    Ok(Json(resp))
}

async fn home(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let resp = bff_get(&state, auth, "/api/v1/bff/home", locale).await?;
    Ok(Json(resp))
}

async fn find_intent(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<FindIntentRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let mut payload = json!({ "raw_query": body.raw_query });
    if let Some(tags) = body.intent_tags {
        payload["intent_tags"] = json!(tags);
    }
    let resp = bff_post(&state, auth, "/api/v1/bff/find/intent", &payload, locale).await?;
    Ok(Json(resp))
}

async fn find_results(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let resp = bff_get(&state, auth, "/api/v1/bff/find/results", locale).await?;
    Ok(Json(resp))
}

async fn find_request_detail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let resp = bff_get(
        &state,
        auth,
        &format!("/api/v1/bff/find/requests/{}", id),
        locale,
    )
    .await?;
    Ok(Json(resp))
}

async fn clarification_submit(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<ClarificationSubmitRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let resp = bff_post(
        &state,
        auth,
        &format!("/api/v1/bff/find/requests/{}/clarifications", id),
        &json!({ "answers": body.answers }),
        locale,
    )
    .await?;
    Ok(Json(resp))
}

async fn recommendation_list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let resp = bff_get(&state, auth, "/api/v1/bff/recommendations", locale).await?;
    Ok(Json(resp))
}

async fn recommendation_detail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let resp = bff_get(
        &state,
        auth,
        &format!("/api/v1/bff/recommendations/{}", id),
        locale,
    )
    .await?;
    Ok(Json(resp))
}

async fn recommendation_feedback(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<RecommendationFeedbackRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let mut payload = json!({ "feedback_type": body.feedback_type });
    if let Some(comment) = body.comment {
        payload["comment"] = json!(comment);
    }
    let resp = bff_post(
        &state,
        auth,
        &format!("/api/v1/bff/recommendations/{}/feedback", id),
        &payload,
        locale,
    )
    .await?;
    Ok(Json(resp))
}

async fn dm_draft(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<DmDraftRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let resp = bff_post(
        &state,
        auth,
        "/api/v1/bff/dm/threads/draft",
        &json!({
            "recommendation_id": body.recommendation_id,
            "initial_message": body.initial_message
        }),
        locale,
    )
    .await?;
    Ok(Json(resp))
}

async fn dm_first_message(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<DmFirstMessageRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let resp = bff_post(
        &state,
        auth,
        "/api/v1/bff/dm/threads/first-message",
        &json!({
            "thread_id": body.thread_id,
            "message": body.message
        }),
        locale,
    )
    .await?;
    Ok(Json(resp))
}

async fn dm_thread_detail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let resp = bff_get(
        &state,
        auth,
        &format!("/api/v1/bff/dm/threads/{}", id),
        locale,
    )
    .await?;
    Ok(Json(resp))
}

async fn conversation_list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let resp = bff_get(&state, auth, "/api/v1/bff/conversations", locale).await?;
    Ok(Json(resp))
}

async fn safety_report(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SafetyReportRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let mut payload = json!({
        "target_user_id": body.target_user_id,
        "category": body.category
    });
    if let Some(desc) = body.description {
        payload["description"] = json!(desc);
    }
    let resp = bff_post(&state, auth, "/api/v1/bff/safety/reports", &payload, locale).await?;
    Ok(Json(resp))
}

async fn safety_block(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SafetyBlockRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let mut payload = json!({ "target_user_id": body.target_user_id });
    if let Some(reason) = body.reason {
        payload["reason"] = json!(reason);
    }
    let resp = bff_post(&state, auth, "/api/v1/bff/safety/blocks", &payload, locale).await?;
    Ok(Json(resp))
}

async fn appeal_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let resp = bff_get(
        &state,
        auth,
        &format!("/api/v1/bff/safety/appeals/{}", id),
        locale,
    )
    .await?;
    Ok(Json(resp))
}

async fn compliance_summary(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let resp = bff_get(&state, auth, "/api/v1/bff/compliance/summary", locale).await?;
    Ok(Json(resp))
}

async fn compliance_export(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ComplianceExportRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let mut payload = json!({});
    if let Some(fmt) = body.export_format {
        payload["export_format"] = json!(fmt);
    }
    let resp = bff_post(
        &state,
        auth,
        "/api/v1/bff/compliance/export",
        &payload,
        locale,
    )
    .await?;
    Ok(Json(resp))
}

async fn compliance_correction(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ComplianceCorrectionRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let mut payload = json!({});
    if let Some(field) = body.field_name {
        payload["field_name"] = json!(field);
    }
    let resp = bff_post(
        &state,
        auth,
        "/api/v1/bff/compliance/correction",
        &payload,
        locale,
    )
    .await?;
    Ok(Json(resp))
}

async fn compliance_delete(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ComplianceDeleteRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let mut payload = json!({});
    if let Some(scope) = body.scope {
        payload["scope"] = json!(scope);
    }
    let resp = bff_post(
        &state,
        auth,
        "/api/v1/bff/compliance/delete",
        &payload,
        locale,
    )
    .await?;
    Ok(Json(resp))
}

async fn admin_reports(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let resp = bff_get(&state, auth, "/api/v1/bff/admin/reports", locale).await?;
    Ok(Json(resp))
}

async fn admin_report_detail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let resp = bff_get(
        &state,
        auth,
        &format!("/api/v1/bff/admin/reports/{}", id),
        locale,
    )
    .await?;
    Ok(Json(resp))
}

async fn admin_report_action(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<AdminReportActionRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let mut payload = json!({ "action_type": body.action_type });
    if let Some(reason) = body.reason {
        payload["reason"] = json!(reason);
    }
    let resp = bff_post(
        &state,
        auth,
        &format!("/api/v1/bff/admin/reports/{}/action", id),
        &payload,
        locale,
    )
    .await?;
    Ok(Json(resp))
}

async fn admin_appeals(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let resp = bff_get(&state, auth, "/api/v1/bff/admin/appeals", locale).await?;
    Ok(Json(resp))
}

async fn admin_metrics(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let resp = bff_get(&state, auth, "/api/v1/bff/admin/metrics", locale).await?;
    Ok(Json(resp))
}

async fn settings_locale(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let resp = bff_get(&state, auth, "/api/v1/bff/settings/locale", locale).await?;
    Ok(Json(resp))
}

async fn settings_locale_update(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SettingsLocaleUpdateRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let mut payload = json!({});
    if let Some(v) = body.locale {
        payload["locale"] = json!(v);
    }
    if let Some(v) = body.region {
        payload["region"] = json!(v);
    }
    if let Some(v) = body.timezone {
        payload["timezone"] = json!(v);
    }
    if let Some(v) = body.content_language {
        payload["content_language"] = json!(v);
    }
    if let Some(v) = body.notification_language {
        payload["notification_language"] = json!(v);
    }
    if let Some(v) = body.notifications_enabled {
        payload["notifications_enabled"] = json!(v);
    }
    let resp = bff_post(
        &state,
        auth,
        "/api/v1/bff/settings/locale/update",
        &payload,
        locale,
    )
    .await?;
    Ok(Json(resp))
}

async fn profile_me(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let resp = bff_get(&state, auth, "/api/v1/bff/profile/me", locale).await?;
    Ok(Json(resp))
}

async fn users_me(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let resp = bff_get(&state, auth, "/api/v1/bff/users/me", locale).await?;
    Ok(Json(resp))
}

async fn region_gate(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let locale = extract_locale(&headers);
    let resp = bff_get(&state, auth, "/api/v1/bff/region/gate", locale).await?;
    Ok(Json(resp))
}

async fn i18n_translate(
    headers: HeaderMap,
    axum::extract::Path(key): axum::extract::Path<String>,
) -> Json<Value> {
    let locale = Locale::resolve_or_default(extract_locale(&headers));
    let registry = I18nRegistry::new();
    match registry.translate(&key, &locale) {
        Some(text) => Json(json!({
            "key": key,
            "locale": locale.tag(),
            "text": text,
        })),
        None => Json(json!({
            "key": key,
            "locale": locale.tag(),
            "text": null,
            "error": "translation_not_found",
        })),
    }
}

async fn bff_get(
    state: &AppState,
    auth: &str,
    path: &str,
    locale: &str,
) -> Result<Value, (StatusCode, String)> {
    let url = format!("{}{}", state.config.bff_base_url, path);
    let resp = state
        .http
        .get(&url)
        .header(AUTHORIZATION, auth)
        .header("Accept-Language", locale)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("bff get {}: {e}", path)))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err((status, format!("bff {} failed: {body}", path)));
    }
    resp.json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("bff {} json: {e}", path)))
}

async fn bff_post(
    state: &AppState,
    auth: &str,
    path: &str,
    body: &Value,
    locale: &str,
) -> Result<Value, (StatusCode, String)> {
    let url = format!("{}{}", state.config.bff_base_url, path);
    let resp = state
        .http
        .post(&url)
        .header(AUTHORIZATION, auth)
        .header("Content-Type", "application/json")
        .header("Accept-Language", locale)
        .json(body)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("bff post {}: {e}", path)))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        return Err((status, format!("bff {} failed: {body_text}", path)));
    }
    resp.json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("bff {} json: {e}", path)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn test_state() -> Arc<AppState> {
        let config = AppConfig {
            port: 0,
            bff_base_url: "http://127.0.0.1:19999".into(),
            ..Default::default()
        };
        AppState::new(config)
    }

    fn test_app() -> Router {
        router(test_state())
    }

    #[test]
    fn test_router_constructs() {
        let state = test_state();
        let _app = router(state);
    }

    #[tokio::test]
    async fn test_i18n_locales_handler() {
        let app = test_app();
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/v1/bff/i18n/locales")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["default_locale"], "zh-CN");
        assert!(v["supported_locales"].is_array());
        let arr = v["supported_locales"].as_array().unwrap();
        assert!(arr.iter().any(|l| l == "zh-CN"));
        assert!(arr.iter().any(|l| l == "en"));
    }

    #[tokio::test]
    async fn test_i18n_translate_handler_found() {
        let app = test_app();
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/v1/bff/i18n/translate/safety.block.applied")
                    .header("Accept-Language", "zh-CN")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["key"], "safety.block.applied");
        assert_eq!(v["locale"], "zh-CN");
        assert!(v["text"].is_string());
    }

    #[tokio::test]
    async fn test_i18n_translate_handler_en() {
        let app = test_app();
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/v1/bff/i18n/translate/safety.block.applied")
                    .header("Accept-Language", "en")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["locale"], "en");
        assert_eq!(v["text"], "This user has been blocked");
    }

    #[tokio::test]
    async fn test_i18n_translate_handler_not_found() {
        let app = test_app();
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/v1/bff/i18n/translate/nonexistent.key")
                    .header("Accept-Language", "zh-CN")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let v: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["error"], "translation_not_found");
    }
}
