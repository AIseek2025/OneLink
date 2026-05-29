//! Minimal internal invoke route with capacity controls.

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use onelink_internal_auth::verify_internal_token;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::budget::TokenBudgetTracker;
use crate::bulkhead::CapabilityBulkheads;
use crate::cache::{cache_key, ResponseCache};
use crate::circuit_breaker::CircuitBreakerRegistry;
use crate::compliance::{ComplianceAction, CompliancePolicy, UserRight};
use crate::config::Config;
use crate::cost_metrics::{CostEventKind, CostMetrics};
use crate::fallback::{FallbackConfig, FallbackResponse};
use crate::locale::{Locale, TerminologyRegistry};
use crate::region::{compute_residency, RegionRegistry};

pub struct GatewayState {
    pub config: Config,
    pub http_client: reqwest::Client,
    pub bulkheads: CapabilityBulkheads,
    pub circuit_breakers: CircuitBreakerRegistry,
    pub budget_tracker: TokenBudgetTracker,
    pub cache: ResponseCache,
    pub cost_metrics: CostMetrics,
    pub fallback_config: FallbackConfig,
    pub terminology: TerminologyRegistry,
    pub compliance: CompliancePolicy,
    pub regions: RegionRegistry,
}

pub fn router(state: Arc<GatewayState>) -> Router {
    Router::new()
        .route(
            "/api/v1/placeholder",
            get(|| async { "model-gateway skeleton" }),
        )
        .route("/internal/v1/invoke", post(invoke))
        .route("/internal/v1/capacity/status", get(capacity_status))
        .route("/internal/v1/i18n/policy", get(i18n_policy))
        .route("/internal/v1/i18n/translate", post(i18n_translate))
        .route("/internal/v1/compliance/policy", get(compliance_policy))
        .route("/internal/v1/residency/decision", post(residency_decision))
        .with_state(state)
}

async fn invoke(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, StatusCode> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)?;

    let request: InvokeRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(err) => {
            tracing::warn!(error = %err, "model-gateway /internal/v1/invoke: invalid request body");
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    let capability = request.capability_name.as_str();
    let estimated_tokens = request.estimated_tokens.unwrap_or(0);
    let locale = request
        .locale_hint
        .as_deref()
        .map(Locale::resolve_or_default)
        .unwrap_or_else(|| Locale::parse("zh-CN").unwrap());

    if state.bulkheads.is_at_capacity(capability) {
        state.cost_metrics.record(CostEventKind::BulkheadRejected);
        let fallback = FallbackResponse::bulkhead_rejected(capability, &state.fallback_config);
        state.cost_metrics.record(CostEventKind::FallbackUsed);
        return Ok(Json(serde_json::to_value(fallback).unwrap()));
    }

    let cb_available = if let Some(cb) = state.circuit_breakers.get(capability) {
        cb.is_available().await
    } else {
        false
    };

    if !cb_available {
        state.cost_metrics.record(CostEventKind::CircuitBreakerOpen);
        let fallback = FallbackResponse::circuit_open(capability, &state.fallback_config);
        state.cost_metrics.record(CostEventKind::FallbackUsed);
        return Ok(Json(serde_json::to_value(fallback).unwrap()));
    }

    if estimated_tokens > 0 {
        if state
            .budget_tracker
            .try_consume(capability, estimated_tokens)
            .is_err()
        {
            state.cost_metrics.record(CostEventKind::BudgetExceeded);
            let fallback = FallbackResponse::budget_exceeded(capability, &state.fallback_config);
            state.cost_metrics.record(CostEventKind::FallbackUsed);
            return Ok(Json(serde_json::to_value(fallback).unwrap()));
        }
        state.cost_metrics.record_tokens(estimated_tokens);
    }

    let ckey = cache_key(capability, &request.payload);
    if let Some(cached) = state.cache.get(&ckey).await {
        state.cost_metrics.record(CostEventKind::CacheHit);
        let resp = InvokeResponse {
            capability_name: request.capability_name,
            model_id: "cached".to_string(),
            output_text: cached,
            degraded: false,
            fallback_reason: None,
        };
        return Ok(Json(serde_json::to_value(resp).unwrap()));
    }

    let _guard = state.bulkheads.try_acquire(capability);

    state.cost_metrics.record(CostEventKind::ModelInvocation);

    if let Some(simulated_error) = request.simulate_provider_error.as_deref() {
        if let Some(cb) = state.circuit_breakers.get(capability) {
            cb.record_failure().await;
        }
        let fallback =
            FallbackResponse::provider_error(capability, &state.fallback_config, simulated_error);
        state.cost_metrics.record(CostEventKind::FallbackUsed);
        return Ok(Json(serde_json::to_value(fallback).unwrap()));
    }

    let provider_response = match capability {
        "chat.respond" if state.config.deepseek_api_key.is_some() => {
            match invoke_deepseek(&state, &request, &locale).await {
                Ok(response) => response,
                Err(error) => {
                    tracing::warn!(capability, error = %error, "deepseek invoke failed");
                    if let Some(cb) = state.circuit_breakers.get(capability) {
                        cb.record_failure().await;
                    }
                    let fallback =
                        FallbackResponse::provider_error(capability, &state.fallback_config, &error);
                    state.cost_metrics.record(CostEventKind::FallbackUsed);
                    return Ok(Json(serde_json::to_value(fallback).unwrap()));
                }
            }
        }
        "chat.respond" => {
            let trace_suffix = request
                .trace_id
                .as_deref()
                .map(|value| format!(" trace_id={value}"))
                .unwrap_or_default();
            let context_preview = request
                .payload
                .get("context")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            let lumi_greeting = state
                .terminology
                .translate("lumi.greeting", &locale)
                .unwrap_or("Hello");
            ProviderResponse {
                model_id: "mock-model-v1".to_string(),
                output_text: format!(
                    "{} | context={}{}",
                    lumi_greeting, context_preview, trace_suffix
                ),
            }
        }
        other => {
            let trace_suffix = request
                .trace_id
                .as_deref()
                .map(|value| format!(" trace_id={value}"))
                .unwrap_or_default();
            ProviderResponse {
                model_id: "mock-model-v1".to_string(),
                output_text: format!(
                    "model-gateway skeleton handled capability={other} locale={}{}",
                    locale, trace_suffix
                ),
            }
        }
    };

    if let Some(cb) = state.circuit_breakers.get(capability) {
        cb.record_success().await;
    }

    state.cache.put(ckey, provider_response.output_text.clone()).await;

    let resp = InvokeResponse {
        capability_name: request.capability_name.clone(),
        model_id: provider_response.model_id,
        output_text: provider_response.output_text,
        degraded: false,
        fallback_reason: None,
    };
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

async fn capacity_status(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
) -> Result<Json<CapacityStatusResponse>, StatusCode> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)?;

    let bulkhead_counts = state.bulkheads.active_counts();
    let circuit_status = state.circuit_breakers.status_snapshot().await;
    let budget_usage = state.budget_tracker.usage_snapshot();
    let cache_stats = state.cache.stats().await;
    let cost_snapshot = state.cost_metrics.snapshot();

    Ok(Json(CapacityStatusResponse {
        bulkheads: bulkhead_counts,
        circuit_breakers: circuit_status,
        token_budgets: budget_usage,
        cache: cache_stats,
        cost_metrics: cost_snapshot,
    }))
}

#[derive(Debug, Deserialize)]
struct InvokeRequest {
    capability_name: String,
    trace_id: Option<String>,
    estimated_tokens: Option<u64>,
    locale_hint: Option<String>,
    simulate_provider_error: Option<String>,
    #[serde(default)]
    payload: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct InvokeResponse {
    capability_name: String,
    model_id: String,
    output_text: String,
    degraded: bool,
    fallback_reason: Option<String>,
}

#[derive(Debug)]
struct ProviderResponse {
    model_id: String,
    output_text: String,
}

#[derive(Debug, Serialize)]
struct DeepSeekChatRequest {
    model: String,
    messages: Vec<DeepSeekMessage>,
    stream: bool,
    thinking: DeepSeekThinking,
}

#[derive(Debug, Serialize)]
struct DeepSeekThinking {
    #[serde(rename = "type")]
    thinking_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct DeepSeekMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct DeepSeekChatResponse {
    model: Option<String>,
    choices: Vec<DeepSeekChoice>,
}

#[derive(Debug, Deserialize)]
struct DeepSeekChoice {
    message: DeepSeekAssistantMessage,
}

#[derive(Debug, Deserialize)]
struct DeepSeekAssistantMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeepSeekErrorEnvelope {
    error: DeepSeekErrorBody,
}

#[derive(Debug, Deserialize)]
struct DeepSeekErrorBody {
    message: Option<String>,
}

async fn invoke_deepseek(
    state: &GatewayState,
    request: &InvokeRequest,
    locale: &Locale,
) -> Result<ProviderResponse, String> {
    let api_key = state
        .config
        .deepseek_api_key
        .as_deref()
        .ok_or_else(|| "missing DEEPSEEK_API_KEY".to_string())?;
    let endpoint = format!(
        "{}/chat/completions",
        state.config.deepseek_base_url.trim_end_matches('/')
    );
    let body = DeepSeekChatRequest {
        model: state.config.deepseek_model.clone(),
        messages: build_deepseek_messages(request, locale),
        stream: false,
        thinking: DeepSeekThinking {
            thinking_type: normalize_thinking_type(&state.config.deepseek_thinking_type),
        },
    };
    let response = state
        .http_client
        .post(endpoint)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|error| format!("deepseek request failed: {error}"))?;
    let status = response.status();
    let response_body = response
        .text()
        .await
        .map_err(|error| format!("deepseek body read failed: {error}"))?;
    if !status.is_success() {
        let message = serde_json::from_str::<DeepSeekErrorEnvelope>(&response_body)
            .ok()
            .and_then(|decoded| decoded.error.message)
            .unwrap_or(response_body);
        return Err(format!("deepseek status {status}: {message}"));
    }
    let decoded: DeepSeekChatResponse = serde_json::from_str(&response_body)
        .map_err(|error| format!("deepseek decode failed: {error}"))?;
    let content = decoded
        .choices
        .into_iter()
        .find_map(|choice| choice.message.content)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "deepseek returned empty content".to_string())?;
    Ok(ProviderResponse {
        model_id: decoded.model.unwrap_or_else(|| state.config.deepseek_model.clone()),
        output_text: content,
    })
}

fn build_deepseek_messages(request: &InvokeRequest, locale: &Locale) -> Vec<DeepSeekMessage> {
    let mut messages = Vec::new();
    let mut system_sections = vec![match locale.tag() {
        tag if tag.starts_with("zh") => "你是 OneLink 的 AI 助手 Lumi，请基于提供的上下文给出自然、准确、简洁的回复。".to_string(),
        _ => "You are Lumi, the AI assistant for OneLink. Use the supplied context to produce a natural, accurate, concise reply.".to_string(),
    }];
    if let Some(context) = request.payload.get("context") {
        push_context_section(&mut system_sections, "System Prompt", context, "system_prompt");
        push_context_section(&mut system_sections, "User Context", context, "user_context");
        push_context_section(&mut system_sections, "Memory Context", context, "memory_context");
        push_context_section(&mut system_sections, "Task Context", context, "task_context");
    }
    if !system_sections.is_empty() {
        messages.push(DeepSeekMessage {
            role: "system".to_string(),
            content: system_sections.join("\n\n"),
        });
    }
    if let Some(items) = request.payload.get("messages").and_then(|value| value.as_array()) {
        for item in items {
            let role = item
                .get("role")
                .and_then(|value| value.as_str())
                .unwrap_or("user");
            let content = item
                .get("content")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .trim();
            if !content.is_empty() {
                messages.push(DeepSeekMessage {
                    role: role.to_string(),
                    content: content.to_string(),
                });
            }
        }
    }
    if messages.len() == 1 {
        messages.push(DeepSeekMessage {
            role: "user".to_string(),
            content: match locale.tag() {
                tag if tag.starts_with("zh") => "请根据当前上下文继续帮助用户。".to_string(),
                _ => "Please continue helping the user based on the current context.".to_string(),
            },
        });
    }
    messages
}

fn push_context_section(
    sections: &mut Vec<String>,
    label: &str,
    context: &serde_json::Value,
    key: &str,
) {
    if let Some(value) = context.get(key).and_then(|value| value.as_str()) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            sections.push(format!("{label}:\n{trimmed}"));
        }
    }
}

fn normalize_thinking_type(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        "enabled" => "enabled".to_string(),
        _ => "disabled".to_string(),
    }
}

#[derive(Debug, Serialize)]
struct CapacityStatusResponse {
    bulkheads: std::collections::HashMap<String, u64>,
    circuit_breakers: Vec<crate::circuit_breaker::CircuitBreakerStatus>,
    token_budgets: Vec<crate::budget::TokenUsage>,
    cache: crate::cache::CacheStats,
    cost_metrics: crate::cost_metrics::CostMetricsSnapshot,
}

async fn i18n_policy(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)?;

    let supported: Vec<&str> = crate::locale::supported_locales().to_vec();
    let default = crate::locale::default_locale();
    let policy = serde_json::json!({
        "supported_locales": supported,
        "default_locale": default,
        "safety_human_review_required": state.compliance.requires_human_review_for_safety(),
    });
    Ok(Json(policy))
}

#[derive(Debug, Deserialize)]
struct TranslateRequest {
    key: String,
    locale: String,
}

async fn i18n_translate(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, StatusCode> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)?;

    let req: TranslateRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    let locale = Locale::resolve_or_default(&req.locale);
    let text = state.terminology.translate(&req.key, &locale).unwrap_or("");
    Ok(Json(serde_json::json!({
        "key": req.key,
        "locale": locale.tag(),
        "text": text,
    })))
}

async fn compliance_policy(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)?;

    let rights: Vec<&str> = state
        .compliance
        .supported_rights()
        .iter()
        .map(|r| match r {
            UserRight::ViewData => "view_data",
            UserRight::ExportData => "export_data",
            UserRight::CorrectData => "correct_data",
            UserRight::DeleteData => "delete_data",
        })
        .collect();
    let categories: Vec<&str> = state
        .compliance
        .safety_categories()
        .iter()
        .map(|c| c.label())
        .collect();
    Ok(Json(serde_json::json!({
        "user_rights": rights,
        "safety_categories": categories,
        "unauthorized_find_person_blocked": !state.compliance.check_find_person(false).eq(&ComplianceAction::AllowWithAudit),
        "unauthorized_data_collection_blocked": !state.compliance.check_data_collection(false).eq(&ComplianceAction::AllowWithAudit),
    })))
}

#[derive(Debug, Deserialize)]
struct ResidencyRequest {
    user_region: String,
}

async fn residency_decision(
    State(state): State<Arc<GatewayState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<serde_json::Value>, StatusCode> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)?;

    let req: ResidencyRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    let decision = compute_residency(&state.regions, &req.user_region);
    Ok(Json(serde_json::json!({
        "user_region": decision.user_region,
        "data_region": decision.data_region,
        "model_call_region": decision.model_call_region,
        "log_storage_region": decision.log_storage_region,
        "zones_isolated": decision.zones_isolated,
        "transfer_required": decision.transfer_required,
        "transfer_permitted": decision.transfer_permitted,
    })))
}
