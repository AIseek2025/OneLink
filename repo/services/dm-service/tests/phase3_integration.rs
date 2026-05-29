use axum::body::Body;
use axum::http::StatusCode;
use dm_service::config::Config;
use dm_service::http::routes::{router, DmState};
use http_body_util::BodyExt;
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

const TEST_SECRET: &str = "test-internal-secret-at-least-32-chars!!";

#[tokio::test]
async fn create_thread_and_send_message() {
    let config = test_config();
    let state = DmState::new(config, None);
    let app = router(state);

    let create_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/dm/threads")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "initiator_user_id": "u-init",
                "recipient_user_id": "u-recv"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(create_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let thread_id = json["thread_id"].as_str().unwrap();
    assert!(json["created"].as_bool().unwrap());

    let msg_req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/api/v1/dm/threads/{thread_id}/messages"))
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "sender_user_id": "u-init",
                "content": "你好，很高兴认识你！"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(msg_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(!json["message_id"].as_str().unwrap().is_empty());
    assert_eq!(json["thread_id"], thread_id);
}

#[tokio::test]
async fn create_thread_duplicate_returns_existing() {
    let config = test_config();
    let state = DmState::new(config, None);
    let app = router(state);

    let create1 = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/dm/threads")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "initiator_user_id": "u-dup-a",
                "recipient_user_id": "u-dup-b"
            })
            .to_string(),
        ))
        .unwrap();

    let resp1 = app.clone().oneshot(create1).await.unwrap();
    let bytes1 = resp1.into_body().collect().await.unwrap().to_bytes();
    let json1: serde_json::Value = serde_json::from_slice(&bytes1).unwrap();
    let tid1 = json1["thread_id"].as_str().unwrap();

    let create2 = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/dm/threads")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "initiator_user_id": "u-dup-a",
                "recipient_user_id": "u-dup-b"
            })
            .to_string(),
        ))
        .unwrap();

    let resp2 = app.oneshot(create2).await.unwrap();
    let bytes2 = resp2.into_body().collect().await.unwrap().to_bytes();
    let json2: serde_json::Value = serde_json::from_slice(&bytes2).unwrap();
    assert_eq!(json2["thread_id"].as_str().unwrap(), tid1);
    assert!(!json2["created"].as_bool().unwrap());
}

#[tokio::test]
async fn send_message_non_participant_forbidden() {
    let config = test_config();
    let state = DmState::new(config, None);
    let app = router(state);

    let create_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/dm/threads")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "initiator_user_id": "u-p-a",
                "recipient_user_id": "u-p-b"
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(create_req).await.unwrap();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let thread_id = json["thread_id"].as_str().unwrap();

    let msg_req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/api/v1/dm/threads/{thread_id}/messages"))
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "sender_user_id": "u-outsider",
                "content": "hi"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(msg_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn list_threads_returns_user_threads() {
    let config = test_config();
    let state = DmState::new(config, None);
    let app = router(state);

    let create_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/dm/threads")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "initiator_user_id": "u-list-a",
                "recipient_user_id": "u-list-b"
            })
            .to_string(),
        ))
        .unwrap();

    let _ = app.clone().oneshot(create_req).await.unwrap();

    let list_req = axum::http::Request::builder()
        .uri("/api/v1/dm/threads?user_id=u-list-a&limit=10")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(list_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let threads = json["threads"].as_array().unwrap();
    assert!(!threads.is_empty());
    assert!(threads
        .iter()
        .any(|t| t["participant_a_id"] == "u-list-a" || t["participant_b_id"] == "u-list-a"));
}

#[tokio::test]
async fn mark_read_updates_messages() {
    let config = test_config();
    let state = DmState::new(config, None);
    let app = router(state);

    let create_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/dm/threads")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "initiator_user_id": "u-read-a",
                "recipient_user_id": "u-read-b"
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(create_req).await.unwrap();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let thread_id = json["thread_id"].as_str().unwrap();

    let msg_req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/api/v1/dm/threads/{thread_id}/messages"))
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "sender_user_id": "u-read-a",
                "content": "hello"
            })
            .to_string(),
        ))
        .unwrap();

    let _ = app.clone().oneshot(msg_req).await.unwrap();

    let mark_req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/api/v1/dm/threads/{thread_id}/read"))
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "reader_user_id": "u-read-b"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(mark_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(json["marked_read"].as_u64().unwrap() >= 1);
}

#[tokio::test]
async fn archive_thread_updates_status() {
    let config = test_config();
    let state = DmState::new(config, None);
    let app = router(state);

    let create_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/dm/threads")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "initiator_user_id": "u-arch-a",
                "recipient_user_id": "u-arch-b"
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(create_req).await.unwrap();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let thread_id = json["thread_id"].as_str().unwrap();

    let archive_req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/api/v1/dm/threads/{thread_id}/archive"))
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(archive_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["status"], "archived");
}

#[tokio::test]
async fn get_thread_detail_returns_message_count() {
    let config = test_config();
    let state = DmState::new(config, None);
    let app = router(state);

    let create_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/dm/threads")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "initiator_user_id": "u-detail-a",
                "recipient_user_id": "u-detail-b"
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(create_req).await.unwrap();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let thread_id = json["thread_id"].as_str().unwrap();

    let detail_req = axum::http::Request::builder()
        .uri(format!("/api/v1/dm/threads/{thread_id}"))
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(detail_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["thread_id"], thread_id);
    assert_eq!(json["message_count"].as_u64().unwrap(), 0);
}

#[tokio::test]
async fn screening_log_returns_entries() {
    let config = test_config();
    let state = DmState::new(config, None);
    let app = router(state);

    let create_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/dm/threads")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "initiator_user_id": "u-scr-a",
                "recipient_user_id": "u-scr-b"
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(create_req).await.unwrap();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let thread_id = json["thread_id"].as_str().unwrap();

    let log_req = axum::http::Request::builder()
        .uri(format!("/api/v1/dm/threads/{thread_id}/screening-log"))
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(log_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["thread_id"], thread_id);
    assert!(json["screenings"].is_array());
}

#[tokio::test]
async fn create_thread_self_rejected() {
    let config = test_config();
    let state = DmState::new(config, None);
    let app = router(state);

    let create_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/dm/threads")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "initiator_user_id": "u-self",
                "recipient_user_id": "u-self"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(create_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn send_empty_message_rejected() {
    let config = test_config();
    let state = DmState::new(config, None);
    let app = router(state);

    let create_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/dm/threads")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "initiator_user_id": "u-empty-a",
                "recipient_user_id": "u-empty-b"
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(create_req).await.unwrap();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let thread_id = json["thread_id"].as_str().unwrap();

    let msg_req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/api/v1/dm/threads/{thread_id}/messages"))
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "sender_user_id": "u-empty-a",
                "content": "   "
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(msg_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn first_message_clean_passes_safety_screening() {
    let config = test_config();
    let state = DmState::new(config, None);
    let app = router(state);

    let create_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/dm/threads")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "initiator_user_id": "u-clean-a",
                "recipient_user_id": "u-clean-b"
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(create_req).await.unwrap();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let thread_id = json["thread_id"].as_str().unwrap();

    let msg_req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/api/v1/dm/threads/{thread_id}/messages"))
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "sender_user_id": "u-clean-a",
                "content": "你好，很高兴认识你！"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(msg_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let log_req = axum::http::Request::builder()
        .uri(format!("/api/v1/dm/threads/{thread_id}/screening-log"))
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::empty())
        .unwrap();

    let log_resp = app.oneshot(log_req).await.unwrap();
    let log_bytes = log_resp.into_body().collect().await.unwrap().to_bytes();
    let log_json: serde_json::Value = serde_json::from_slice(&log_bytes).unwrap();
    let screenings = log_json["screenings"].as_array().unwrap();
    assert_eq!(screenings.len(), 1);
    assert_eq!(screenings[0]["verdict"], "allow");
}

#[tokio::test]
async fn first_message_suspicious_rejected_by_safety() {
    let config = test_config();
    let state = DmState::new(config, None);
    let app = router(state);

    let create_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/dm/threads")
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "initiator_user_id": "u-susp-a",
                "recipient_user_id": "u-susp-b"
            })
            .to_string(),
        ))
        .unwrap();

    let resp = app.clone().oneshot(create_req).await.unwrap();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let thread_id = json["thread_id"].as_str().unwrap();

    let msg_req = axum::http::Request::builder()
        .method("POST")
        .uri(format!("/api/v1/dm/threads/{thread_id}/messages"))
        .header("content-type", "application/json")
        .header(INTERNAL_TOKEN_HEADER, TEST_SECRET)
        .body(Body::from(
            serde_json::json!({
                "sender_user_id": "u-susp-a",
                "content": "wire transfer crypto investment send money http:// telegram whatsapp"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(msg_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
