use axum::body::Body;
use axum::http::StatusCode;
use onelink_internal_auth::INTERNAL_TOKEN_HEADER;
use profile_service::config::Config;
use profile_service::http::routes::{router, ProfileState};
use tower::ServiceExt;

fn test_config() -> Config {
    Config {
        port: 0,
        identity_service_base_url: "http://127.0.0.1:8081".to_string(),
        context_service_base_url: "http://127.0.0.1:8089".to_string(),
        internal_shared_secret: "test-internal-secret-at-least-32-chars!!".to_string(),
        env_mode: "dev".to_string(),
        database_url: None,
        internal_bind_addr: "127.0.0.1".to_string(),
    }
}

fn test_app() -> axum::Router {
    let config = test_config();
    let state = ProfileState::new(config, None);
    router(state)
}

#[tokio::test]
async fn internal_events_receive_rejects_missing_token() {
    let app = test_app();

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/internal/events/receive")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "event_name": "profile.memory_projection.requested.v1",
                "producer": "context-service",
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
async fn internal_events_receive_rejects_wrong_token() {
    let app = test_app();

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/internal/events/receive")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, "wrong-secret-value")
        .body(Body::from(
            serde_json::json!({
                "event_name": "profile.memory_projection.requested.v1",
                "producer": "context-service",
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
        "internal events/receive must reject request with wrong x-internal-token"
    );
}

#[tokio::test]
async fn internal_events_receive_accepts_correct_token() {
    let app = test_app();

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/internal/events/receive")
        .header("content-type", "application/json")
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::from(
            serde_json::json!({
                "event_id": "e1",
                "event_name": "some.other.event.v1",
                "event_version": "v1",
                "occurred_at": "2026-01-01T00:00:00Z",
                "producer": "context-service",
                "trace_id": "t1",
                "payload": {}
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::ACCEPTED,
        "internal events/receive must accept request with correct x-internal-token"
    );
}

#[tokio::test]
async fn public_profile_me_does_not_require_internal_token() {
    let app = test_app();

    let request_without_internal = axum::http::Request::builder()
        .uri("/api/v1/profile/me")
        .header("authorization", "Bearer some-session-token")
        .body(Body::empty())
        .unwrap();

    let request_with_internal = axum::http::Request::builder()
        .uri("/api/v1/profile/me")
        .header("authorization", "Bearer some-session-token")
        .header(INTERNAL_TOKEN_HEADER, "wrong-secret-value")
        .body(Body::empty())
        .unwrap();

    let response_without_internal = app.clone().oneshot(request_without_internal).await.unwrap();
    let response_with_internal = app.oneshot(request_with_internal).await.unwrap();

    assert_eq!(
        response_without_internal.status(),
        response_with_internal.status(),
        "public /api/v1/ routes should be gated by Bearer auth only; x-internal-token must not affect them"
    );
}
