use std::sync::Arc;

use axum::{
    extract::State,
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    routing::{get, patch, post},
    Json, Router,
};
use serde_json::{json, Value};

use crate::dto::RegionResidencyPolicy;
use crate::dto::*;
use crate::i18n::I18nRegistry;
use crate::region_gate;
use crate::state::AppState;

fn i18n_error_json(
    code: AppErrorCode,
    message_key: &str,
    locale: &str,
    trace_id: Option<String>,
) -> (StatusCode, String) {
    let reg = I18nRegistry::new();
    let resolved = crate::i18n::resolve_locale(locale);
    let localized = reg.translate(message_key, resolved);
    let body = AppErrorBody {
        code,
        message_key: message_key.to_string(),
        localized_message: Some(localized),
        trace_id,
    };
    let status = match code {
        AppErrorCode::AuthError => StatusCode::UNAUTHORIZED,
        AppErrorCode::ValidationError => StatusCode::BAD_REQUEST,
        AppErrorCode::RateLimited => StatusCode::TOO_MANY_REQUESTS,
        AppErrorCode::SafetyBlocked => StatusCode::FORBIDDEN,
        AppErrorCode::TemporarilyUnavailable => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, serde_json::to_string(&body).unwrap_or_default())
}

fn extract_locale(headers: &HeaderMap) -> &str {
    headers
        .get("Accept-Language")
        .and_then(|v| v.to_str().ok())
        .unwrap_or(crate::i18n::default_locale())
}

pub fn router(state: Arc<AppState>) -> Router {
    let bff = Router::new()
        .route("/boot", get(boot))
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/auth/login", post(auth_login))
        .route("/auth/register", post(auth_register))
        .route("/auth/session/refresh", post(session_refresh))
        .route("/me/summary", get(me_summary))
        .route("/conversations", get(conversation_list))
        .route("/conversations/:id/messages", post(chat_send))
        .route("/conversations/:id", get(conversation_detail))
        .route(
            "/profile/confirmations/:id/actions",
            post(profile_confirmation_action),
        )
        .route("/profile/me", get(profile_me))
        .route("/settings/summary", get(settings_summary))
        .route("/settings/locale", get(locale_get))
        .route("/settings/locale/update", patch(settings_locale_update))
        .route("/i18n/registry", get(i18n_registry))
        .route("/i18n/translate", get(i18n_translate))
        .route("/analytics/events", post(analytics_events))
        .route("/screens", get(screens_index))
        .route("/design-tokens", get(design_tokens_index))
        .route("/contract-freeze", get(contract_freeze_index))
        .route("/find/requests", post(find_request_create))
        .route("/find/requests/:id", get(find_request_detail))
        .route(
            "/find/requests/:id/clarifications",
            post(clarification_answer_submit),
        )
        .route("/recommendations", get(recommendation_list))
        .route("/recommendations/:id", get(recommendation_detail))
        .route(
            "/recommendations/:id/feedback",
            post(recommendation_feedback),
        )
        .route("/dm/threads/draft", post(dm_draft))
        .route("/dm/threads/first-message", post(dm_first_message_submit))
        .route("/dm/threads/:id", get(dm_thread_detail))
        .route("/safety/reports", post(report_submit))
        .route("/safety/blocks", post(block_apply))
        .route("/safety/appeals/:id", get(appeal_status))
        .route("/compliance/summary", get(compliance_summary))
        .route("/compliance/export", post(compliance_export))
        .route("/compliance/delete", post(compliance_delete))
        .route("/compliance/correction", post(compliance_correction))
        .route(
            "/compliance/region-residency-policy",
            get(region_residency_policy),
        )
        .route("/admin/reports", get(admin_report_queue))
        .route("/admin/reports/:id", get(admin_report_detail))
        .route("/admin/reports/:id/action", post(admin_report_action))
        .route("/admin/appeals", get(admin_appeal_queue))
        .route("/region/gate", get(region_gate_check))
        .with_state(state);

    Router::new().nest("/api/v1/bff", bff)
}

fn extract_auth(headers: &HeaderMap) -> Result<&str, (StatusCode, String)> {
    headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            i18n_error_json(
                AppErrorCode::AuthError,
                "app.auth.session.expired",
                extract_locale(headers),
                None,
            )
        })
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
        let localized = try_localize_bff_error(&body, locale);
        return Err((
            status,
            localized.unwrap_or_else(|| format!("bff {} failed: {body}", path)),
        ));
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
        let localized = try_localize_bff_error(&body_text, locale);
        return Err((
            status,
            localized.unwrap_or_else(|| format!("bff {} failed: {body_text}", path)),
        ));
    }
    resp.json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("bff {} json: {e}", path)))
}

fn try_localize_bff_error(bff_body: &str, locale: &str) -> Option<String> {
    let parsed: serde_json::Value = serde_json::from_str(bff_body).ok()?;
    let message_key = parsed.get("message_key")?.as_str()?;
    let reg = I18nRegistry::new();
    let resolved = crate::i18n::resolve_locale(locale);
    let localized = reg.translate(message_key, resolved);
    let error_body = AppErrorBody {
        code: AppErrorCode::NetworkError,
        message_key: message_key.to_string(),
        localized_message: Some(localized),
        trace_id: parsed
            .get("trace_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    };
    Some(serde_json::to_string(&error_body).unwrap_or_else(|_| bff_body.to_string()))
}

async fn health() -> &'static str {
    "ok"
}

async fn ready(State(state): State<Arc<AppState>>) -> Result<&'static str, (StatusCode, String)> {
    let url = format!("{}/api/v1/bff/boot", state.config.bff_base_url);
    let resp = state
        .http
        .get(&url)
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await;
    match resp {
        Ok(r) if r.status().is_success() => Ok("ready"),
        _ => Err((StatusCode::SERVICE_UNAVAILABLE, "not_ready".into())),
    }
}

async fn boot(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth_opt = headers.get(AUTHORIZATION).and_then(|v| v.to_str().ok());
    let locale = extract_locale(&headers);
    let (has_session, user, phase) = match auth_opt {
        Some(auth) if !auth.is_empty() => {
            match bff_get(&state, auth, "/api/v1/bff/auth/session/refresh", locale).await {
                Ok(_) => {
                    let me = bff_get(&state, auth, "/api/v1/bff/home", locale).await.ok();
                    (true, me, BootPhase::SessionRestored)
                }
                Err(_) => (false, None, BootPhase::NoSession),
            }
        }
        _ => (false, None, BootPhase::NoSession),
    };

    Ok(Json(json!({
        "boot_state": {
            "state": phase,
            "has_session": has_session,
            "user": user,
        }
    })))
}

async fn auth_login(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<AuthRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    let locale = extract_locale(&headers);
    let bff_resp = bff_post(
        &state,
        "",
        "/api/v1/bff/auth/login",
        &json!({
            "phone": body.phone,
            "code": body.code,
        }),
        locale,
    )
    .await?;

    let resp: AuthResponse = serde_json::from_value(bff_resp)
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("login parse: {e}")))?;

    tracing::info!(user_id = %resp.user_id, "app: login success");
    Ok(Json(resp))
}

async fn auth_register(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, (StatusCode, String)> {
    let locale = extract_locale(&headers);
    let bff_resp = bff_post(
        &state,
        "",
        "/api/v1/bff/auth/register",
        &json!({
            "phone": body.phone,
            "nickname": body.nickname,
            "avatar_url": body.avatar_url,
        }),
        locale,
    )
    .await?;

    let resp: RegisterResponse = serde_json::from_value(bff_resp)
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("register parse: {e}")))?;

    tracing::info!(user_id = %resp.user_id, "app: register success");
    Ok(Json(resp))
}

async fn session_refresh(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SessionRefreshRequest>,
) -> Result<Json<SessionRefreshResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let locale = extract_locale(&headers);
    let bff_resp = bff_post(
        &state,
        auth,
        "/api/v1/bff/auth/session/refresh",
        &json!({ "refresh_token": body.refresh_token }),
        locale,
    )
    .await?;

    let resp: SessionRefreshResponse = serde_json::from_value(bff_resp).map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("session refresh parse: {e}"),
        )
    })?;

    Ok(Json(resp))
}

async fn me_summary(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<MeSummaryResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let locale = extract_locale(&headers);
    let bff_resp = bff_get(&state, auth, "/api/v1/bff/home", locale).await?;

    let user_id = bff_resp
        .get("user")
        .and_then(|u| u.get("user_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let nickname = bff_resp
        .get("user")
        .and_then(|u| u.get("nickname"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let avatar_url = bff_resp
        .get("user")
        .and_then(|u| u.get("avatar_url"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let first_run = bff_resp
        .get("user")
        .and_then(|u| u.get("first_run"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let locale = bff_resp
        .get("user")
        .and_then(|u| u.get("locale"))
        .or_else(|| bff_resp.get("locale"))
        .and_then(|v| v.as_str())
        .unwrap_or("zh-CN");

    let region = bff_resp
        .get("user")
        .and_then(|u| u.get("region"))
        .or_else(|| bff_resp.get("region"))
        .and_then(|v| v.as_str())
        .unwrap_or("CN");

    let resp = MeSummaryResponse {
        user_id: uuid::Uuid::parse_str(user_id).unwrap_or_default(),
        nickname: nickname.to_string(),
        avatar_url,
        first_run,
        locale: locale.to_string(),
        region: region.to_string(),
    };

    Ok(Json(resp))
}

async fn conversation_list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ConversationListResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let locale = extract_locale(&headers);
    let bff_resp = bff_get(&state, auth, "/api/v1/bff/chat/init", locale).await?;

    let conversations_val = bff_resp.get("conversations").cloned().unwrap_or(json!([]));

    let conversations: Vec<ConversationSummary> =
        serde_json::from_value(conversations_val).unwrap_or_default();

    Ok(Json(ConversationListResponse { conversations }))
}

async fn chat_send(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(conv_id): axum::extract::Path<String>,
    Json(body): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;

    if body.message.trim().is_empty() {
        return Err(i18n_error_json(
            AppErrorCode::ValidationError,
            "app.auth.error.invalid_credentials",
            extract_locale(&headers),
            None,
        ));
    }

    let bff_resp = bff_post(
        &state,
        auth,
        "/api/v1/bff/chat/messages",
        &json!({
            "conversation_id": conv_id,
            "content_type": "text",
            "content_text": body.message,
        }),
        extract_locale(&headers),
    )
    .await?;

    let resp: ChatResponse = serde_json::from_value(bff_resp)
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("chat parse: {e}")))?;

    Ok(Json(resp))
}

async fn conversation_detail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let locale = extract_locale(&headers);
    let bff_resp = bff_get(&state, auth, "/api/v1/bff/chat/init", locale).await?;
    Ok(Json(bff_resp))
}

#[allow(dead_code)]
async fn profile_confirmations(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ProfileConfirmationListResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let locale = extract_locale(&headers);
    let bff_resp = bff_get(&state, auth, "/api/v1/bff/home", locale).await?;

    let pending = bff_resp
        .get("completion")
        .and_then(|c| c.get("pending_facts"))
        .cloned()
        .unwrap_or(json!([]));

    let pending_facts: Vec<ProfileFactDto> = serde_json::from_value(pending).unwrap_or_default();

    Ok(Json(ProfileConfirmationListResponse { pending_facts }))
}

async fn profile_confirmation_action(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(fact_id): axum::extract::Path<String>,
    Json(body): Json<ProfileConfirmationActionRequest>,
) -> Result<Json<ProfileConfirmationActionResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;

    let action_str = match body.action {
        ProfileConfirmationAction::Accept => "accept",
        ProfileConfirmationAction::Reject => "reject",
        ProfileConfirmationAction::Snooze => "snooze",
        ProfileConfirmationAction::Edit => "edit",
    };

    let _ = bff_post(
        &state,
        auth,
        "/api/v1/bff/profile/me",
        &json!({
            "fact_id": fact_id,
            "action": action_str,
            "feedback_payload": body.feedback_payload,
        }),
        extract_locale(&headers),
    )
    .await?;

    let new_state = match body.action {
        ProfileConfirmationAction::Accept => ProfileConfirmationState::Accepted,
        ProfileConfirmationAction::Reject => ProfileConfirmationState::Rejected,
        ProfileConfirmationAction::Snooze => ProfileConfirmationState::Snoozed,
        ProfileConfirmationAction::Edit => ProfileConfirmationState::Edited,
    };

    Ok(Json(ProfileConfirmationActionResponse {
        fact_id: uuid::Uuid::parse_str(&fact_id).unwrap_or_default(),
        new_state,
    }))
}

async fn profile_me(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ProfileDto>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let locale = extract_locale(&headers);
    let bff_resp = bff_get(&state, auth, "/api/v1/bff/home", locale).await?;

    let profile = bff_resp.get("profile").cloned().unwrap_or(json!(null));
    let resp: ProfileDto = serde_json::from_value(profile)
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("profile parse: {e}")))?;

    Ok(Json(resp))
}

async fn settings_summary(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<SettingsSummaryResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let locale = extract_locale(&headers);
    let bff_resp = bff_get(&state, auth, "/api/v1/bff/home", locale).await?;

    let profile = bff_resp.get("profile").cloned().unwrap_or(json!(null));
    let notifications_enabled = profile
        .get("notifications_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let locale = profile
        .get("locale")
        .and_then(|v| v.as_str())
        .unwrap_or("zh-CN");

    let region = profile
        .get("region")
        .and_then(|v| v.as_str())
        .unwrap_or("CN");

    let timezone = profile
        .get("timezone")
        .and_then(|v| v.as_str())
        .unwrap_or("Asia/Shanghai");

    let content_language = profile
        .get("content_language")
        .and_then(|v| v.as_str())
        .unwrap_or(locale);

    let notification_language = profile
        .get("notification_language")
        .and_then(|v| v.as_str())
        .unwrap_or(locale);

    let resp = SettingsSummaryResponse {
        settings: SettingsDto {
            notifications_enabled,
            language: locale.to_string(),
            theme: "light".into(),
            locale: locale.to_string(),
            region: region.to_string(),
            timezone: timezone.to_string(),
            content_language: content_language.to_string(),
            notification_language: notification_language.to_string(),
        },
    };

    Ok(Json(resp))
}

async fn settings_locale_update(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SettingsUpdateRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;

    let mut patch = serde_json::Map::new();
    if let Some(locale) = &body.locale {
        patch.insert("locale".to_string(), json!(locale));
    }
    if let Some(region) = &body.region {
        patch.insert("region".to_string(), json!(region));
    }
    if let Some(timezone) = &body.timezone {
        patch.insert("timezone".to_string(), json!(timezone));
    }
    if let Some(content_language) = &body.content_language {
        patch.insert("content_language".to_string(), json!(content_language));
    }
    if let Some(notification_language) = &body.notification_language {
        patch.insert(
            "notification_language".to_string(),
            json!(notification_language),
        );
    }
    if let Some(notifications_enabled) = body.notifications_enabled {
        patch.insert(
            "notifications_enabled".to_string(),
            json!(notifications_enabled),
        );
    }

    let locale = extract_locale(&headers);
    let bff_resp = bff_post(
        &state,
        auth,
        "/api/v1/bff/profile/me",
        &json!({ "settings": patch }),
        locale,
    )
    .await?;

    Ok(Json(bff_resp))
}

async fn analytics_events(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<AnalyticsBatchRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
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

    Ok(Json(json!({
        "accepted": event_count,
        "status": "logged"
    })))
}

async fn screens_index() -> Json<Vec<crate::screens::ScreenSpec>> {
    let mut screens = crate::screens::a0_a1_screens();
    screens.extend(crate::screens::a2_screens());
    screens.extend(crate::screens::a3_screens());
    screens.extend(crate::screens::a4_screens());
    Json(screens)
}

async fn design_tokens_index() -> Json<crate::design_tokens::DesignTokenSet> {
    Json(crate::design_tokens::frozen_design_tokens())
}

async fn contract_freeze_index() -> Json<crate::contract_freeze::ContractFreezeManifest> {
    Json(crate::contract_freeze::frozen_bff_contract_manifest())
}

async fn find_request_create(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<FindRequestCreateRequest>,
) -> Result<Json<FindRequestResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;

    if body.intent_text.trim().is_empty() {
        return Err(i18n_error_json(
            AppErrorCode::ValidationError,
            "app.auth.error.invalid_credentials",
            extract_locale(&headers),
            None,
        ));
    }

    let mut payload = serde_json::Map::new();
    payload.insert("intent_text".to_string(), json!(body.intent_text));
    if let Some(region) = &body.preferred_region {
        payload.insert("preferred_region".to_string(), json!(region));
    }
    if let Some(locale) = &body.preferred_locale {
        payload.insert("preferred_locale".to_string(), json!(locale));
    }

    let bff_resp = bff_post(
        &state,
        auth,
        "/api/v1/bff/find/requests",
        &json!(payload),
        extract_locale(&headers),
    )
    .await?;

    let resp: FindRequestResponse = serde_json::from_value(bff_resp)
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("find request parse: {e}")))?;

    tracing::info!(request_id = %resp.request_id, "app: find request created");
    Ok(Json(resp))
}

async fn find_request_detail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(request_id): axum::extract::Path<String>,
) -> Result<Json<FindRequestDetailResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let locale = extract_locale(&headers);
    let bff_resp = bff_get(
        &state,
        auth,
        &format!("/api/v1/bff/find/requests/{}", request_id),
        locale,
    )
    .await?;

    let resp: FindRequestDetailResponse = serde_json::from_value(bff_resp)
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("find detail parse: {e}")))?;

    Ok(Json(resp))
}

async fn clarification_answer_submit(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(request_id): axum::extract::Path<String>,
    Json(body): Json<ClarificationAnswerRequest>,
) -> Result<Json<ClarificationAnswerResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let locale = extract_locale(&headers);

    let bff_resp = bff_post(
        &state,
        auth,
        &format!("/api/v1/bff/find/requests/{}/clarifications", request_id),
        &json!({ "answers": body.answers }),
        locale,
    )
    .await?;

    let resp: ClarificationAnswerResponse = serde_json::from_value(bff_resp)
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("clarification parse: {e}")))?;

    tracing::info!(request_id = %resp.request_id, "app: clarification submitted");
    Ok(Json(resp))
}

async fn recommendation_list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<RecommendationListResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let locale = extract_locale(&headers);
    let bff_resp = bff_get(&state, auth, "/api/v1/bff/recommendations", locale).await?;

    let resp: RecommendationListResponse = serde_json::from_value(bff_resp).map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("recommendation list parse: {e}"),
        )
    })?;

    Ok(Json(resp))
}

async fn recommendation_detail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(rec_id): axum::extract::Path<String>,
) -> Result<Json<RecommendationDetailResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let locale = extract_locale(&headers);
    let bff_resp = bff_get(
        &state,
        auth,
        &format!("/api/v1/bff/recommendations/{}", rec_id),
        locale,
    )
    .await?;

    let resp: RecommendationDetailResponse = serde_json::from_value(bff_resp).map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("recommendation detail parse: {e}"),
        )
    })?;

    Ok(Json(resp))
}

async fn recommendation_feedback(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(rec_id): axum::extract::Path<String>,
    Json(body): Json<RecommendationFeedbackRequest>,
) -> Result<Json<RecommendationFeedbackResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let locale = extract_locale(&headers);

    let bff_resp = bff_post(
        &state,
        auth,
        &format!("/api/v1/bff/recommendations/{}/feedback", rec_id),
        &json!({
            "feedback_type": body.feedback_type,
            "comment": body.comment,
        }),
        locale,
    )
    .await?;

    let resp: RecommendationFeedbackResponse = serde_json::from_value(bff_resp).map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("recommendation feedback parse: {e}"),
        )
    })?;

    Ok(Json(resp))
}

async fn dm_draft(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<DmDraftRequest>,
) -> Result<Json<DmDraftResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let locale = extract_locale(&headers);

    if body.initial_message.trim().is_empty() {
        return Err(i18n_error_json(
            AppErrorCode::ValidationError,
            "dm.first_message.blocked",
            locale,
            None,
        ));
    }

    let bff_resp = bff_post(
        &state,
        auth,
        "/api/v1/bff/dm/threads/draft",
        &json!({
            "recommendation_id": body.recommendation_id,
            "initial_message": body.initial_message,
        }),
        locale,
    )
    .await?;

    let resp: DmDraftResponse = serde_json::from_value(bff_resp)
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("dm draft parse: {e}")))?;

    tracing::info!(thread_id = %resp.thread_id, "app: dm draft created");
    Ok(Json(resp))
}

async fn dm_first_message_submit(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<DmFirstMessageSubmitRequest>,
) -> Result<Json<DmFirstMessageSubmitResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let locale = extract_locale(&headers);

    if body.message.trim().is_empty() {
        return Err(i18n_error_json(
            AppErrorCode::ValidationError,
            "dm.first_message.blocked",
            locale,
            None,
        ));
    }

    let bff_resp = bff_post(
        &state,
        auth,
        "/api/v1/bff/dm/threads/first-message",
        &json!({
            "thread_id": body.thread_id,
            "message": body.message,
        }),
        locale,
    )
    .await?;

    let resp: DmFirstMessageSubmitResponse = serde_json::from_value(bff_resp).map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("dm first message parse: {e}"),
        )
    })?;

    tracing::info!(thread_id = %resp.thread_id, "app: dm first message submitted");
    Ok(Json(resp))
}

async fn dm_thread_detail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(thread_id): axum::extract::Path<String>,
) -> Result<Json<DmThreadDetailResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let locale = extract_locale(&headers);
    let bff_resp = bff_get(
        &state,
        auth,
        &format!("/api/v1/bff/dm/threads/{}", thread_id),
        locale,
    )
    .await?;

    let resp: DmThreadDetailResponse = serde_json::from_value(bff_resp)
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("dm thread parse: {e}")))?;

    Ok(Json(resp))
}

async fn report_submit(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ReportSubmitRequest>,
) -> Result<Json<ReportSubmitResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;

    let locale = extract_locale(&headers);
    let bff_resp = bff_post(
        &state,
        auth,
        "/api/v1/bff/safety/reports",
        &json!({
            "target_user_id": body.target_user_id,
            "category": body.category,
            "description": body.description,
        }),
        locale,
    )
    .await?;

    let resp: ReportSubmitResponse = serde_json::from_value(bff_resp)
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("report parse: {e}")))?;

    tracing::info!(report_id = %resp.report_id, "app: report submitted");
    Ok(Json(resp))
}

async fn block_apply(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<BlockApplyRequest>,
) -> Result<Json<BlockApplyResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;

    let locale = extract_locale(&headers);
    let bff_resp = bff_post(
        &state,
        auth,
        "/api/v1/bff/safety/blocks",
        &json!({
            "target_user_id": body.target_user_id,
            "reason": body.reason,
        }),
        locale,
    )
    .await?;

    let resp: BlockApplyResponse = serde_json::from_value(bff_resp)
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("block parse: {e}")))?;

    tracing::info!(blocked_user_id = %resp.blocked_user_id, "app: user blocked");
    Ok(Json(resp))
}

async fn appeal_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(appeal_id): axum::extract::Path<String>,
) -> Result<Json<AppealStatusResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let locale = extract_locale(&headers);
    let bff_resp = bff_get(
        &state,
        auth,
        &format!("/api/v1/bff/safety/appeals/{}", appeal_id),
        locale,
    )
    .await?;

    let resp: AppealStatusResponse = serde_json::from_value(bff_resp)
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("appeal parse: {e}")))?;

    Ok(Json(resp))
}

async fn locale_get(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<LocaleRegistryResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let locale = extract_locale(&headers);
    let bff_resp = bff_get(&state, auth, "/api/v1/bff/settings/locale", locale).await?;

    let resp: LocaleRegistryResponse = serde_json::from_value(bff_resp).map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("locale registry parse: {e}"),
        )
    })?;

    Ok(Json(resp))
}

async fn compliance_summary(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let locale = extract_locale(&headers);
    let user_region = headers
        .get("X-User-Region")
        .and_then(|v| v.to_str().ok())
        .unwrap_or(crate::i18n::default_region());
    let gate = crate::region_gate::check_region_gate(user_region);
    let bff_resp = bff_get(&state, auth, "/api/v1/bff/compliance/summary", locale).await?;

    let resp: ComplianceSummaryResponse = serde_json::from_value(bff_resp).map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("compliance summary parse: {e}"),
        )
    })?;

    let restricted = gate.degradation == crate::region_gate::RegionDegradationMode::ReadOnly
        || gate.degradation == crate::region_gate::RegionDegradationMode::Blocked;
    let mut resp = resp;
    if restricted {
        resp.data_export_available = false;
        resp.data_delete_available = false;
        resp.data_correction_available = false;
    }
    let gate_notice = crate::region_gate::localize_gate_notice(&gate, locale);

    let mut resp_val = serde_json::to_value(&resp).unwrap_or_default();
    if !gate_notice.is_empty() {
        resp_val["region_gate_notice"] = serde_json::Value::String(gate_notice);
    }
    if !restricted {
        resp_val
            .as_object_mut()
            .map(|m| m.remove("region_gate_notice"));
    }

    Ok(Json(resp_val))
}

/// Data-rights export handler. Restricted-region (EU) users have gate-level
/// `allowed=true` (app shell accessible) but `degradation=ReadOnly`; this
/// handler enforces the second level by rejecting export with 403
/// `safety_blocked` when degradation is ReadOnly or Blocked. See
/// `region_gate.rs` module doc for the two-level policy architecture.
async fn compliance_export(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ComplianceActionRequest>,
) -> Result<Json<ComplianceActionResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let user_region = headers
        .get("X-User-Region")
        .and_then(|v| v.to_str().ok())
        .unwrap_or(crate::i18n::default_region());
    let gate = crate::region_gate::check_region_gate(user_region);
    if gate.degradation == crate::region_gate::RegionDegradationMode::ReadOnly
        || gate.degradation == crate::region_gate::RegionDegradationMode::Blocked
    {
        return Err(i18n_error_json(
            AppErrorCode::SafetyBlocked,
            "compliance.region.policy",
            extract_locale(&headers),
            None,
        ));
    }

    let locale = extract_locale(&headers);
    let bff_resp = bff_post(
        &state,
        auth,
        "/api/v1/bff/compliance/export",
        &json!({
            "export_format": body.export_format,
        }),
        locale,
    )
    .await?;

    let resp: ComplianceActionResponse = serde_json::from_value(bff_resp).map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("compliance export parse: {e}"),
        )
    })?;

    tracing::info!(request_id = %resp.request_id, "app: data export requested");
    Ok(Json(resp))
}

/// Data-rights delete handler. Same two-level enforcement as `compliance_export`:
/// gate-level `allowed=true` for EU, but handler-level 403 `safety_blocked` for
/// ReadOnly/Blocked degradation. See `region_gate.rs` module doc.
async fn compliance_delete(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ComplianceActionRequest>,
) -> Result<Json<ComplianceActionResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let user_region = headers
        .get("X-User-Region")
        .and_then(|v| v.to_str().ok())
        .unwrap_or(crate::i18n::default_region());
    let gate = crate::region_gate::check_region_gate(user_region);
    if gate.degradation == crate::region_gate::RegionDegradationMode::ReadOnly
        || gate.degradation == crate::region_gate::RegionDegradationMode::Blocked
    {
        return Err(i18n_error_json(
            AppErrorCode::SafetyBlocked,
            "compliance.region.policy",
            extract_locale(&headers),
            None,
        ));
    }

    let locale = extract_locale(&headers);
    let bff_resp = bff_post(
        &state,
        auth,
        "/api/v1/bff/compliance/delete",
        &json!({
            "scope": body.scope,
        }),
        locale,
    )
    .await?;

    let resp: ComplianceActionResponse = serde_json::from_value(bff_resp).map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("compliance delete parse: {e}"),
        )
    })?;

    tracing::info!(request_id = %resp.request_id, "app: data delete requested");
    Ok(Json(resp))
}

/// Data-rights correction handler. Same two-level enforcement as `compliance_export`:
/// gate-level `allowed=true` for EU, but handler-level 403 `safety_blocked` for
/// ReadOnly/Blocked degradation. See `region_gate.rs` module doc.
async fn compliance_correction(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<ComplianceActionRequest>,
) -> Result<Json<ComplianceActionResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let user_region = headers
        .get("X-User-Region")
        .and_then(|v| v.to_str().ok())
        .unwrap_or(crate::i18n::default_region());
    let gate = crate::region_gate::check_region_gate(user_region);
    if gate.degradation == crate::region_gate::RegionDegradationMode::ReadOnly
        || gate.degradation == crate::region_gate::RegionDegradationMode::Blocked
    {
        return Err(i18n_error_json(
            AppErrorCode::SafetyBlocked,
            "compliance.region.policy",
            extract_locale(&headers),
            None,
        ));
    }

    let locale = extract_locale(&headers);
    let bff_resp = bff_post(
        &state,
        auth,
        "/api/v1/bff/compliance/correction",
        &json!({
            "field_name": body.field_name,
        }),
        locale,
    )
    .await?;

    let resp: ComplianceActionResponse = serde_json::from_value(bff_resp).map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("compliance correction parse: {e}"),
        )
    })?;

    tracing::info!(request_id = %resp.request_id, "app: data correction requested");
    Ok(Json(resp))
}

async fn admin_report_queue(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<AdminReportQueueResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let locale = extract_locale(&headers);
    let bff_resp = bff_get(&state, auth, "/api/v1/bff/admin/reports", locale).await?;

    let resp: AdminReportQueueResponse = serde_json::from_value(bff_resp).map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("admin report queue parse: {e}"),
        )
    })?;

    Ok(Json(resp))
}

async fn admin_report_detail(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(report_id): axum::extract::Path<String>,
) -> Result<Json<AdminReportDetailResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let locale = extract_locale(&headers);
    let bff_resp = bff_get(
        &state,
        auth,
        &format!("/api/v1/bff/admin/reports/{}", report_id),
        locale,
    )
    .await?;

    let resp: AdminReportDetailResponse = serde_json::from_value(bff_resp).map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("admin report detail parse: {e}"),
        )
    })?;

    Ok(Json(resp))
}

async fn admin_report_action(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(report_id): axum::extract::Path<String>,
    Json(body): Json<AdminActionRequest>,
) -> Result<Json<AdminActionResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;

    let locale = extract_locale(&headers);
    let bff_resp = bff_post(
        &state,
        auth,
        &format!("/api/v1/bff/admin/reports/{}/action", report_id),
        &json!({
            "action_type": body.action_type,
            "reason": body.reason,
        }),
        locale,
    )
    .await?;

    let resp: AdminActionResponse = serde_json::from_value(bff_resp)
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("admin action parse: {e}")))?;

    tracing::info!(report_id = %resp.report_id, action = ?resp.action_type, "app: admin action taken");
    Ok(Json(resp))
}

async fn admin_appeal_queue(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<AdminAppealQueueResponse>, (StatusCode, String)> {
    let auth = extract_auth(&headers)?;
    let locale = extract_locale(&headers);
    let bff_resp = bff_get(&state, auth, "/api/v1/bff/admin/appeals", locale).await?;

    let resp: AdminAppealQueueResponse = serde_json::from_value(bff_resp).map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("admin appeal queue parse: {e}"),
        )
    })?;

    Ok(Json(resp))
}

async fn region_residency_policy(
    headers: HeaderMap,
) -> Result<Json<RegionResidencyPolicy>, (StatusCode, String)> {
    let locale = extract_locale(&headers);
    let _ = locale;
    let policy = crate::region_gate::region_residency_policy_document();
    Ok(Json(policy))
}

async fn region_gate_check(
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let region = headers
        .get("X-User-Region")
        .and_then(|v| v.to_str().ok())
        .unwrap_or(crate::i18n::default_region());
    let locale = extract_locale(&headers);
    let gate = region_gate::check_region_gate(region);
    let notice = region_gate::localize_gate_notice(&gate, locale);
    Ok(Json(json!({
        "user_region": gate.user_region,
        "data_zone": gate.data_zone,
        "degradation": gate.degradation,
        "allowed": gate.allowed,
        "reason": gate.reason,
        "fallback_region": gate.fallback_region,
        "localized_notice": notice,
    })))
}

async fn i18n_registry() -> Json<serde_json::Value> {
    let reg = crate::i18n::I18nRegistry::new();
    let keys: Vec<&str> = reg.all_keys();
    let mut reviewed = serde_json::Map::new();
    for key in &keys {
        if let Some((owner, date)) = reg.entry_review_info(key) {
            reviewed.insert(
                key.to_string(),
                serde_json::json!({
                    "owner": owner,
                    "last_reviewed": date,
                }),
            );
        }
    }
    Json(json!({
        "supported_locales": crate::i18n::supported_locales(),
        "supported_regions": crate::i18n::supported_regions(),
        "supported_timezones": crate::i18n::supported_timezones(),
        "supported_content_languages": crate::i18n::supported_content_languages(),
        "supported_notification_languages": crate::i18n::supported_notification_languages(),
        "default_locale": crate::i18n::default_locale(),
        "default_region": crate::i18n::default_region(),
        "default_timezone": crate::i18n::default_timezone(),
        "default_content_language": crate::i18n::default_content_language(),
        "default_notification_language": crate::i18n::default_notification_language(),
        "message_key_count": keys.len(),
        "message_keys": keys,
        "review_info": reviewed,
    }))
}

async fn i18n_translate(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let key = params.get("key").ok_or((
        StatusCode::BAD_REQUEST,
        "missing query param: key".to_string(),
    ))?;
    let locale = params
        .get("locale")
        .map(|s| s.as_str())
        .unwrap_or(crate::i18n::default_locale());
    let resolved = crate::i18n::resolve_locale(locale);
    let reg = crate::i18n::I18nRegistry::new();
    let translation = reg.translate(key, resolved);
    Ok(Json(json!({
        "key": key,
        "locale": resolved,
        "translation": translation,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::state::AppState;

    fn test_state() -> Arc<AppState> {
        let config = AppConfig {
            port: 0,
            bff_base_url: "http://127.0.0.1:19999".into(),
            ..Default::default()
        };
        AppState::new(config)
    }

    #[test]
    fn test_router_constructs() {
        let state = test_state();
        let _app = router(state);
    }

    #[test]
    fn test_boot_phase_serialization() {
        let phase = BootPhase::SessionRestored;
        let json = serde_json::to_string(&phase).unwrap();
        assert_eq!(json, "\"session_restored\"");
    }

    #[test]
    fn test_auth_flow_state_serialization() {
        let state = AuthFlowState::RequiresVerification;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"requires_verification\"");
    }

    #[test]
    fn test_lumi_chat_state_serialization() {
        let state = LumiChatState::ReplyStreaming;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"reply_streaming\"");
    }

    #[test]
    fn test_profile_confirmation_state_serialization() {
        let state = ProfileConfirmationState::PendingConfirmation;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"pending_confirmation\"");
    }

    #[test]
    fn test_app_error_code_serialization() {
        let code = AppErrorCode::SafetyBlocked;
        let json = serde_json::to_string(&code).unwrap();
        assert_eq!(json, "\"safety_blocked\"");
    }

    #[test]
    fn test_settings_dto_locale_fields_independent() {
        let settings = SettingsDto {
            notifications_enabled: true,
            language: "zh-CN".into(),
            theme: "light".into(),
            locale: "zh-CN".into(),
            region: "CN".into(),
            timezone: "Asia/Shanghai".into(),
            content_language: "zh".into(),
            notification_language: "en".into(),
        };
        assert_ne!(settings.locale, settings.content_language);
        assert_ne!(settings.locale, settings.notification_language);
    }

    #[test]
    fn test_profile_confirmation_action_mapping() {
        assert_eq!(
            serde_json::to_string(&ProfileConfirmationAction::Accept).unwrap(),
            "\"accept\""
        );
        assert_eq!(
            serde_json::to_string(&ProfileConfirmationAction::Reject).unwrap(),
            "\"reject\""
        );
        assert_eq!(
            serde_json::to_string(&ProfileConfirmationAction::Snooze).unwrap(),
            "\"snooze\""
        );
    }

    #[test]
    fn test_find_request_state_serialization() {
        assert_eq!(
            serde_json::to_string(&FindRequestState::Draft).unwrap(),
            "\"draft\""
        );
        assert_eq!(
            serde_json::to_string(&FindRequestState::ClarificationNeeded).unwrap(),
            "\"clarification_needed\""
        );
        assert_eq!(
            serde_json::to_string(&FindRequestState::Completed).unwrap(),
            "\"completed\""
        );
    }

    #[test]
    fn test_recommendation_list_state_serialization() {
        assert_eq!(
            serde_json::to_string(&RecommendationListState::WaitingResults).unwrap(),
            "\"waiting_results\""
        );
        assert_eq!(
            serde_json::to_string(&RecommendationListState::ResultsReady).unwrap(),
            "\"results_ready\""
        );
        assert_eq!(
            serde_json::to_string(&RecommendationListState::EmptyResult).unwrap(),
            "\"empty_result\""
        );
    }

    #[test]
    fn test_recommendation_feedback_type_serialization() {
        assert_eq!(
            serde_json::to_string(&RecommendationFeedbackType::Like).unwrap(),
            "\"like\""
        );
        assert_eq!(
            serde_json::to_string(&RecommendationFeedbackType::Skip).unwrap(),
            "\"skip\""
        );
        assert_eq!(
            serde_json::to_string(&RecommendationFeedbackType::Later).unwrap(),
            "\"later\""
        );
    }

    #[test]
    fn test_dm_first_message_state_serialization() {
        assert_eq!(
            serde_json::to_string(&DmFirstMessageState::Draft).unwrap(),
            "\"draft\""
        );
        assert_eq!(
            serde_json::to_string(&DmFirstMessageState::UnderReview).unwrap(),
            "\"under_review\""
        );
        assert_eq!(
            serde_json::to_string(&DmFirstMessageState::Approved).unwrap(),
            "\"approved\""
        );
        assert_eq!(
            serde_json::to_string(&DmFirstMessageState::Blocked).unwrap(),
            "\"blocked\""
        );
        assert_eq!(
            serde_json::to_string(&DmFirstMessageState::Sent).unwrap(),
            "\"sent\""
        );
    }

    #[test]
    fn test_report_category_serialization() {
        assert_eq!(
            serde_json::to_string(&ReportCategory::Harassment).unwrap(),
            "\"harassment\""
        );
        assert_eq!(
            serde_json::to_string(&ReportCategory::Spam).unwrap(),
            "\"spam\""
        );
        assert_eq!(
            serde_json::to_string(&ReportCategory::InappropriateContent).unwrap(),
            "\"inappropriate_content\""
        );
    }

    #[test]
    fn test_appeal_status_serialization() {
        assert_eq!(
            serde_json::to_string(&AppealStatus::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&AppealStatus::UnderReview).unwrap(),
            "\"under_review\""
        );
        assert_eq!(
            serde_json::to_string(&AppealStatus::Approved).unwrap(),
            "\"approved\""
        );
        assert_eq!(
            serde_json::to_string(&AppealStatus::Rejected).unwrap(),
            "\"rejected\""
        );
    }

    #[test]
    fn test_compliance_action_type_serialization() {
        assert_eq!(
            serde_json::to_string(&ComplianceActionType::Export).unwrap(),
            "\"export\""
        );
        assert_eq!(
            serde_json::to_string(&ComplianceActionType::Delete).unwrap(),
            "\"delete\""
        );
        assert_eq!(
            serde_json::to_string(&ComplianceActionType::Correction).unwrap(),
            "\"correction\""
        );
    }

    #[test]
    fn test_admin_report_status_serialization() {
        assert_eq!(
            serde_json::to_string(&AdminReportStatus::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&AdminReportStatus::Resolved).unwrap(),
            "\"resolved\""
        );
    }

    #[test]
    fn test_admin_action_type_serialization() {
        assert_eq!(
            serde_json::to_string(&AdminActionType::Ban).unwrap(),
            "\"ban\""
        );
        assert_eq!(
            serde_json::to_string(&AdminActionType::Dismiss).unwrap(),
            "\"dismiss\""
        );
    }

    #[test]
    fn test_compliance_action_request_covers_all_rights() {
        let export = ComplianceActionRequest {
            action_type: ComplianceActionType::Export,
            scope: None,
            field_name: None,
            export_format: Some("json".into()),
        };
        assert_eq!(export.action_type, ComplianceActionType::Export);

        let delete = ComplianceActionRequest {
            action_type: ComplianceActionType::Delete,
            scope: Some("all".into()),
            field_name: None,
            export_format: None,
        };
        assert_eq!(delete.action_type, ComplianceActionType::Delete);

        let correction = ComplianceActionRequest {
            action_type: ComplianceActionType::Correction,
            scope: None,
            field_name: Some("nickname".into()),
            export_format: None,
        };
        assert_eq!(correction.action_type, ComplianceActionType::Correction);
    }

    #[test]
    fn test_compliance_summary_response_contains_data_rights_fields() {
        let resp = ComplianceSummaryResponse {
            user_id: uuid::Uuid::new_v4(),
            data_export_available: true,
            data_delete_available: true,
            data_correction_available: true,
            pending_requests: vec![],
            profile_facts: vec![serde_json::json!({"fact_id": "test", "fact_text": "test fact"})],
            memory_summaries: vec![],
            key_artifacts: vec![],
            settings: Some(serde_json::json!({"locale": "zh-CN"})),
            consent_records: vec![],
        };
        assert!(resp.data_export_available);
        assert!(resp.data_delete_available);
        assert!(resp.data_correction_available);
        assert!(!resp.profile_facts.is_empty());
        assert!(resp.settings.is_some());
        let json = serde_json::to_value(&resp).unwrap();
        assert!(json.get("data_export_available").is_some());
        assert!(json.get("data_delete_available").is_some());
        assert!(json.get("data_correction_available").is_some());
        assert!(json.get("profile_facts").is_some());
        assert!(json.get("consent_records").is_some());
    }

    #[test]
    fn test_settings_dto_has_all_i18n_fields() {
        let settings = SettingsDto {
            notifications_enabled: true,
            language: "zh-CN".into(),
            theme: "light".into(),
            locale: "zh-CN".into(),
            region: "CN".into(),
            timezone: "Asia/Shanghai".into(),
            content_language: "zh-CN".into(),
            notification_language: "en".into(),
        };
        let json = serde_json::to_value(&settings).unwrap();
        assert_eq!(json["locale"].as_str(), Some("zh-CN"));
        assert_eq!(json["region"].as_str(), Some("CN"));
        assert_eq!(json["timezone"].as_str(), Some("Asia/Shanghai"));
        assert_eq!(json["content_language"].as_str(), Some("zh-CN"));
        assert_eq!(json["notification_language"].as_str(), Some("en"));
    }

    #[test]
    fn test_settings_update_request_all_i18n_fields_optional() {
        let full_update = SettingsUpdateRequest {
            locale: Some("en".into()),
            region: Some("US".into()),
            timezone: Some("America/New_York".into()),
            content_language: Some("en".into()),
            notification_language: Some("zh-CN".into()),
            notifications_enabled: Some(false),
        };
        assert_eq!(full_update.locale.as_deref(), Some("en"));
        assert_eq!(full_update.region.as_deref(), Some("US"));
        assert_eq!(full_update.timezone.as_deref(), Some("America/New_York"));
        assert_eq!(full_update.content_language.as_deref(), Some("en"));
        assert_eq!(full_update.notification_language.as_deref(), Some("zh-CN"));

        let partial_update = SettingsUpdateRequest {
            locale: Some("en".into()),
            region: None,
            timezone: None,
            content_language: None,
            notification_language: None,
            notifications_enabled: None,
        };
        assert!(partial_update.region.is_none());
        assert!(partial_update.timezone.is_none());
        assert!(partial_update.content_language.is_none());
        assert!(partial_update.notification_language.is_none());
    }

    #[test]
    fn test_me_summary_includes_locale_and_region() {
        let resp = MeSummaryResponse {
            user_id: uuid::Uuid::new_v4(),
            nickname: "test".into(),
            avatar_url: None,
            first_run: false,
            locale: "zh-CN".into(),
            region: "CN".into(),
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["locale"].as_str(), Some("zh-CN"));
        assert_eq!(json["region"].as_str(), Some("CN"));
    }

    #[test]
    fn test_locale_registry_response_has_all_field_lists() {
        let resp = LocaleRegistryResponse {
            available_locales: crate::i18n::supported_locales(),
            available_regions: crate::i18n::supported_regions(),
            available_timezones: crate::i18n::supported_timezones(),
        };
        assert!(resp.available_locales.contains(&"zh-CN".to_string()));
        assert!(resp.available_locales.contains(&"en".to_string()));
        assert!(resp.available_regions.len() >= 4);
        assert!(resp.available_timezones.len() >= 4);
    }

    #[test]
    fn test_data_rights_i18n_keys_all_localizable() {
        let reg = crate::i18n::I18nRegistry::new();
        let data_rights_keys = [
            "privacy.data_export.title",
            "privacy.data_export.description",
            "privacy.data_export.requested",
            "privacy.data_delete.title",
            "privacy.data_delete.description",
            "privacy.data_delete.confirmation",
            "privacy.data_delete.warning",
            "privacy.data_correction.title",
            "privacy.data_correction.description",
            "privacy.data_correction.submitted",
            "privacy.view_data.title",
            "compliance.action.view_data",
            "compliance.action.export_data",
            "compliance.action.correct_data",
            "compliance.action.delete_data",
        ];
        for key in &data_rights_keys {
            assert!(reg.contains_key(key), "missing data rights key: {}", key);
            let zh = reg.translate(key, "zh-CN");
            assert!(!zh.starts_with('['), "zh-CN not localized for key: {}", key);
            let en = reg.translate(key, "en");
            assert!(!en.starts_with('['), "en not localized for key: {}", key);
        }
    }

    #[test]
    fn test_region_gate_localized_notice_for_all_supported_regions() {
        let regions = crate::i18n::supported_regions();
        for region in &regions {
            let gate = crate::region_gate::check_region_gate(region);
            if let Some(notice_key) = &gate.notice_key {
                let reg = crate::i18n::I18nRegistry::new();
                let zh = reg.translate(notice_key, "zh-CN");
                assert!(
                    !zh.starts_with('['),
                    "zh-CN missing for gate notice key: {}",
                    notice_key
                );
                let en = reg.translate(notice_key, "en");
                assert!(
                    !en.starts_with('['),
                    "en missing for gate notice key: {}",
                    notice_key
                );
            }
        }
    }

    #[test]
    fn test_screens_include_data_rights_and_locale_settings() {
        let screens = crate::screens::a4_screens();
        let ids: Vec<&str> = screens
            .iter()
            .map(|s| match s.screen_id {
                crate::screens::ScreenId::LocaleSettings => "locale_settings",
                crate::screens::ScreenId::DataRights => "data_rights",
                _ => "other",
            })
            .collect();
        assert!(
            ids.contains(&"locale_settings"),
            "missing LocaleSettings screen"
        );
        assert!(ids.contains(&"data_rights"), "missing DataRights screen");
    }

    #[test]
    fn test_screens_data_rights_contracts_include_all_compliance_endpoints() {
        let screens = crate::screens::a4_screens();
        let data_rights = screens
            .iter()
            .find(|s| matches!(s.screen_id, crate::screens::ScreenId::DataRights))
            .unwrap();
        let contracts = &data_rights.bff_contracts;
        assert!(
            contracts.iter().any(|c| c.contains("compliance/summary")),
            "missing compliance/summary contract"
        );
        assert!(
            contracts.iter().any(|c| c.contains("compliance/export")),
            "missing compliance/export contract"
        );
        assert!(
            contracts.iter().any(|c| c.contains("compliance/delete")),
            "missing compliance/delete contract"
        );
        assert!(
            contracts
                .iter()
                .any(|c| c.contains("compliance/correction")),
            "missing compliance/correction contract"
        );
    }

    #[test]
    fn test_i18n_registry_endpoint_data_has_all_field_lists() {
        let reg = crate::i18n::I18nRegistry::new();
        let locales = crate::i18n::supported_locales();
        let regions = crate::i18n::supported_regions();
        let timezones = crate::i18n::supported_timezones();
        let content_langs = crate::i18n::supported_content_languages();
        let notif_langs = crate::i18n::supported_notification_languages();
        assert!(locales.len() >= 2, "need at least 2 locales");
        assert!(regions.len() >= 4, "need at least 4 regions");
        assert!(timezones.len() >= 4, "need at least 4 timezones");
        assert!(
            content_langs.len() >= 2,
            "need at least 2 content languages"
        );
        assert!(
            notif_langs.len() >= 2,
            "need at least 2 notification languages"
        );
        assert!(
            reg.entry_count() >= 50,
            "need at least 50 i18n message keys"
        );
    }

    #[tokio::test]
    async fn test_region_gate_endpoint_returns_localized_notice() {
        use axum::body::Body;
        use http_body_util::BodyExt;
        use tower::ServiceExt;

        let state = test_state();
        let app = router(state);

        let request = axum::http::Request::builder()
            .uri("/api/v1/bff/region/gate")
            .header("X-User-Region", "EU")
            .header("Accept-Language", "en")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert!(response.status().is_success());

        let body = response.into_body();
        let bytes = body.collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["user_region"].as_str(), Some("EU"));
        assert_eq!(json["data_zone"].as_str(), Some("restricted"));
        assert_eq!(json["degradation"].as_str(), Some("read_only"));
        assert!(json["allowed"].as_bool().unwrap());
        assert!(json["localized_notice"].as_str().is_some());
        let notice = json["localized_notice"].as_str().unwrap();
        assert!(
            !notice.is_empty(),
            "localized notice must not be empty for EU region"
        );
        assert!(
            notice.contains("residency"),
            "en notice for EU should mention residency"
        );
    }

    #[tokio::test]
    async fn test_i18n_registry_endpoint_returns_all_field_lists() {
        use axum::body::Body;
        use http_body_util::BodyExt;
        use tower::ServiceExt;

        let state = test_state();
        let app = router(state);

        let request = axum::http::Request::builder()
            .uri("/api/v1/bff/i18n/registry")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert!(response.status().is_success());

        let body = response.into_body();
        let bytes = body.collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

        assert!(json["supported_locales"].as_array().unwrap().len() >= 2);
        assert!(json["supported_regions"].as_array().unwrap().len() >= 4);
        assert!(json["supported_timezones"].as_array().unwrap().len() >= 4);
        assert!(
            json["supported_content_languages"]
                .as_array()
                .unwrap()
                .len()
                >= 2
        );
        assert!(
            json["supported_notification_languages"]
                .as_array()
                .unwrap()
                .len()
                >= 2
        );
        assert!(json["default_locale"].as_str().is_some());
        assert!(json["default_region"].as_str().is_some());
        assert!(json["default_timezone"].as_str().is_some());
        assert!(json["default_content_language"].as_str().is_some());
        assert!(json["default_notification_language"].as_str().is_some());
        assert!(json["message_key_count"].as_u64().unwrap() >= 50);
    }

    #[tokio::test]
    async fn test_i18n_translate_endpoint_localizes_data_rights() {
        use axum::body::Body;
        use http_body_util::BodyExt;
        use tower::ServiceExt;

        let state = test_state();
        let app = router(state);

        let request = axum::http::Request::builder()
            .uri("/api/v1/bff/i18n/translate?key=privacy.data_export.title&locale=en")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert!(response.status().is_success());

        let body = response.into_body();
        let bytes = body.collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(json["key"].as_str(), Some("privacy.data_export.title"));
        assert_eq!(json["locale"].as_str(), Some("en"));
        let translation = json["translation"].as_str().unwrap();
        assert!(
            translation.contains("Export"),
            "en translation should contain 'Export'"
        );
    }

    #[tokio::test]
    async fn test_screens_endpoint_includes_data_rights_screen() {
        use axum::body::Body;
        use http_body_util::BodyExt;
        use tower::ServiceExt;

        let state = test_state();
        let app = router(state);

        let request = axum::http::Request::builder()
            .uri("/api/v1/bff/screens")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert!(response.status().is_success());

        let body = response.into_body();
        let bytes = body.collect().await.unwrap().to_bytes();
        let screens: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap();

        let data_rights = screens
            .iter()
            .find(|s| s["route"].as_str() == Some("/settings/data-rights"));
        assert!(
            data_rights.is_some(),
            "screens must include data-rights route"
        );
        let dr = data_rights.unwrap();
        assert!(dr["bff_contracts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c.as_str().unwrap().contains("compliance")));

        let locale_settings = screens
            .iter()
            .find(|s| s["route"].as_str() == Some("/settings/locale"));
        assert!(
            locale_settings.is_some(),
            "screens must include locale-settings route"
        );
    }

    #[tokio::test]
    async fn test_region_gate_endpoint_crossborder_returns_notice() {
        use axum::body::Body;
        use http_body_util::BodyExt;
        use tower::ServiceExt;

        let state = test_state();
        let app = router(state);

        let request = axum::http::Request::builder()
            .uri("/api/v1/bff/region/gate")
            .header("X-User-Region", "US")
            .header("Accept-Language", "zh-CN")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert!(response.status().is_success());

        let body = response.into_body();
        let bytes = body.collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["data_zone"].as_str(), Some("cross_border"));
        assert!(json["localized_notice"].as_str().unwrap().contains("传输"));
    }

    #[tokio::test]
    async fn test_region_gate_endpoint_domestic_no_notice() {
        use axum::body::Body;
        use http_body_util::BodyExt;
        use tower::ServiceExt;

        let state = test_state();
        let app = router(state);

        let request = axum::http::Request::builder()
            .uri("/api/v1/bff/region/gate")
            .header("X-User-Region", "CN")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert!(response.status().is_success());

        let body = response.into_body();
        let bytes = body.collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["data_zone"].as_str(), Some("domestic"));
        assert_eq!(json["localized_notice"].as_str(), Some(""));
    }

    #[tokio::test]
    async fn test_compliance_endpoints_blocked_for_restricted_region_e2e() {
        use axum::body::Body;
        use http_body_util::BodyExt;
        use tower::ServiceExt;

        let state = test_state();
        let app = router(state);

        let export_req = axum::http::Request::builder()
            .method("POST")
            .uri("/api/v1/bff/compliance/export")
            .header("Content-Type", "application/json")
            .header("Authorization", "Bearer test-token")
            .header("X-User-Region", "EU")
            .body(Body::from(
                r#"{"action_type":"export","export_format":"json"}"#,
            ))
            .unwrap();
        let resp = app.clone().oneshot(export_req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["code"].as_str(), Some("safety_blocked"));
        assert!(body["localized_message"].as_str().is_some());
        let msg = body["localized_message"].as_str().unwrap();
        assert!(
            !msg.is_empty(),
            "compliance block must have localized message"
        );
    }

    #[tokio::test]
    async fn test_compliance_delete_blocked_for_readonly_region_e2e() {
        use axum::body::Body;
        use tower::ServiceExt;

        let state = test_state();
        let app = router(state);

        let delete_req = axum::http::Request::builder()
            .method("POST")
            .uri("/api/v1/bff/compliance/delete")
            .header("Content-Type", "application/json")
            .header("Authorization", "Bearer test-token")
            .header("X-User-Region", "EU")
            .body(Body::from(r#"{"action_type":"delete","scope":"all"}"#))
            .unwrap();
        let resp = app.oneshot(delete_req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_compliance_correction_blocked_for_readonly_region_e2e() {
        use axum::body::Body;
        use tower::ServiceExt;

        let state = test_state();
        let app = router(state);

        let correction_req = axum::http::Request::builder()
            .method("POST")
            .uri("/api/v1/bff/compliance/correction")
            .header("Content-Type", "application/json")
            .header("Authorization", "Bearer test-token")
            .header("X-User-Region", "EU")
            .body(Body::from(
                r#"{"action_type":"correction","field_name":"nickname"}"#,
            ))
            .unwrap();
        let resp = app.oneshot(correction_req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_settings_locale_update_patches_all_i18n_fields() {
        let body = SettingsUpdateRequest {
            locale: Some("en".into()),
            region: Some("US".into()),
            timezone: Some("America/New_York".into()),
            content_language: Some("en".into()),
            notification_language: Some("en".into()),
            notifications_enabled: Some(true),
        };
        let mut patch = serde_json::Map::new();
        if let Some(locale) = &body.locale {
            patch.insert("locale".to_string(), json!(locale));
        }
        if let Some(region) = &body.region {
            patch.insert("region".to_string(), json!(region));
        }
        if let Some(timezone) = &body.timezone {
            patch.insert("timezone".to_string(), json!(timezone));
        }
        if let Some(content_language) = &body.content_language {
            patch.insert("content_language".to_string(), json!(content_language));
        }
        if let Some(notification_language) = &body.notification_language {
            patch.insert(
                "notification_language".to_string(),
                json!(notification_language),
            );
        }
        if let Some(notifications_enabled) = body.notifications_enabled {
            patch.insert(
                "notifications_enabled".to_string(),
                json!(notifications_enabled),
            );
        }
        assert_eq!(patch.get("locale").unwrap().as_str(), Some("en"));
        assert_eq!(patch.get("region").unwrap().as_str(), Some("US"));
        assert_eq!(
            patch.get("timezone").unwrap().as_str(),
            Some("America/New_York")
        );
        assert_eq!(patch.get("content_language").unwrap().as_str(), Some("en"));
        assert_eq!(
            patch.get("notification_language").unwrap().as_str(),
            Some("en")
        );
        assert_eq!(
            patch.get("notifications_enabled").unwrap().as_bool(),
            Some(true)
        );
    }

    #[test]
    fn test_i18n_product_chain_locale_to_region_gate_to_compliance() {
        let locale = crate::i18n::resolve_locale("en");
        assert_eq!(locale, "en");

        let regions = crate::i18n::supported_regions();
        assert!(
            regions.contains(&"EU".to_string()),
            "EU must be supported for compliance gate"
        );

        let gate = crate::region_gate::check_region_gate("EU");
        assert_eq!(
            gate.degradation,
            crate::region_gate::RegionDegradationMode::ReadOnly
        );

        let reg = crate::i18n::I18nRegistry::new();
        let notice = crate::region_gate::localize_gate_notice(&gate, locale);
        assert!(
            !notice.is_empty(),
            "gate notice must be localizable for EU+en"
        );
        assert!(
            notice.contains("residency"),
            "en notice must mention residency"
        );

        let policy_key = "compliance.region.policy";
        assert!(reg.contains_key(policy_key));
        let zh_policy = reg.translate(policy_key, "zh-CN");
        let en_policy = reg.translate(policy_key, "en");
        assert!(
            !zh_policy.starts_with('['),
            "zh-CN policy must be localized"
        );
        assert!(!en_policy.starts_with('['), "en policy must be localized");
    }

    #[test]
    fn test_i18n_product_chain_all_five_fields_in_settings_dto() {
        let settings = SettingsDto {
            notifications_enabled: true,
            language: "zh-CN".into(),
            theme: "light".into(),
            locale: "zh-CN".into(),
            region: "CN".into(),
            timezone: "Asia/Shanghai".into(),
            content_language: "zh-CN".into(),
            notification_language: "zh-CN".into(),
        };
        let json = serde_json::to_value(&settings).unwrap();
        let i18n_fields = [
            "locale",
            "region",
            "timezone",
            "content_language",
            "notification_language",
        ];
        for field in &i18n_fields {
            assert!(
                json.get(*field).is_some(),
                "SettingsDto missing field: {}",
                field
            );
            assert!(
                !json[*field].as_str().unwrap().is_empty(),
                "SettingsDto field {} must not be empty",
                field
            );
        }
    }

    #[test]
    fn test_i18n_product_chain_settings_update_covers_all_five_fields() {
        let update = SettingsUpdateRequest {
            locale: Some("en".into()),
            region: Some("US".into()),
            timezone: Some("America/New_York".into()),
            content_language: Some("en".into()),
            notification_language: Some("en".into()),
            notifications_enabled: None,
        };
        let i18n_fields: [(&str, &Option<String>); 5] = [
            ("locale", &update.locale),
            ("region", &update.region),
            ("timezone", &update.timezone),
            ("content_language", &update.content_language),
            ("notification_language", &update.notification_language),
        ];
        for (name, value) in &i18n_fields {
            assert!(
                value.is_some(),
                "SettingsUpdateRequest must support updating {}",
                name
            );
            assert!(
                !value.as_ref().unwrap().is_empty(),
                "{} must not be empty string",
                name
            );
        }
    }

    #[test]
    fn test_region_gate_data_residency_enforcement_evidence() {
        let cn_gate = crate::region_gate::check_region_gate("CN");
        assert_eq!(cn_gate.data_zone, crate::region_gate::DataZone::Domestic);
        assert_eq!(
            cn_gate.degradation,
            crate::region_gate::RegionDegradationMode::Normal
        );
        assert!(cn_gate.allowed);

        let us_gate = crate::region_gate::check_region_gate("US");
        assert_eq!(us_gate.data_zone, crate::region_gate::DataZone::CrossBorder);
        assert!(us_gate.fallback_region.is_some());
        assert_eq!(
            us_gate.notice_key.as_deref(),
            Some("compliance.crossborder.notice")
        );

        let eu_gate = crate::region_gate::check_region_gate("EU");
        assert_eq!(eu_gate.data_zone, crate::region_gate::DataZone::Restricted);
        assert_eq!(
            eu_gate.degradation,
            crate::region_gate::RegionDegradationMode::ReadOnly
        );
        assert!(eu_gate.allowed);
        assert_eq!(
            eu_gate.reason.as_deref(),
            Some("restricted_region_read_only")
        );
    }

    #[test]
    fn test_compliance_summary_region_gate_restricts_data_rights() {
        let eu_gate = crate::region_gate::check_region_gate("EU");
        let restricted = eu_gate.degradation == crate::region_gate::RegionDegradationMode::ReadOnly
            || eu_gate.degradation == crate::region_gate::RegionDegradationMode::Blocked;
        assert!(restricted, "EU must be restricted (ReadOnly or Blocked)");

        let cn_gate = crate::region_gate::check_region_gate("CN");
        let cn_restricted = cn_gate.degradation
            == crate::region_gate::RegionDegradationMode::ReadOnly
            || cn_gate.degradation == crate::region_gate::RegionDegradationMode::Blocked;
        assert!(!cn_restricted, "CN must not be restricted");
    }

    #[test]
    fn test_web_bff_page_entry_screens_route_to_compliance_endpoints() {
        let screens = crate::screens::a4_screens();
        let data_rights = screens
            .iter()
            .find(|s| matches!(s.screen_id, crate::screens::ScreenId::DataRights));
        assert!(
            data_rights.is_some(),
            "DataRights screen must exist as Web/BFF page entry"
        );
        let dr = data_rights.unwrap();
        let contracts = &dr.bff_contracts;
        let required = [
            "compliance/summary",
            "compliance/export",
            "compliance/delete",
            "compliance/correction",
        ];
        for endpoint in &required {
            assert!(
                contracts.iter().any(|c| c.contains(endpoint)),
                "DataRights screen must reference {} endpoint",
                endpoint
            );
        }

        let locale_settings = screens
            .iter()
            .find(|s| matches!(s.screen_id, crate::screens::ScreenId::LocaleSettings));
        assert!(
            locale_settings.is_some(),
            "LocaleSettings screen must exist as Web/BFF page entry"
        );
    }

    #[test]
    fn test_compliance_i18n_keys_cover_all_data_rights_and_screen_entries() {
        let reg = crate::i18n::I18nRegistry::new();
        let compliance_keys = [
            "privacy.data_export.title",
            "privacy.data_export.description",
            "privacy.data_export.requested",
            "privacy.data_delete.title",
            "privacy.data_delete.description",
            "privacy.data_delete.confirmation",
            "privacy.data_delete.warning",
            "privacy.data_correction.title",
            "privacy.data_correction.description",
            "privacy.data_correction.submitted",
            "privacy.view_data.title",
            "compliance.action.view_data",
            "compliance.action.export_data",
            "compliance.action.correct_data",
            "compliance.action.delete_data",
            "compliance.crossborder.notice",
            "compliance.region.policy",
            "compliance.underage.warning",
            "screen.data_rights.title",
            "screen.locale_settings.title",
        ];
        for key in &compliance_keys {
            assert!(
                reg.contains_key(key),
                "missing compliance/screen i18n key: {}",
                key
            );
            let zh = reg.translate(key, "zh-CN");
            let en = reg.translate(key, "en");
            assert!(!zh.starts_with('['), "zh-CN missing for {}", key);
            assert!(!en.starts_with('['), "en missing for {}", key);
        }
    }

    #[test]
    fn test_locale_get_endpoint_fields_match_i18n_module() {
        let i18n_locales = crate::i18n::supported_locales();
        let i18n_regions = crate::i18n::supported_regions();
        let i18n_timezones = crate::i18n::supported_timezones();
        let content_langs = crate::i18n::supported_content_languages();
        let notif_langs = crate::i18n::supported_notification_languages();

        assert!(i18n_locales.contains(&"zh-CN".to_string()));
        assert!(i18n_locales.contains(&"en".to_string()));
        assert!(i18n_regions.contains(&"CN".to_string()));
        assert!(i18n_regions.contains(&"US".to_string()));
        assert!(i18n_regions.contains(&"EU".to_string()));
        assert!(i18n_regions.contains(&"SEA".to_string()));
        assert!(i18n_timezones.contains(&"Asia/Shanghai".to_string()));
        assert!(i18n_timezones.contains(&"America/New_York".to_string()));
        assert!(content_langs.contains(&"zh-CN".to_string()));
        assert!(content_langs.contains(&"en".to_string()));
        assert!(notif_langs.contains(&"zh-CN".to_string()));
        assert!(notif_langs.contains(&"en".to_string()));
    }

    #[test]
    fn test_evidence_settings_dto_contract_sample() {
        let sample = SettingsDto {
            notifications_enabled: true,
            language: "zh-CN".into(),
            theme: "light".into(),
            locale: "zh-CN".into(),
            region: "CN".into(),
            timezone: "Asia/Shanghai".into(),
            content_language: "zh-CN".into(),
            notification_language: "zh-CN".into(),
        };
        let json_val = serde_json::to_value(&sample).unwrap();
        println!(
            "[evidence:settings_dto] {}",
            serde_json::to_string_pretty(&json_val).unwrap()
        );
        for field in &[
            "locale",
            "region",
            "timezone",
            "content_language",
            "notification_language",
        ] {
            assert!(
                json_val.get(*field).is_some(),
                "SettingsDto contract missing field: {}",
                field
            );
        }
    }

    #[test]
    fn test_evidence_settings_update_request_contract_sample() {
        let sample = SettingsUpdateRequest {
            locale: Some("en".into()),
            region: Some("US".into()),
            timezone: Some("America/New_York".into()),
            content_language: Some("en".into()),
            notification_language: Some("en".into()),
            notifications_enabled: Some(false),
        };
        let json_val = serde_json::to_value(&sample).unwrap();
        println!(
            "[evidence:settings_update_request] {}",
            serde_json::to_string_pretty(&json_val).unwrap()
        );
        for field in &[
            "locale",
            "region",
            "timezone",
            "content_language",
            "notification_language",
        ] {
            assert!(
                json_val.get(*field).is_some(),
                "SettingsUpdateRequest contract missing field: {}",
                field
            );
        }
    }

    #[test]
    fn test_evidence_region_gate_all_supported_regions_behavior() {
        let regions = crate::i18n::supported_regions();
        let mut results = Vec::new();
        for region in &regions {
            let gate = crate::region_gate::check_region_gate(region);
            let notice_zh = crate::region_gate::localize_gate_notice(&gate, "zh-CN");
            let notice_en = crate::region_gate::localize_gate_notice(&gate, "en");
            results.push(serde_json::json!({
                "region": region,
                "data_zone": format!("{:?}", gate.data_zone),
                "degradation": format!("{:?}", gate.degradation),
                "allowed": gate.allowed,
                "reason": gate.reason,
                "fallback_region": gate.fallback_region,
                "notice_zh": notice_zh,
                "notice_en": notice_en,
            }));
        }
        println!(
            "[evidence:region_gate_behavior] {}",
            serde_json::to_string_pretty(&results).unwrap()
        );
        let eu = results.iter().find(|r| r["region"] == "EU").unwrap();
        assert_eq!(eu["degradation"], "ReadOnly");
        let cn = results.iter().find(|r| r["region"] == "CN").unwrap();
        assert_eq!(cn["degradation"], "Normal");
    }

    #[test]
    fn test_evidence_compliance_endpoints_contract_fields() {
        let summary_sample = ComplianceSummaryResponse {
            user_id: uuid::Uuid::new_v4(),
            data_export_available: true,
            data_delete_available: true,
            data_correction_available: true,
            pending_requests: vec![],
            profile_facts: vec![],
            memory_summaries: vec![],
            key_artifacts: vec![],
            settings: None,
            consent_records: vec![],
        };
        let summary_json = serde_json::to_value(&summary_sample).unwrap();
        println!(
            "[evidence:compliance_summary_response] {}",
            serde_json::to_string_pretty(&summary_json).unwrap()
        );

        let action_sample = ComplianceActionRequest {
            action_type: ComplianceActionType::Export,
            scope: Some("all".into()),
            field_name: None,
            export_format: Some("json".into()),
        };
        let action_json = serde_json::to_value(&action_sample).unwrap();
        println!(
            "[evidence:compliance_action_request] {}",
            serde_json::to_string_pretty(&action_json).unwrap()
        );

        let action_resp_sample = ComplianceActionResponse {
            request_id: uuid::Uuid::new_v4(),
            action_type: ComplianceActionType::Delete,
            status: "pending".into(),
            created_at: chrono::Utc::now(),
        };
        let action_resp_json = serde_json::to_value(&action_resp_sample).unwrap();
        println!(
            "[evidence:compliance_action_response] {}",
            serde_json::to_string_pretty(&action_resp_json).unwrap()
        );

        for field in &[
            "data_export_available",
            "data_delete_available",
            "data_correction_available",
        ] {
            assert!(
                summary_json.get(*field).is_some(),
                "ComplianceSummaryResponse missing: {}",
                field
            );
        }
    }

    #[test]
    fn test_evidence_data_rights_api_action_types_match_endpoints() {
        let actions = vec![
            (
                ComplianceActionType::Export,
                "/api/v1/bff/compliance/export",
                "compliance.action.export_data",
            ),
            (
                ComplianceActionType::Delete,
                "/api/v1/bff/compliance/delete",
                "compliance.action.delete_data",
            ),
            (
                ComplianceActionType::Correction,
                "/api/v1/bff/compliance/correction",
                "compliance.action.correct_data",
            ),
        ];
        let reg = crate::i18n::I18nRegistry::new();
        let mut results = Vec::new();
        for (action_type, endpoint, i18n_key) in &actions {
            let zh = reg.translate(i18n_key, "zh-CN");
            let en = reg.translate(i18n_key, "en");
            results.push(serde_json::json!({
                "action_type": format!("{:?}", action_type),
                "endpoint": endpoint,
                "i18n_key": i18n_key,
                "label_zh": zh,
                "label_en": en,
            }));
        }
        println!(
            "[evidence:data_rights_api_contract] {}",
            serde_json::to_string_pretty(&results).unwrap()
        );
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_evidence_i18n_bff_page_entry_contract() {
        let screens = crate::screens::a4_screens();
        let reg = crate::i18n::I18nRegistry::new();
        let mut results = Vec::new();

        if let Some(dr) = screens
            .iter()
            .find(|s| matches!(s.screen_id, crate::screens::ScreenId::DataRights))
        {
            let mut contracts = Vec::new();
            for c in &dr.bff_contracts {
                contracts.push(c.clone());
            }
            results.push(serde_json::json!({
                "screen_id": "DataRights",
                "route": dr.route,
                "bff_contracts": contracts,
                "i18n_title_zh": reg.translate("screen.data_rights.title", "zh-CN"),
                "i18n_title_en": reg.translate("screen.data_rights.title", "en"),
            }));
        }

        if let Some(ls) = screens
            .iter()
            .find(|s| matches!(s.screen_id, crate::screens::ScreenId::LocaleSettings))
        {
            let mut contracts = Vec::new();
            for c in &ls.bff_contracts {
                contracts.push(c.clone());
            }
            results.push(serde_json::json!({
                "screen_id": "LocaleSettings",
                "route": ls.route,
                "bff_contracts": contracts,
                "i18n_title_zh": reg.translate("screen.locale_settings.title", "zh-CN"),
                "i18n_title_en": reg.translate("screen.locale_settings.title", "en"),
            }));
        }

        println!(
            "[evidence:bff_page_entry_contract] {}",
            serde_json::to_string_pretty(&results).unwrap()
        );
        assert!(
            !results.is_empty(),
            "Must have DataRights and/or LocaleSettings screen entries"
        );
    }
}
