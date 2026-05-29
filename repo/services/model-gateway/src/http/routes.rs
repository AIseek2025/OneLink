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

    let trace_suffix = request
        .trace_id
        .as_deref()
        .map(|value| format!(" trace_id={value}"))
        .unwrap_or_default();
    let response_text = match capability {
        "chat.respond" => {
            let context_preview = request
                .payload
                .get("context")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            let lumi_greeting = state
                .terminology
                .translate("lumi.greeting", &locale)
                .unwrap_or("Hello");
            format!(
                "{} | context={}{}",
                lumi_greeting, context_preview, trace_suffix
            )
        }
        other => format!(
            "model-gateway skeleton handled capability={other} locale={}{}",
            locale, trace_suffix
        ),
    };

    if let Some(cb) = state.circuit_breakers.get(capability) {
        cb.record_success().await;
    }

    state.cache.put(ckey, response_text.clone()).await;

    let resp = InvokeResponse {
        capability_name: request.capability_name.clone(),
        model_id: "mock-model-v1".to_string(),
        output_text: response_text,
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
