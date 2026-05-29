use ai_chat_service::config::Config;
use ai_chat_service::http::routes::router;
use axum::body::Body;
use axum::http::StatusCode;
use onelink_internal_auth::INTERNAL_TOKEN_HEADER;
use tower::ServiceExt;

fn test_config() -> Config {
    Config {
        port: 0,
        identity_service_base_url: "http://127.0.0.1:8081".to_string(),
        context_service_base_url: "http://127.0.0.1:8089".to_string(),
        model_gateway_base_url: "http://127.0.0.1:8090".to_string(),
        internal_shared_secret: "test-internal-secret-at-least-32-chars!!".to_string(),
        env_mode: "dev".to_string(),
        database_url: None,
        internal_bind_addr: "127.0.0.1".to_string(),
    }
}

#[tokio::test]
async fn internal_get_message_rejects_missing_token() {
    let config = test_config();
    let app = router(config, None);

    let request = axum::http::Request::builder()
        .uri("/internal/chat/conversations/conv-1/messages/msg-1")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "internal /internal/chat/ route must reject request without x-internal-token header"
    );
}

#[tokio::test]
async fn internal_get_message_rejects_wrong_token() {
    let config = test_config();
    let app = router(config, None);

    let request = axum::http::Request::builder()
        .uri("/internal/chat/conversations/conv-1/messages/msg-1")
        .header(INTERNAL_TOKEN_HEADER, "wrong-secret-value-not-matching")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "internal /internal/chat/ route must reject request with wrong x-internal-token value"
    );
}

#[tokio::test]
async fn internal_observability_relay_rejects_missing_token() {
    let config = test_config();
    let app = router(config, None);

    let request = axum::http::Request::builder()
        .uri("/internal/observability/chat-relay")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "internal /internal/observability/ route must reject request without x-internal-token header"
    );
}

#[tokio::test]
async fn internal_observability_relay_rejects_wrong_token() {
    let config = test_config();
    let app = router(config, None);

    let request = axum::http::Request::builder()
        .uri("/internal/observability/chat-relay")
        .header(INTERNAL_TOKEN_HEADER, "wrong-secret-value-not-matching")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "internal /internal/observability/ route must reject request with wrong x-internal-token value"
    );
}

#[tokio::test]
async fn internal_observability_relay_accepts_correct_token() {
    let config = test_config();
    let app = router(config, None);

    let request = axum::http::Request::builder()
        .uri("/internal/observability/chat-relay")
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "internal /internal/observability/ route must accept request with correct x-internal-token"
    );
}
