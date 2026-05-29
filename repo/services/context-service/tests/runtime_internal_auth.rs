use axum::body::Body;
use axum::http::StatusCode;
use context_service::app_state::ContextAppState;
use context_service::config::Config;
use context_service::http::routes::router;
use context_service::policy::PolicyConfigStore;
use context_service::store::MemoryBackend;
use onelink_internal_auth::INTERNAL_TOKEN_HEADER;
use tower::ServiceExt;

fn test_config() -> Config {
    Config {
        port: 0,
        database_url: None,
        default_reply_style: "brief".to_string(),
        profile_service_base_url: "http://127.0.0.1:8082".to_string(),
        ai_chat_service_base_url: "http://127.0.0.1:8085".to_string(),
        internal_shared_secret: "test-internal-secret-at-least-32-chars!!".to_string(),
        env_mode: "dev".to_string(),
        internal_bind_addr: "127.0.0.1".to_string(),
    }
}

fn test_app() -> axum::Router {
    let config = test_config();
    let policy = PolicyConfigStore::default();
    let store = MemoryBackend::in_memory();
    let state = ContextAppState::new(policy, config, store);
    router(state)
}

#[tokio::test]
async fn internal_build_context_rejects_missing_token() {
    let app = test_app();

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/internal/context/build")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "user_id": "u1",
                "agent_id": "a1",
                "conversation_id": "c1",
                "input": "hello",
                "task_type": "chat",
                "max_tokens": 8000,
                "memory_limit": 20,
                "summary_limit": 3
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "internal route must reject request without x-internal-token"
    );
}

#[tokio::test]
async fn internal_build_context_rejects_wrong_token() {
    let app = test_app();

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/internal/context/build")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, "wrong-secret-value")
        .body(Body::from(
            serde_json::json!({
                "user_id": "u1",
                "agent_id": "a1",
                "conversation_id": "c1",
                "input": "hello",
                "task_type": "chat",
                "max_tokens": 8000,
                "memory_limit": 20,
                "summary_limit": 3
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "internal route must reject request with wrong x-internal-token"
    );
}

#[tokio::test]
async fn internal_receive_event_rejects_missing_token() {
    let app = test_app();

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/internal/events/receive")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "event_name": "test.event.v1",
                "producer": "test",
                "event_id": "e1",
                "trace_id": "t1",
                "payload": {}
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "internal events/receive must reject request without x-internal-token"
    );
}

#[tokio::test]
async fn internal_forgetting_decide_rejects_missing_token() {
    let app = test_app();

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/internal/memory/forgetting/decide")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "user_id": "u1",
                "target_type": "artifact",
                "target_id": "m1",
                "decision": "retain",
                "reason_codes": [],
                "policy_version": "v1"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "internal forgetting/decide must reject request without x-internal-token"
    );
}

#[tokio::test]
async fn internal_observability_asmr_lite_rejects_missing_token() {
    let app = test_app();

    let request = axum::http::Request::builder()
        .uri("/internal/observability/asmr-lite")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "internal asmr-lite must reject request without x-internal-token"
    );
}

#[tokio::test]
async fn internal_memory_search_rejects_missing_token() {
    let app = test_app();

    let request = axum::http::Request::builder()
        .uri("/internal/memory/search?user_id=u1&query=hello")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "internal memory/search must reject request without x-internal-token"
    );
}

#[tokio::test]
async fn internal_memory_write_rejects_missing_token() {
    let app = test_app();

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/internal/memory/write")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "user_id": "u1",
                "raw_text": "I like coffee"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "internal memory/write must reject request without x-internal-token"
    );
}

#[tokio::test]
async fn internal_session_checkpoint_rejects_missing_token() {
    let app = test_app();

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/internal/session/checkpoint")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "agent_id": "a1",
                "user_id": "u1",
                "schema_version": 1,
                "runtime_state_blob": {},
                "policy_versions": {}
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "internal session/checkpoint must reject request without x-internal-token"
    );
}

#[tokio::test]
async fn internal_build_context_accepts_correct_token() {
    let app = test_app();

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/internal/context/build")
        .header("content-type", "application/json")
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::from(
            serde_json::json!({
                "user_id": "u1",
                "agent_id": "a1",
                "conversation_id": "c1",
                "input": "hello",
                "task_type": "chat",
                "max_tokens": 8000,
                "memory_limit": 20,
                "summary_limit": 3
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "internal route must accept request with correct x-internal-token"
    );
}
