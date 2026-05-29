use std::sync::Arc;

use axum::body::Body;
use axum::http::StatusCode;
use http_body_util::BodyExt;
use model_gateway::budget::TokenBudgetTracker;
use model_gateway::bulkhead::CapabilityBulkheads;
use model_gateway::cache::ResponseCache;
use model_gateway::circuit_breaker::CircuitBreakerRegistry;
use model_gateway::compliance::CompliancePolicy;
use model_gateway::config::Config;
use model_gateway::cost_metrics::CostMetrics;
use model_gateway::fallback::FallbackConfig;
use model_gateway::http::routes::{router, GatewayState};
use model_gateway::locale::TerminologyRegistry;
use model_gateway::region::RegionRegistry;
use onelink_internal_auth::INTERNAL_TOKEN_HEADER;
use tower::ServiceExt;

fn test_config() -> Config {
    Config {
        port: 0,
        internal_shared_secret: "test-internal-secret-at-least-32-chars!!".to_string(),
        env_mode: "dev".to_string(),
        internal_bind_addr: "127.0.0.1".to_string(),
    }
}

fn test_state() -> Arc<GatewayState> {
    Arc::new(GatewayState {
        config: test_config(),
        bulkheads: CapabilityBulkheads::with_defaults(),
        circuit_breakers: CircuitBreakerRegistry::with_default_capabilities(),
        budget_tracker: TokenBudgetTracker::with_default_capabilities(),
        cache: ResponseCache::with_defaults(),
        cost_metrics: CostMetrics::new(),
        fallback_config: FallbackConfig::default(),
        terminology: TerminologyRegistry::new(),
        compliance: CompliancePolicy::new(),
        regions: RegionRegistry::new(),
    })
}

#[tokio::test]
async fn internal_invoke_rejects_missing_token() {
    let state = test_state();
    let app = router(state);

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/internal/v1/invoke")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "capability_name": "chat.respond",
                "payload": {}
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "internal invoke must reject request without x-internal-token"
    );
}

#[tokio::test]
async fn internal_invoke_rejects_wrong_token() {
    let state = test_state();
    let app = router(state);

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/internal/v1/invoke")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, "wrong-secret-value")
        .body(Body::from(
            serde_json::json!({
                "capability_name": "chat.respond",
                "payload": {}
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "internal invoke must reject request with wrong x-internal-token"
    );
}

#[tokio::test]
async fn internal_invoke_accepts_correct_token() {
    let state = test_state();
    let app = router(state);

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/internal/v1/invoke")
        .header("content-type", "application/json")
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::from(
            serde_json::json!({
                "capability_name": "chat.respond",
                "payload": { "messages": [] }
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "internal invoke must accept request with correct x-internal-token"
    );

    let body = response.into_body();
    let bytes = body.collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["capability_name"], "chat.respond");
    assert_eq!(json["model_id"], "mock-model-v1");
    assert_eq!(json["degraded"], false);
}

#[tokio::test]
async fn invoke_returns_fallback_when_budget_exceeded() {
    let budget_tracker = TokenBudgetTracker::from_configs(vec![(
        "chat.respond".to_string(),
        model_gateway::budget::TokenBudgetConfig {
            max_tokens_per_request: 100,
            daily_budget_tokens: 10,
        },
    )]);
    let state = Arc::new(GatewayState {
        config: test_config(),
        bulkheads: CapabilityBulkheads::with_defaults(),
        circuit_breakers: CircuitBreakerRegistry::with_default_capabilities(),
        budget_tracker,
        cache: ResponseCache::with_defaults(),
        cost_metrics: CostMetrics::new(),
        fallback_config: FallbackConfig::default(),
        terminology: TerminologyRegistry::new(),
        compliance: CompliancePolicy::new(),
        regions: RegionRegistry::new(),
    });
    let app = router(state);

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/internal/v1/invoke")
        .header("content-type", "application/json")
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::from(
            serde_json::json!({
                "capability_name": "chat.respond",
                "estimated_tokens": 20,
                "payload": {}
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body();
    let bytes = body.collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["degraded"], true);
    assert_eq!(json["fallback_reason"], "token_budget_exceeded");
}

#[tokio::test]
async fn invoke_returns_fallback_when_provider_error_is_simulated() {
    let state = test_state();
    let app = router(state);

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/internal/v1/invoke")
        .header("content-type", "application/json")
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::from(
            serde_json::json!({
                "capability_name": "chat.respond",
                "simulate_provider_error": "timeout",
                "payload": {}
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body();
    let bytes = body.collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["degraded"], true);
    assert_eq!(json["fallback_reason"], "provider_error");
    assert!(json["output_text"].as_str().unwrap().contains("timeout"));
}

#[tokio::test]
async fn repeated_simulated_provider_errors_open_circuit_breaker() {
    let state = test_state();
    let app = router(state.clone());

    for _ in 0..5 {
        let request = axum::http::Request::builder()
            .method("POST")
            .uri("/internal/v1/invoke")
            .header("content-type", "application/json")
            .header(
                INTERNAL_TOKEN_HEADER,
                "test-internal-secret-at-least-32-chars!!",
            )
            .body(Body::from(
                serde_json::json!({
                    "capability_name": "chat.respond",
                    "simulate_provider_error": "provider_5xx",
                    "payload": {}
                })
                .to_string(),
            ))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    let circuit = state
        .circuit_breakers
        .get("chat.respond")
        .expect("default chat.respond circuit breaker");
    assert_eq!(
        circuit.state().await,
        model_gateway::circuit_breaker::CircuitState::Open
    );
}

#[tokio::test]
async fn capacity_status_requires_auth() {
    let state = test_state();
    let app = router(state);

    let request = axum::http::Request::builder()
        .method("GET")
        .uri("/internal/v1/capacity/status")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn invoke_with_locale_hint_en() {
    let state = test_state();
    let app = router(state);

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/internal/v1/invoke")
        .header("content-type", "application/json")
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::from(
            serde_json::json!({
                "capability_name": "chat.respond",
                "locale_hint": "en",
                "payload": { "context": {} }
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body();
    let bytes = body.collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let text = json["output_text"].as_str().unwrap();
    assert!(
        text.contains("Hello! I'm Lumi") || text.contains("Hello"),
        "invoke with locale_hint=en should use English terminology"
    );
}

#[tokio::test]
async fn invoke_with_locale_hint_zh_cn() {
    let state = test_state();
    let app = router(state);

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/internal/v1/invoke")
        .header("content-type", "application/json")
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::from(
            serde_json::json!({
                "capability_name": "chat.respond",
                "locale_hint": "zh-CN",
                "payload": { "context": {} }
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body();
    let bytes = body.collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let text = json["output_text"].as_str().unwrap();
    assert!(
        text.contains("你好") || text.contains("Lumi"),
        "invoke with locale_hint=zh-CN should use Chinese terminology"
    );
}

#[tokio::test]
async fn i18n_policy_endpoint_requires_auth() {
    let state = test_state();
    let app = router(state);

    let request = axum::http::Request::builder()
        .method("GET")
        .uri("/internal/v1/i18n/policy")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn i18n_policy_returns_supported_locales() {
    let state = test_state();
    let app = router(state);

    let request = axum::http::Request::builder()
        .method("GET")
        .uri("/internal/v1/i18n/policy")
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body();
    let bytes = body.collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["default_locale"], "zh-CN");
    assert!(json["supported_locales"].is_array());
    assert!(json["safety_human_review_required"].is_boolean());
}

#[tokio::test]
async fn compliance_policy_endpoint_requires_auth() {
    let state = test_state();
    let app = router(state);

    let request = axum::http::Request::builder()
        .method("GET")
        .uri("/internal/v1/compliance/policy")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn compliance_policy_returns_rights_and_categories() {
    let state = test_state();
    let app = router(state);

    let request = axum::http::Request::builder()
        .method("GET")
        .uri("/internal/v1/compliance/policy")
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body();
    let bytes = body.collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(json["user_rights"].is_array());
    assert!(json["safety_categories"].is_array());
    assert_eq!(json["unauthorized_find_person_blocked"], true);
    assert_eq!(json["unauthorized_data_collection_blocked"], true);
}

#[tokio::test]
async fn residency_decision_endpoint_requires_auth() {
    let state = test_state();
    let app = router(state);

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/internal/v1/residency/decision")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({ "user_region": "cn-north" }).to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn residency_decision_returns_data_regions() {
    let state = test_state();
    let app = router(state);

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/internal/v1/residency/decision")
        .header("content-type", "application/json")
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::from(
            serde_json::json!({ "user_region": "cn-north" }).to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body();
    let bytes = body.collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["user_region"], "cn-north");
    assert_eq!(json["data_region"], "cn-north");
    assert_eq!(json["zones_isolated"], true);
}

#[tokio::test]
async fn capacity_status_returns_metrics() {
    let state = test_state();
    let app = router(state);

    let request = axum::http::Request::builder()
        .method("GET")
        .uri("/internal/v1/capacity/status")
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body();
    let bytes = body.collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(json["bulkheads"].is_object());
    assert!(json["circuit_breakers"].is_array());
    assert!(json["token_budgets"].is_array());
    assert!(json["cache"].is_object());
    assert!(json["cost_metrics"].is_object());
}
