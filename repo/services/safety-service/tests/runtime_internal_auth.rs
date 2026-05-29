use axum::body::Body;
use axum::http::StatusCode;
use http_body_util::BodyExt;
use onelink_internal_auth::INTERNAL_TOKEN_HEADER;
use safety_service::config::Config;
use safety_service::http::routes::{router, SafetyState};
use tower::ServiceExt;

fn test_config() -> Config {
    Config {
        port: 0,
        internal_shared_secret: "test-internal-secret-at-least-32-chars!!".to_string(),
        env_mode: "dev".to_string(),
        database_url: None,
        internal_bind_addr: "127.0.0.1".to_string(),
    }
}

#[tokio::test]
async fn internal_health_detail_rejects_missing_token() {
    let config = test_config();
    let state = SafetyState::new(config, None);
    let app = router(state);

    let request = axum::http::Request::builder()
        .uri("/internal/safety/health-detail")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "internal route must reject request without x-internal-token header"
    );
}

#[tokio::test]
async fn internal_health_detail_rejects_wrong_token() {
    let config = test_config();
    let state = SafetyState::new(config, None);
    let app = router(state);

    let request = axum::http::Request::builder()
        .uri("/internal/safety/health-detail")
        .header(INTERNAL_TOKEN_HEADER, "wrong-secret-value-not-matching")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "internal route must reject request with wrong x-internal-token value"
    );
}

#[tokio::test]
async fn internal_health_detail_accepts_correct_token() {
    let config = test_config();
    let state = SafetyState::new(config, None);
    let app = router(state);

    let request = axum::http::Request::builder()
        .uri("/internal/safety/health-detail")
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
        "internal route must accept request with correct x-internal-token"
    );

    let body = response.into_body();
    let bytes = body.collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["env_mode"], "dev");
    assert_eq!(json["backend"], "in-memory");
}

#[tokio::test]
async fn public_routes_require_internal_token() {
    let config = test_config();
    let state = SafetyState::new(config, None);
    let app = router(state);

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/safety/reports")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "reporter_user_id": "u-1",
                "reported_user_id": "u-2",
                "reason": "spam"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "safety-service /api/v1/ routes require x-internal-token"
    );
}
