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

const TEST_SECRET: &str = "test-internal-secret-at-least-32-chars!!";

#[tokio::test]
async fn screen_message_clean_content_allowed() {
    let config = test_config();
    let state = SafetyState::new(config, None);
    let app = router(state);

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/safety/screen-message")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "sender_user_id": "u-1",
                "recipient_user_id": "u-2",
                "content": "你好，很高兴认识你！",
                "is_first_message": true
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["verdict"], "allow");
}

#[tokio::test]
async fn screen_message_suspicious_content_flagged() {
    let config = test_config();
    let state = SafetyState::new(config, None);
    let app = router(state);

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/safety/screen-message")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "sender_user_id": "u-1",
                "recipient_user_id": "u-2",
                "content": "加我telegram，投资加密货币赚大钱",
                "is_first_message": true
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let verdict = json["verdict"].as_str().unwrap();
    assert!(
        verdict == "flag" || verdict == "reject",
        "suspicious content should be flagged or rejected, got: {verdict}"
    );
}

#[tokio::test]
async fn screen_message_blocked_sender_rejected() {
    let config = test_config();
    let state = SafetyState::new(config, None);
    let app = router(state);

    let block_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/safety/blocks")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "user_id": "u-recipient",
                "blocked_user_id": "u-sender"
            })
            .to_string(),
        ))
        .unwrap();

    let _ = app.clone().oneshot(block_req).await.unwrap();

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/safety/screen-message")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "sender_user_id": "u-sender",
                "recipient_user_id": "u-recipient",
                "content": "你好",
                "is_first_message": false
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["verdict"], "reject");
    assert_eq!(json["reason"], "sender_blocked_by_recipient");
}

#[tokio::test]
async fn create_report_success() {
    let config = test_config();
    let state = SafetyState::new(config, None);
    let app = router(state);

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/safety/reports")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "reporter_user_id": "u-reporter",
                "reported_user_id": "u-bad-actor",
                "reason": "harassment",
                "description": "骚扰行为"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["status"], "submitted");
    assert!(!json["report_ticket_id"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn create_report_self_report_rejected() {
    let config = test_config();
    let state = SafetyState::new(config, None);
    let app = router(state);

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/safety/reports")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "reporter_user_id": "u-same",
                "reported_user_id": "u-same",
                "reason": "spam"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn get_report_after_create() {
    let config = test_config();
    let state = SafetyState::new(config, None);
    let app = router(state);

    let create_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/safety/reports")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "reporter_user_id": "u-r1",
                "reported_user_id": "u-r2",
                "reason": "spam"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(create_req).await.unwrap();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let ticket_id = json["report_ticket_id"].as_str().unwrap();

    let get_req = axum::http::Request::builder()
        .uri(format!("/api/v1/safety/reports/{ticket_id}"))
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(get_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["report_ticket_id"], ticket_id);
    assert_eq!(json["reason"], "spam");
}

#[tokio::test]
async fn create_block_and_list() {
    let config = test_config();
    let state = SafetyState::new(config, None);
    let app = router(state);

    let block_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/safety/blocks")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "user_id": "u-blocker",
                "blocked_user_id": "u-blocked"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(block_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let list_req = axum::http::Request::builder()
        .uri("/api/v1/safety/blocks?user_id=u-blocker")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(list_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let blocked = json["blocked_users"].as_array().unwrap();
    assert!(blocked.iter().any(|b| b["blocked_user_id"] == "u-blocked"));
}

#[tokio::test]
async fn create_block_duplicate_rejected() {
    let config = test_config();
    let state = SafetyState::new(config, None);
    let app = router(state);

    let block_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/safety/blocks")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "user_id": "u-dup-blocker",
                "blocked_user_id": "u-dup-blocked"
            })
            .to_string(),
        ))
        .unwrap();

    let _ = app.clone().oneshot(block_req).await.unwrap();

    let dup_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/safety/blocks")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "user_id": "u-dup-blocker",
                "blocked_user_id": "u-dup-blocked"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(dup_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn delete_block_removes_from_list() {
    let config = test_config();
    let state = SafetyState::new(config, None);
    let app = router(state);

    let block_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/safety/blocks")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "user_id": "u-unblocker",
                "blocked_user_id": "u-unblocked"
            })
            .to_string(),
        ))
        .unwrap();

    let _ = app.clone().oneshot(block_req).await.unwrap();

    let del_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/safety/blocks/unblock")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "user_id": "u-unblocker",
                "blocked_user_id": "u-unblocked"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(del_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let list_req = axum::http::Request::builder()
        .uri("/api/v1/safety/blocks?user_id=u-unblocker")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(list_req).await.unwrap();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let blocked = json["blocked_users"].as_array().unwrap();
    assert!(!blocked
        .iter()
        .any(|b| b["blocked_user_id"] == "u-unblocked"));
}

#[tokio::test]
async fn dm_first_message_review_blocked_sender() {
    let config = test_config();
    let state = SafetyState::new(config, None);
    let app = router(state);

    let block_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/safety/blocks")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "user_id": "u-dm-recipient",
                "blocked_user_id": "u-dm-sender"
            })
            .to_string(),
        ))
        .unwrap();

    let _ = app.clone().oneshot(block_req).await.unwrap();

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/safety/dm-first-message-review")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "sender_user_id": "u-dm-sender",
                "recipient_user_id": "u-dm-recipient",
                "message_content": "你好"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["allowed"], false);
    assert_eq!(json["reason"], "sender_blocked_by_recipient");
}

#[tokio::test]
async fn dm_first_message_review_clean_allowed() {
    let config = test_config();
    let state = SafetyState::new(config, None);
    let app = router(state);

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/safety/dm-first-message-review")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "sender_user_id": "u-clean-sender",
                "recipient_user_id": "u-clean-recipient",
                "message_content": "你好，很高兴认识你！"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["allowed"], true);
}

#[tokio::test]
async fn dm_first_message_review_high_risk_rejected() {
    let config = test_config();
    let state = SafetyState::new(config, None);
    let app = router(state);

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/safety/dm-first-message-review")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "sender_user_id": "u-risk-sender",
                "recipient_user_id": "u-risk-recipient",
                "message_content": "wire transfer crypto investment send money http:// telegram whatsapp"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["allowed"], false);
}

#[tokio::test]
async fn create_appeal_after_report() {
    let config = test_config();
    let state = SafetyState::new(config, None);
    let app = router(state);

    let create_report = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/safety/reports")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "reporter_user_id": "u-ap-reporter",
                "reported_user_id": "u-ap-reported",
                "reason": "spam"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(create_report).await.unwrap();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let ticket_id = json["report_ticket_id"].as_str().unwrap();

    let appeal_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/safety/appeals")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "report_ticket_id": ticket_id,
                "appellant_user_id": "u-ap-reported",
                "reason": "误判"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(appeal_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["status"], "submitted");
}

#[tokio::test]
async fn get_moderation_shows_reports() {
    let config = test_config();
    let state = SafetyState::new(config, None);
    let app = router(state);

    let create_report = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/safety/reports")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "reporter_user_id": "u-mod-reporter",
                "reported_user_id": "u-mod-reported",
                "reason": "harassment"
            })
            .to_string(),
        ))
        .unwrap();

    let _ = app.clone().oneshot(create_report).await.unwrap();

    let mod_req = axum::http::Request::builder()
        .uri("/api/v1/safety/me/moderation?user_id=u-mod-reported")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(mod_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["user_id"], "u-mod-reported");
    let entries = json["entries"].as_array().unwrap();
    assert!(!entries.is_empty());
}

#[tokio::test]
async fn report_action_ban_creates_risk_flag() {
    let config = test_config();
    let state = SafetyState::new(config, None);
    let app = router(state);

    let create_report = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/safety/reports")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "reporter_user_id": "u-ban-reporter",
                "reported_user_id": "u-ban-reported",
                "reason": "safety_concern"
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(create_report).await.unwrap();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let ticket_id = json["report_ticket_id"].as_str().unwrap();

    let action_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/safety/reports/actions")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "report_ticket_id": ticket_id,
                "action_type": "permanent_ban",
                "actor_user_id": "admin-1"
            })
            .to_string(),
        ))
        .unwrap();

    let action_resp = app.clone().oneshot(action_req).await.unwrap();
    assert_eq!(action_resp.status(), StatusCode::OK);

    let risk_req = axum::http::Request::builder()
        .uri("/api/v1/safety/risk-flags/u-ban-reported")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::empty())
        .unwrap();

    let risk_resp = app.oneshot(risk_req).await.unwrap();
    assert_eq!(risk_resp.status(), StatusCode::OK);
    let risk_bytes = risk_resp.into_body().collect().await.unwrap().to_bytes();
    let risk_json: serde_json::Value = serde_json::from_slice(&risk_bytes).unwrap();
    let flags = risk_json["flags"].as_array().unwrap();
    assert!(flags.iter().any(|f| f["severity"] == "critical"));
}

#[tokio::test]
async fn create_risk_flag_and_retrieve() {
    let config = test_config();
    let state = SafetyState::new(config, None);
    let app = router(state);

    let flag_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/safety/risk-flags")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "user_id": "u-flag-target",
                "flag_type": "pattern_violation",
                "source": "automated",
                "description": "suspicious pattern detected",
                "severity": "high"
            })
            .to_string(),
        ))
        .unwrap();

    let flag_resp = app.clone().oneshot(flag_req).await.unwrap();
    assert_eq!(flag_resp.status(), StatusCode::OK);
    let flag_bytes = flag_resp.into_body().collect().await.unwrap().to_bytes();
    let flag_json: serde_json::Value = serde_json::from_slice(&flag_bytes).unwrap();
    assert_eq!(flag_json["severity"], "high");

    let get_req = axum::http::Request::builder()
        .uri("/api/v1/safety/risk-flags/u-flag-target")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::empty())
        .unwrap();

    let get_resp = app.oneshot(get_req).await.unwrap();
    assert_eq!(get_resp.status(), StatusCode::OK);
    let get_bytes = get_resp.into_body().collect().await.unwrap().to_bytes();
    let get_json: serde_json::Value = serde_json::from_slice(&get_bytes).unwrap();
    let flags = get_json["flags"].as_array().unwrap();
    assert!(flags.iter().any(|f| f["flag_type"] == "pattern_violation"));
}
