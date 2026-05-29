use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::StatusCode,
    routing::post,
    Json, Router,
};
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

#[derive(Clone)]
struct MockState;

async fn deepseek_chat_completion(
    State(_state): State<MockState>,
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    assert_eq!(payload["model"], "deepseek-v4-flash");
    assert_eq!(payload["thinking"]["type"], "disabled");
    assert_eq!(payload["messages"][0]["role"], "system");
    assert_eq!(payload["messages"][1]["role"], "user");
    Json(serde_json::json!({
        "id": "chatcmpl-test",
        "model": "deepseek-v4-flash",
        "choices": [
            {
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "DeepSeek says hello"
                }
            }
        ]
    }))
}

async fn start_mock_server() -> (String, tokio::task::JoinHandle<()>) {
    let app = Router::new()
        .route("/chat/completions", post(deepseek_chat_completion))
        .with_state(MockState);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}"), handle)
}

fn test_state(base_url: String) -> Arc<GatewayState> {
    Arc::new(GatewayState {
        config: Config {
            port: 0,
            internal_shared_secret: "test-internal-secret-at-least-32-chars!!".to_string(),
            env_mode: "dev".to_string(),
            internal_bind_addr: "127.0.0.1".to_string(),
            deepseek_base_url: base_url,
            deepseek_api_key: Some("test-deepseek-key".to_string()),
            deepseek_model: "deepseek-v4-flash".to_string(),
            deepseek_timeout_ms: 3_000,
            deepseek_thinking_type: "disabled".to_string(),
        },
        http_client: reqwest::Client::new(),
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
async fn invoke_uses_deepseek_provider_when_api_key_is_configured() {
    let (base_url, server) = start_mock_server().await;
    let app = router(test_state(base_url));

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
                "payload": {
                    "messages": [
                        { "role": "user", "content": "你好" }
                    ],
                    "context": {
                        "system_prompt": "你是 Lumi",
                        "memory_context": "用户偏好简洁回答"
                    }
                }
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["model_id"], "deepseek-v4-flash");
    assert_eq!(json["output_text"], "DeepSeek says hello");
    assert_eq!(json["degraded"], false);

    server.abort();
}
