use std::sync::Arc;

use axum::body::Body;
use axum::http::StatusCode;
use http_body_util::BodyExt;
use identity_service::config::Config;
use identity_service::http::routes::{router, IdentityState};
use onelink_internal_auth::INTERNAL_TOKEN_HEADER;
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
    let state = Arc::new(IdentityState::new(config));
    let app = router(state);

    let request = axum::http::Request::builder()
        .uri("/internal/identity/health-detail")
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
    let state = Arc::new(IdentityState::new(config));
    let app = router(state);

    let request = axum::http::Request::builder()
        .uri("/internal/identity/health-detail")
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
    let state = Arc::new(IdentityState::new(config));
    let app = router(state);

    let request = axum::http::Request::builder()
        .uri("/internal/identity/health-detail")
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
    assert_eq!(json["uses_argon2"], true);
}

#[tokio::test]
async fn public_routes_do_not_require_internal_token() {
    let config = test_config();
    let state = Arc::new(IdentityState::new(config));
    let app = router(state);

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/identity/register")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "provider": "email",
                "email": "runtime-test@example.com",
                "password": "test-password-123",
                "primary_region": "CN",
                "primary_language": "zh"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_ne!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "public /api/v1/ routes must not require x-internal-token"
    );
}

#[tokio::test]
async fn register_login_me_full_flow_in_memory() {
    let config = test_config();
    let state = Arc::new(IdentityState::new(config));
    let app = router(Arc::clone(&state));

    let register_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/identity/register")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "provider": "email",
                "email": "smoke-user@example.com",
                "password": "smoke-password-123",
                "primary_region": "CN",
                "primary_language": "zh"
            })
            .to_string(),
        ))
        .unwrap();

    let reg_response = app.oneshot(register_req).await.unwrap();
    assert_eq!(reg_response.status(), StatusCode::OK);

    let reg_bytes = reg_response.into_body().collect().await.unwrap().to_bytes();
    let reg_json: serde_json::Value = serde_json::from_slice(&reg_bytes).unwrap();
    let token = reg_json["session"]["token"].as_str().unwrap();

    let me_req = axum::http::Request::builder()
        .uri("/api/v1/identity/me")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let app2 = router(state);
    let me_response = app2.oneshot(me_req).await.unwrap();
    assert_eq!(
        me_response.status(),
        StatusCode::OK,
        "me endpoint must return user profile with valid session token"
    );

    let me_bytes = me_response.into_body().collect().await.unwrap().to_bytes();
    let me_json: serde_json::Value = serde_json::from_slice(&me_bytes).unwrap();
    assert_eq!(me_json["status"], "active");
    assert_eq!(me_json["primary_region"], "CN");
}
