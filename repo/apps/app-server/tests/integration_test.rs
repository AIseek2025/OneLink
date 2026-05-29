mod mock_bff;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use onelink_app_server::config::AppConfig;
use onelink_app_server::router;
use onelink_app_server::state::AppState;
use tower::ServiceExt;

fn app_with_bff(bff_url: &str) -> axum::Router {
    let config = AppConfig {
        bff_base_url: bff_url.to_string(),
        ..Default::default()
    };
    let state = AppState::new(config);
    router::router(state)
}

fn bff_home_response() -> serde_json::Value {
    serde_json::json!({
        "user": {
            "user_id": "00000000-0000-0000-0000-000000000001",
            "nickname": "Test",
            "avatar_url": null,
            "first_run": false
        },
        "profile": {
            "user_id": "00000000-0000-0000-0000-000000000001",
            "nickname": "Test",
            "avatar_url": null,
            "bio": null,
            "age": null,
            "location": null,
            "updated_at": "2026-01-01T00:00:00Z"
        },
        "completion": {
            "pending_facts": []
        }
    })
}

#[tokio::test]
async fn test_boot_with_mock_bff() {
    let mut mock = mock_bff::MockBff::start().await;
    mock.set_response(
        "GET",
        "/api/v1/bff/auth/session/refresh",
        200,
        serde_json::json!({"access_token": "t", "refresh_token": "r", "expires_at": "2026-01-01T00:00:00Z"}),
    ).await;
    mock.set_response("GET", "/api/v1/bff/home", 200, bff_home_response())
        .await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .uri("/api/v1/bff/boot")
        .header("Authorization", "Bearer test-token")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["boot_state"]["has_session"], true);
    mock.shutdown();
}

#[tokio::test]
async fn test_boot_no_auth() {
    let mut mock = mock_bff::MockBff::start().await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .uri("/api/v1/bff/boot")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["boot_state"]["has_session"], false);
    mock.shutdown();
}

#[tokio::test]
async fn test_home_with_mock_bff() {
    let mut mock = mock_bff::MockBff::start().await;
    mock.set_response("GET", "/api/v1/bff/home", 200, bff_home_response())
        .await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .uri("/api/v1/bff/home")
        .header("Authorization", "Bearer test-token")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["user"]["user_id"], "00000000-0000-0000-0000-000000000001");
    assert_eq!(json["profile"]["nickname"], "Test");
    mock.shutdown();
}

#[tokio::test]
async fn test_chat_init_with_mock_bff() {
    let mut mock = mock_bff::MockBff::start().await;
    mock.set_response(
        "GET",
        "/api/v1/bff/chat/init",
        200,
        serde_json::json!({
            "user": { "user_id": "00000000-0000-0000-0000-000000000001" },
            "conversation": { "conversation_id": "conv-1" },
            "pending_questions": []
        }),
    )
    .await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .uri("/api/v1/bff/chat/init")
        .header("Authorization", "Bearer test-token")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["conversation"]["conversation_id"], "conv-1");
    mock.shutdown();
}

#[tokio::test]
async fn test_onboarding_with_mock_bff() {
    let mut mock = mock_bff::MockBff::start().await;
    mock.set_response(
        "GET",
        "/api/v1/bff/onboarding",
        200,
        serde_json::json!({
            "user": { "user_id": "00000000-0000-0000-0000-000000000001" },
            "pending_questions": [],
            "progress": { "completed": 0, "total": 5 }
        }),
    )
    .await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .uri("/api/v1/bff/onboarding")
        .header("Authorization", "Bearer test-token")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["progress"]["total"], 5);
    mock.shutdown();
}

#[tokio::test]
async fn test_profile_patch_with_mock_bff() {
    let mut mock = mock_bff::MockBff::start().await;
    mock.set_response(
        "PATCH",
        "/api/v1/bff/profile/me",
        200,
        serde_json::json!({
            "ok": true,
            "display_name": "Updated"
        }),
    )
    .await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .method("PATCH")
        .uri("/api/v1/bff/profile/me")
        .header("Authorization", "Bearer test-token")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({ "display_name": "Updated" }).to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["ok"], true);
    mock.shutdown();
}

#[tokio::test]
async fn test_question_answers_with_mock_bff() {
    let mut mock = mock_bff::MockBff::start().await;
    mock.set_response(
        "POST",
        "/api/v1/bff/questions/answers",
        200,
        serde_json::json!({
            "accepted": true,
            "delivery_id": "delivery-1"
        }),
    )
    .await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/bff/questions/answers")
        .header("Authorization", "Bearer test-token")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "delivery_id": "delivery-1",
                "variant_id": "variant-1",
                "answer": "yes"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["accepted"], true);
    mock.shutdown();
}

#[tokio::test]
async fn test_login_with_mock_bff() {
    let mut mock = mock_bff::MockBff::start().await;
    mock.set_response(
        "POST",
        "/api/v1/bff/auth/login",
        200,
        serde_json::json!({
            "user_id": "00000000-0000-0000-0000-000000000002",
            "session": {
                "token": "login-token",
                "expires_at": "2026-01-01T00:00:00Z"
            }
        }),
    )
    .await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/bff/auth/login")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "provider": "email",
                "email": "test@example.com",
                "password": "Secret123!"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["user_id"], "00000000-0000-0000-0000-000000000002");
    assert_eq!(json["session"]["token"], "login-token");
    mock.shutdown();
}

#[tokio::test]
async fn test_register_with_mock_bff() {
    let mut mock = mock_bff::MockBff::start().await;
    mock.set_response(
        "POST",
        "/api/v1/bff/auth/register",
        200,
        serde_json::json!({
            "user_id": "00000000-0000-0000-0000-000000000003",
            "session": {
                "token": "register-token",
                "expires_at": "2026-01-01T00:00:00Z"
            }
        }),
    )
    .await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/bff/auth/register")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "provider": "email",
                "email": "new@example.com",
                "password": "Secret123!",
                "primary_region": "CN",
                "primary_language": "zh"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["user_id"], "00000000-0000-0000-0000-000000000003");
    assert_eq!(json["session"]["token"], "register-token");
    mock.shutdown();
}

#[tokio::test]
async fn test_find_request_create_with_mock_bff() {
    let mut mock = mock_bff::MockBff::start().await;
    mock.set_response(
        "POST",
        "/api/v1/bff/find/requests",
        200,
        serde_json::json!({
            "request_id": "00000000-0000-0000-0000-000000000003",
            "state": "submitted",
            "created_at": "2026-01-01T00:00:00Z"
        }),
    )
    .await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/bff/find/requests")
        .header("Authorization", "Bearer test-token")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({"intent_text": "想找志同道合的朋友", "preferred_region": "CN"})
                .to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["request_id"], "00000000-0000-0000-0000-000000000003");
    assert_eq!(json["state"], "submitted");
    mock.shutdown();
}

#[tokio::test]
async fn test_recommendation_list_with_mock_bff() {
    let mut mock = mock_bff::MockBff::start().await;
    mock.set_response(
        "GET",
        "/api/v1/bff/recommendations",
        200,
        serde_json::json!({
            "request_id": "00000000-0000-0000-0000-000000000003",
            "state": "results_ready",
            "recommendations": [
                {
                    "recommendation_id": "00000000-0000-0000-0000-000000000010",
                    "display_name": "Alice",
                    "avatar_url": null,
                    "match_score": 0.85,
                    "explanation_summary": "兴趣匹配度高"
                },
                {
                    "recommendation_id": "00000000-0000-0000-0000-000000000011",
                    "display_name": "Bob",
                    "avatar_url": null,
                    "match_score": 0.72,
                    "explanation_summary": "地域接近"
                }
            ]
        }),
    )
    .await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .uri("/api/v1/bff/recommendations")
        .header("Authorization", "Bearer test-token")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["state"], "results_ready");
    assert_eq!(json["recommendations"].as_array().unwrap().len(), 2);
    mock.shutdown();
}

#[tokio::test]
async fn test_screens_index_includes_a2() {
    let mut mock = mock_bff::MockBff::start().await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .uri("/api/v1/bff/screens")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let screens = json.as_array().unwrap();
    assert!(screens.len() >= 11);
    let a2: Vec<&serde_json::Value> = screens
        .iter()
        .filter(|s| s["priority"].as_str() == Some("a2"))
        .collect();
    assert_eq!(a2.len(), 3);
    mock.shutdown();
}

#[tokio::test]
async fn test_find_request_empty_intent_rejected() {
    let mut mock = mock_bff::MockBff::start().await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/bff/find/requests")
        .header("Authorization", "Bearer test-token")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({"intent_text": ""}).to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    mock.shutdown();
}

#[tokio::test]
async fn test_unauth_routes_reject_missing_auth() {
    let mut mock = mock_bff::MockBff::start().await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .uri("/api/v1/bff/me/summary")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    mock.shutdown();
}

#[tokio::test]
async fn test_unauth_error_returns_i18n_zh_cn() {
    let mut mock = mock_bff::MockBff::start().await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .uri("/api/v1/bff/me/summary")
        .header("Accept-Language", "zh-CN")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["code"], "auth_error");
    assert_eq!(json["message_key"], "app.auth.session.expired");
    assert_eq!(json["localized_message"], "会话已过期，请重新登录");
    println!(
        "[i18n-error-zh] code={}, key={}, msg={}",
        json["code"], json["message_key"], json["localized_message"]
    );
    mock.shutdown();
}

#[tokio::test]
async fn test_unauth_error_returns_i18n_en() {
    let mut mock = mock_bff::MockBff::start().await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .uri("/api/v1/bff/me/summary")
        .header("Accept-Language", "en")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["code"], "auth_error");
    assert_eq!(json["message_key"], "app.auth.session.expired");
    assert_eq!(
        json["localized_message"],
        "Session expired, please log in again"
    );
    println!(
        "[i18n-error-en] code={}, key={}, msg={}",
        json["code"], json["message_key"], json["localized_message"]
    );
    mock.shutdown();
}

#[tokio::test]
async fn test_validation_error_returns_i18n_localized() {
    let mut mock = mock_bff::MockBff::start().await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/bff/find/requests")
        .header("Authorization", "Bearer test-token")
        .header("Accept-Language", "en")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({"intent_text": ""}).to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["code"], "validation_error");
    assert!(json["message_key"].is_string());
    assert!(json["localized_message"].is_string());
    println!(
        "[i18n-validation-error] code={}, key={}, msg={}",
        json["code"], json["message_key"], json["localized_message"]
    );
    mock.shutdown();
}

#[tokio::test]
async fn test_dm_empty_message_error_returns_i18n_localized() {
    let mut mock = mock_bff::MockBff::start().await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/bff/dm/threads/draft")
        .header("Authorization", "Bearer test-token")
        .header("Accept-Language", "zh-CN")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "recommendation_id": "00000000-0000-0000-0000-000000000010",
                "initial_message": ""
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["code"], "validation_error");
    assert_eq!(json["message_key"], "dm.first_message.blocked");
    assert_eq!(
        json["localized_message"],
        "首条消息未通过安全审查，无法发送"
    );
    println!(
        "[i18n-dm-error-zh] code={}, key={}, msg={}",
        json["code"], json["message_key"], json["localized_message"]
    );
    mock.shutdown();
}

#[tokio::test]
async fn test_dm_draft_with_mock_bff() {
    let mut mock = mock_bff::MockBff::start().await;
    mock.set_response(
        "POST",
        "/api/v1/bff/dm/threads/draft",
        200,
        serde_json::json!({
            "thread_id": "00000000-0000-0000-0000-000000000020",
            "state": "draft"
        }),
    )
    .await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/bff/dm/threads/draft")
        .header("Authorization", "Bearer test-token")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "recommendation_id": "00000000-0000-0000-0000-000000000010",
                "initial_message": "你好，很高兴认识你！"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["thread_id"], "00000000-0000-0000-0000-000000000020");
    assert_eq!(json["state"], "draft");
    mock.shutdown();
}

#[tokio::test]
async fn test_dm_first_message_submit_with_mock_bff() {
    let mut mock = mock_bff::MockBff::start().await;
    mock.set_response(
        "POST",
        "/api/v1/bff/dm/threads/first-message",
        200,
        serde_json::json!({
            "thread_id": "00000000-0000-0000-0000-000000000020",
            "state": "under_review",
            "safety_decision": null
        }),
    )
    .await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/bff/dm/threads/first-message")
        .header("Authorization", "Bearer test-token")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "thread_id": "00000000-0000-0000-0000-000000000020",
                "message": "你好，想和你交流一下"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["state"], "under_review");
    mock.shutdown();
}

#[tokio::test]
async fn test_report_submit_with_mock_bff() {
    let mut mock = mock_bff::MockBff::start().await;
    mock.set_response(
        "POST",
        "/api/v1/bff/safety/reports",
        200,
        serde_json::json!({
            "report_id": "00000000-0000-0000-0000-000000000030",
            "category": "harassment",
            "created_at": "2026-01-01T00:00:00Z"
        }),
    )
    .await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/bff/safety/reports")
        .header("Authorization", "Bearer test-token")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "target_user_id": "00000000-0000-0000-0000-000000000015",
                "category": "harassment",
                "description": "骚扰行为"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["report_id"], "00000000-0000-0000-0000-000000000030");
    assert_eq!(json["category"], "harassment");
    mock.shutdown();
}

#[tokio::test]
async fn test_block_apply_with_mock_bff() {
    let mut mock = mock_bff::MockBff::start().await;
    mock.set_response(
        "POST",
        "/api/v1/bff/safety/blocks",
        200,
        serde_json::json!({
            "blocked_user_id": "00000000-0000-0000-0000-000000000015",
            "blocked_at": "2026-01-01T00:00:00Z"
        }),
    )
    .await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/bff/safety/blocks")
        .header("Authorization", "Bearer test-token")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "target_user_id": "00000000-0000-0000-0000-000000000015",
                "reason": "不受欢迎的互动"
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        json["blocked_user_id"],
        "00000000-0000-0000-0000-000000000015"
    );
    mock.shutdown();
}

#[tokio::test]
async fn test_appeal_status_with_mock_bff() {
    let mut mock = mock_bff::MockBff::start().await;
    mock.set_response(
        "GET",
        "/api/v1/bff/safety/appeals/00000000-0000-0000-0000-000000000040",
        200,
        serde_json::json!({
            "appeal_id": "00000000-0000-0000-0000-000000000040",
            "status": "pending",
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        }),
    )
    .await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .uri("/api/v1/bff/safety/appeals/00000000-0000-0000-0000-000000000040")
        .header("Authorization", "Bearer test-token")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["appeal_id"], "00000000-0000-0000-0000-000000000040");
    assert_eq!(json["status"], "pending");
    mock.shutdown();
}

#[tokio::test]
async fn test_locale_get_with_mock_bff() {
    let mut mock = mock_bff::MockBff::start().await;
    mock.set_response(
        "GET",
        "/api/v1/bff/settings/locale",
        200,
        serde_json::json!({
            "available_locales": ["zh-CN", "en-US", "ja-JP"],
            "available_regions": ["CN", "US", "JP"],
            "available_timezones": ["Asia/Shanghai", "America/New_York", "Asia/Tokyo"]
        }),
    )
    .await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .uri("/api/v1/bff/settings/locale")
        .header("Authorization", "Bearer test-token")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["available_locales"].as_array().unwrap().len(), 3);
    assert_eq!(json["available_regions"].as_array().unwrap().len(), 3);
    mock.shutdown();
}

#[tokio::test]
async fn test_compliance_summary_with_mock_bff() {
    let mut mock = mock_bff::MockBff::start().await;
    mock.set_response(
        "GET",
        "/api/v1/bff/compliance/summary",
        200,
        serde_json::json!({
            "user_id": "00000000-0000-0000-0000-000000000001",
            "data_export_available": true,
            "data_delete_available": true,
            "data_correction_available": true,
            "pending_requests": []
        }),
    )
    .await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .uri("/api/v1/bff/compliance/summary")
        .header("Authorization", "Bearer test-token")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["data_export_available"], true);
    assert_eq!(json["data_delete_available"], true);
    mock.shutdown();
}

#[tokio::test]
async fn test_compliance_export_with_mock_bff() {
    let mut mock = mock_bff::MockBff::start().await;
    mock.set_response(
        "POST",
        "/api/v1/bff/compliance/export",
        200,
        serde_json::json!({
            "request_id": "00000000-0000-0000-0000-000000000050",
            "action_type": "export",
            "status": "processing",
            "created_at": "2026-01-01T00:00:00Z"
        }),
    )
    .await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/bff/compliance/export")
        .header("Authorization", "Bearer test-token")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({"action_type": "export", "export_format": "json"}).to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["request_id"], "00000000-0000-0000-0000-000000000050");
    assert_eq!(json["action_type"], "export");
    mock.shutdown();
}

#[tokio::test]
async fn test_compliance_delete_with_mock_bff() {
    let mut mock = mock_bff::MockBff::start().await;
    mock.set_response(
        "POST",
        "/api/v1/bff/compliance/delete",
        200,
        serde_json::json!({
            "request_id": "00000000-0000-0000-0000-000000000051",
            "action_type": "delete",
            "status": "processing",
            "created_at": "2026-01-01T00:00:00Z"
        }),
    )
    .await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/bff/compliance/delete")
        .header("Authorization", "Bearer test-token")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({"action_type": "delete", "scope": "all"}).to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["action_type"], "delete");
    mock.shutdown();
}

#[tokio::test]
async fn test_compliance_correction_with_mock_bff() {
    let mut mock = mock_bff::MockBff::start().await;
    mock.set_response(
        "POST",
        "/api/v1/bff/compliance/correction",
        200,
        serde_json::json!({
            "request_id": "00000000-0000-0000-0000-000000000052",
            "action_type": "correction",
            "status": "processing",
            "created_at": "2026-01-01T00:00:00Z"
        }),
    )
    .await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/bff/compliance/correction")
        .header("Authorization", "Bearer test-token")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({"action_type": "correction", "field_name": "nickname"}).to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["action_type"], "correction");
    mock.shutdown();
}

#[tokio::test]
async fn test_screens_index_includes_a3_a4() {
    let mut mock = mock_bff::MockBff::start().await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .uri("/api/v1/bff/screens")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let screens = json.as_array().unwrap();
    assert!(screens.len() >= 18);
    let a3: Vec<&serde_json::Value> = screens
        .iter()
        .filter(|s| s["priority"].as_str() == Some("a3"))
        .collect();
    assert_eq!(a3.len(), 5);
    let a4: Vec<&serde_json::Value> = screens
        .iter()
        .filter(|s| s["priority"].as_str() == Some("a4"))
        .collect();
    assert_eq!(a4.len(), 2);
    mock.shutdown();
}

#[tokio::test]
async fn test_dm_draft_empty_message_rejected() {
    let mut mock = mock_bff::MockBff::start().await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/bff/dm/threads/draft")
        .header("Authorization", "Bearer test-token")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "recommendation_id": "00000000-0000-0000-0000-000000000010",
                "initial_message": ""
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    mock.shutdown();
}

#[tokio::test]
async fn test_dm_first_message_empty_rejected() {
    let mut mock = mock_bff::MockBff::start().await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/bff/dm/threads/first-message")
        .header("Authorization", "Bearer test-token")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "thread_id": "00000000-0000-0000-0000-000000000020",
                "message": ""
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    mock.shutdown();
}

#[tokio::test]
async fn test_a0_a1_main_chain_e2e() {
    let mut mock = mock_bff::MockBff::start().await;

    mock.set_response(
        "POST",
        "/api/v1/bff/auth/login",
        200,
        serde_json::json!({
            "user_id": "00000000-0000-0000-0000-000000000001",
            "session": {
                "token": "e2e-token",
                "expires_at": "2026-01-01T00:00:00Z"
            }
        }),
    )
    .await;

    mock.set_response(
        "GET",
        "/api/v1/bff/auth/session/refresh",
        200,
        serde_json::json!({
            "access_token": "e2e-token-2",
            "refresh_token": "e2e-refresh-2",
            "expires_at": "2026-01-02T00:00:00Z"
        }),
    )
    .await;

    mock.set_response("GET", "/api/v1/bff/home", 200, bff_home_response())
        .await;

    mock.set_response(
        "GET",
        "/api/v1/bff/chat/init",
        200,
        serde_json::json!({
            "conversations": [{
                "conversation_id": "00000000-0000-0000-0000-000000000100",
                "title": "Lumi",
                "last_message_preview": "你好！",
                "last_message_at": "2026-01-01T00:00:00Z",
                "unread_count": 1
            }]
        }),
    )
    .await;

    mock.set_response(
        "POST",
        "/api/v1/bff/chat/messages",
        200,
        serde_json::json!({
            "session_id": "00000000-0000-0000-0000-000000000100",
            "reply": {
                "id": "00000000-0000-0000-0000-000000000101",
                "conversation_id": "00000000-0000-0000-0000-000000000100",
                "role": "assistant",
                "content": "很高兴认识你！",
                "created_at": "2026-01-01T00:01:00Z",
                "trace_id": "e2e-trace-001"
            }
        }),
    )
    .await;

    mock.set_response(
        "GET",
        "/api/v1/bff/profile/confirmations",
        200,
        serde_json::json!({
            "pending_facts": [{
                "fact_id": "00000000-0000-0000-0000-000000000070",
                "fact_text": "你喜欢户外运动",
                "source": "ai_inference",
                "confidence": 0.85,
                "state": "pending_confirmation"
            }]
        }),
    )
    .await;

    mock.set_response(
        "GET",
        "/api/v1/bff/settings/locale",
        200,
        serde_json::json!({
            "available_locales": ["zh-CN", "en-US"],
            "available_regions": ["CN", "US"],
            "available_timezones": ["Asia/Shanghai", "America/New_York"]
        }),
    )
    .await;

    let app = app_with_bff(&mock.url());

    let req_boot = Request::builder()
        .uri("/api/v1/bff/boot")
        .header("Authorization", "Bearer e2e-token")
        .body(Body::empty())
        .unwrap();
    let resp_boot = app.clone().oneshot(req_boot).await.unwrap();
    assert_eq!(resp_boot.status(), StatusCode::OK);
    let boot_bytes = resp_boot.into_body().collect().await.unwrap().to_bytes();
    let boot_json: serde_json::Value = serde_json::from_slice(&boot_bytes).unwrap();
    assert_eq!(boot_json["boot_state"]["has_session"], true);

    let req_login = Request::builder()
        .method("POST")
        .uri("/api/v1/bff/auth/login")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "provider": "email",
                "email": "e2e@example.com",
                "password": "Secret123!"
            })
            .to_string(),
        ))
        .unwrap();
    let resp_login = app.clone().oneshot(req_login).await.unwrap();
    assert_eq!(resp_login.status(), StatusCode::OK);
    let login_bytes = resp_login.into_body().collect().await.unwrap().to_bytes();
    let login_json: serde_json::Value = serde_json::from_slice(&login_bytes).unwrap();
    assert_eq!(
        login_json["user_id"],
        "00000000-0000-0000-0000-000000000001"
    );
    assert_eq!(login_json["session"]["token"], "e2e-token");

    let req_conv = Request::builder()
        .uri("/api/v1/bff/conversations")
        .header("Authorization", "Bearer e2e-token")
        .body(Body::empty())
        .unwrap();
    let resp_conv = app.clone().oneshot(req_conv).await.unwrap();
    assert_eq!(resp_conv.status(), StatusCode::OK);
    let conv_bytes = resp_conv.into_body().collect().await.unwrap().to_bytes();
    let conv_json: serde_json::Value = serde_json::from_slice(&conv_bytes).unwrap();
    assert_eq!(conv_json["conversations"].as_array().unwrap().len(), 1);
    assert_eq!(conv_json["conversations"][0]["title"], "Lumi");

    let req_chat = Request::builder()
        .method("POST")
        .uri("/api/v1/bff/conversations/00000000-0000-0000-0000-000000000100/messages")
        .header("Authorization", "Bearer e2e-token")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({"message": "你好 Lumi"}).to_string(),
        ))
        .unwrap();
    let resp_chat = app.clone().oneshot(req_chat).await.unwrap();
    assert_eq!(resp_chat.status(), StatusCode::OK);
    let chat_bytes = resp_chat.into_body().collect().await.unwrap().to_bytes();
    let chat_json: serde_json::Value = serde_json::from_slice(&chat_bytes).unwrap();
    assert_eq!(chat_json["reply"]["role"], "assistant");

    let req_locale = Request::builder()
        .uri("/api/v1/bff/settings/locale")
        .header("Authorization", "Bearer e2e-token")
        .body(Body::empty())
        .unwrap();
    let resp_locale = app.clone().oneshot(req_locale).await.unwrap();
    assert_eq!(resp_locale.status(), StatusCode::OK);

    mock.shutdown();
}

#[tokio::test]
async fn test_phase2b_product_loop_e2e() {
    let mut mock = mock_bff::MockBff::start().await;

    mock.set_response(
        "POST",
        "/api/v1/bff/auth/login",
        200,
        serde_json::json!({
            "user_id": "00000000-0000-0000-0000-000000000001",
            "access_token": "p2b-token",
            "refresh_token": "p2b-refresh",
            "expires_at": "2026-01-01T00:00:00Z",
            "first_run": false
        }),
    )
    .await;

    mock.set_response(
        "POST",
        "/api/v1/bff/find/requests",
        200,
        serde_json::json!({
            "request_id": "00000000-0000-0000-0000-000000000100",
            "state": "submitted",
            "created_at": "2026-01-01T00:00:00Z"
        }),
    )
    .await;

    mock.set_response(
        "GET",
        "/api/v1/bff/recommendations",
        200,
        serde_json::json!({
            "request_id": "00000000-0000-0000-0000-000000000100",
            "state": "results_ready",
            "recommendations": [{
                "recommendation_id": "00000000-0000-0000-0000-000000000110",
                "display_name": "小明",
                "avatar_url": null,
                "match_score": 0.88,
                "explanation_summary": "兴趣和地域高度匹配"
            }]
        }),
    )
    .await;

    mock.set_response(
        "POST",
        "/api/v1/bff/dm/threads/first-message",
        200,
        serde_json::json!({
            "thread_id": "00000000-0000-0000-0000-000000000200",
            "state": "under_review",
            "safety_decision": null
        }),
    )
    .await;

    mock.set_response(
        "POST",
        "/api/v1/bff/safety/reports",
        200,
        serde_json::json!({
            "report_id": "00000000-0000-0000-0000-000000000300",
            "category": "inappropriate_content",
            "created_at": "2026-01-01T00:01:00Z"
        }),
    )
    .await;

    mock.set_response(
        "POST",
        "/api/v1/bff/safety/blocks",
        200,
        serde_json::json!({
            "blocked_user_id": "00000000-0000-0000-0000-000000000110",
            "blocked_at": "2026-01-01T00:02:00Z"
        }),
    )
    .await;

    mock.set_response(
        "GET",
        "/api/v1/bff/safety/appeals/00000000-0000-0000-0000-000000000400",
        200,
        serde_json::json!({
            "appeal_id": "00000000-0000-0000-0000-000000000400",
            "status": "pending",
            "created_at": "2026-01-01T00:03:00Z",
            "updated_at": "2026-01-01T00:03:00Z"
        }),
    )
    .await;

    mock.set_response(
        "GET",
        "/api/v1/bff/admin/reports",
        200,
        serde_json::json!({
            "reports": [{
                "report_id": "00000000-0000-0000-0000-000000000300",
                "reporter_id": "00000000-0000-0000-0000-000000000001",
                "target_user_id": "00000000-0000-0000-0000-000000000110",
                "category": "inappropriate_content",
                "status": "pending",
                "created_at": "2026-01-01T00:01:00Z"
            }],
            "total_count": 1
        }),
    )
    .await;

    mock.set_response(
        "POST",
        "/api/v1/bff/admin/reports/00000000-0000-0000-0000-000000000300/action",
        200,
        serde_json::json!({
            "report_id": "00000000-0000-0000-0000-000000000300",
            "action_type": "ban",
            "result": "applied",
            "acted_at": "2026-01-01T00:05:00Z"
        }),
    )
    .await;

    let app = app_with_bff(&mock.url());
    let auth = "Bearer p2b-token";

    let resp_login = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/bff/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"phone": "13900000000", "code": "5678"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_login.status(), StatusCode::OK);
    println!("[step-1] login: status={}", resp_login.status());

    let resp_find = app.clone().oneshot(
        Request::builder()
            .method("POST")
            .uri("/api/v1/bff/find/requests")
            .header("Authorization", auth)
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::json!({"intent_text": "想找志同道合的朋友", "preferred_region": "CN"}).to_string()))
            .unwrap(),
    ).await.unwrap();
    assert_eq!(resp_find.status(), StatusCode::OK);
    let find_bytes = resp_find.into_body().collect().await.unwrap().to_bytes();
    let find_json: serde_json::Value = serde_json::from_slice(&find_bytes).unwrap();
    assert_eq!(find_json["state"], "submitted");
    println!("[step-2] find/requests: state={}", find_json["state"]);

    let resp_rec = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/bff/recommendations")
                .header("Authorization", auth)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_rec.status(), StatusCode::OK);
    let rec_bytes = resp_rec.into_body().collect().await.unwrap().to_bytes();
    let rec_json: serde_json::Value = serde_json::from_slice(&rec_bytes).unwrap();
    assert_eq!(rec_json["state"], "results_ready");
    assert_eq!(rec_json["recommendations"].as_array().unwrap().len(), 1);
    assert_eq!(
        rec_json["recommendations"][0]["explanation_summary"],
        "兴趣和地域高度匹配"
    );
    println!(
        "[step-3] recommendations: state={}, count={}, explanation={}",
        rec_json["state"],
        rec_json["recommendations"].as_array().unwrap().len(),
        rec_json["recommendations"][0]["explanation_summary"]
    );

    let resp_dm = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/bff/dm/threads/first-message")
                .header("Authorization", auth)
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "thread_id": "00000000-0000-0000-0000-000000000200",
                        "message": "你好，很高兴认识你！"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_dm.status(), StatusCode::OK);
    let dm_bytes = resp_dm.into_body().collect().await.unwrap().to_bytes();
    let dm_json: serde_json::Value = serde_json::from_slice(&dm_bytes).unwrap();
    assert_eq!(dm_json["state"], "under_review");
    println!("[step-4] dm first-message: state={}", dm_json["state"]);

    let resp_report = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/bff/safety/reports")
                .header("Authorization", auth)
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "target_user_id": "00000000-0000-0000-0000-000000000110",
                        "category": "inappropriate_content",
                        "description": "不当内容"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_report.status(), StatusCode::OK);
    let report_bytes = resp_report.into_body().collect().await.unwrap().to_bytes();
    let report_json: serde_json::Value = serde_json::from_slice(&report_bytes).unwrap();
    assert_eq!(report_json["category"], "inappropriate_content");
    println!(
        "[step-5] safety/reports: category={}",
        report_json["category"]
    );

    let resp_block = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/bff/safety/blocks")
                .header("Authorization", auth)
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "target_user_id": "00000000-0000-0000-0000-000000000110",
                        "reason": "不受欢迎的互动"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_block.status(), StatusCode::OK);
    let block_bytes = resp_block.into_body().collect().await.unwrap().to_bytes();
    let block_json: serde_json::Value = serde_json::from_slice(&block_bytes).unwrap();
    assert_eq!(
        block_json["blocked_user_id"],
        "00000000-0000-0000-0000-000000000110"
    );
    println!(
        "[step-6] safety/blocks: blocked_user_id={}",
        block_json["blocked_user_id"]
    );

    let resp_appeal = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/bff/safety/appeals/00000000-0000-0000-0000-000000000400")
                .header("Authorization", auth)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_appeal.status(), StatusCode::OK);
    let appeal_bytes = resp_appeal.into_body().collect().await.unwrap().to_bytes();
    let appeal_json: serde_json::Value = serde_json::from_slice(&appeal_bytes).unwrap();
    assert_eq!(appeal_json["status"], "pending");
    println!("[step-7] safety/appeals: status={}", appeal_json["status"]);

    let resp_admin_reports = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/bff/admin/reports")
                .header("Authorization", auth)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_admin_reports.status(), StatusCode::OK);
    let admin_reports_bytes = resp_admin_reports
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let admin_reports_json: serde_json::Value =
        serde_json::from_slice(&admin_reports_bytes).unwrap();
    assert!(!admin_reports_json["reports"].as_array().unwrap().is_empty());
    assert_eq!(
        admin_reports_json["reports"][0]["category"],
        "inappropriate_content"
    );
    assert_eq!(admin_reports_json["reports"][0]["status"], "pending");
    println!(
        "[step-8] admin/reports: count={}, reports[0].category={}, reports[0].status={}",
        admin_reports_json["reports"].as_array().unwrap().len(),
        admin_reports_json["reports"][0]["category"],
        admin_reports_json["reports"][0]["status"]
    );

    let resp_admin_action = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/bff/admin/reports/00000000-0000-0000-0000-000000000300/action")
                .header("Authorization", auth)
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "action_type": "ban",
                        "reason": "违反社区规范"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_admin_action.status(), StatusCode::OK);
    let admin_action_bytes = resp_admin_action
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let admin_action_json: serde_json::Value = serde_json::from_slice(&admin_action_bytes).unwrap();
    assert_eq!(admin_action_json["action_type"], "ban");
    assert_eq!(admin_action_json["result"], "applied");
    println!(
        "[step-9] admin/reports/:id/action: action_type={}, result={}",
        admin_action_json["action_type"], admin_action_json["result"]
    );

    mock.shutdown();
}

#[tokio::test]
async fn test_settings_locale_update_with_mock_bff() {
    let mut mock = mock_bff::MockBff::start().await;
    mock.set_response(
        "POST",
        "/api/v1/bff/profile/me",
        200,
        serde_json::json!({
            "locale": "en",
            "region": "US",
            "timezone": "America/New_York",
            "content_language": "en",
            "notification_language": "en"
        }),
    )
    .await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .method("PATCH")
        .uri("/api/v1/bff/settings/locale/update")
        .header("Authorization", "Bearer test-token")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({
                "locale": Some("en"),
                "region": Some("US"),
                "timezone": Some("America/New_York"),
                "content_language": Some("en"),
                "notification_language": Some("en")
            })
            .to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["locale"], "en");
    assert_eq!(json["region"], "US");
    assert_eq!(json["timezone"], "America/New_York");
    println!(
        "[settings-locale-update] locale={}, region={}, tz={}",
        json["locale"], json["region"], json["timezone"]
    );
    mock.shutdown();
}

#[tokio::test]
async fn test_i18n_registry_endpoint() {
    let mut mock = mock_bff::MockBff::start().await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .uri("/api/v1/bff/i18n/registry")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(json["message_key_count"].as_u64().unwrap() >= 28);
    assert!(json["supported_locales"].as_array().unwrap().len() >= 2);
    assert!(json["supported_regions"].as_array().unwrap().len() >= 4);
    assert!(json["supported_timezones"].as_array().unwrap().len() >= 4);
    assert_eq!(json["default_locale"], "zh-CN");
    assert_eq!(json["default_region"], "CN");
    assert!(json["review_info"].as_object().unwrap().len() >= 10);
    println!(
        "[i18n-registry] keys={}, locales={}, regions={}, tz={}",
        json["message_key_count"],
        json["supported_locales"].as_array().unwrap().len(),
        json["supported_regions"].as_array().unwrap().len(),
        json["supported_timezones"].as_array().unwrap().len()
    );
    mock.shutdown();
}

#[tokio::test]
async fn test_i18n_translate_endpoint() {
    let mut mock = mock_bff::MockBff::start().await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .uri("/api/v1/bff/i18n/translate?key=safety.block.applied&locale=en")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["key"], "safety.block.applied");
    assert_eq!(json["locale"], "en");
    assert_eq!(json["translation"], "This user has been blocked");
    println!(
        "[i18n-translate] key={}, locale={}, translation={}",
        json["key"], json["locale"], json["translation"]
    );
    mock.shutdown();
}

#[tokio::test]
async fn test_i18n_translate_fallback() {
    let mut mock = mock_bff::MockBff::start().await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .uri("/api/v1/bff/i18n/translate?key=safety.block.applied&locale=ja-JP")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["locale"], "zh-CN");
    assert_eq!(json["translation"], "该用户已被拉黑");
    println!(
        "[i18n-translate-fallback] locale resolved={}, translation={}",
        json["locale"], json["translation"]
    );
    mock.shutdown();
}

#[tokio::test]
async fn test_i18n_translate_missing_key() {
    let mut mock = mock_bff::MockBff::start().await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .uri("/api/v1/bff/i18n/translate?key=nonexistent.key")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(json["translation"]
        .as_str()
        .unwrap()
        .contains("nonexistent.key"));
    println!(
        "[i18n-translate-missing] translation={}",
        json["translation"]
    );
    mock.shutdown();
}

#[tokio::test]
async fn test_bff_contract_sync_evidence() {
    use onelink_app_server::contract_freeze::frozen_bff_contract_manifest;
    use onelink_app_server::screens::{a0_a1_screens, a2_screens, a3_screens, a4_screens};

    let manifest = frozen_bff_contract_manifest();
    let a1_endpoints: Vec<&str> = manifest
        .endpoints
        .iter()
        .filter(|e| e.phase == "A1")
        .map(|e| e.path.as_str())
        .collect();
    assert!(
        a1_endpoints.len() >= 10,
        "A1 should have >= 10 BFF endpoints, got {}",
        a1_endpoints.len()
    );

    let a1_screens = a0_a1_screens();
    let a1_screen_bff_refs: Vec<String> = a1_screens
        .iter()
        .filter(|s| {
            s.priority == onelink_app_server::screens::ScreenPriority::A1
                || s.priority == onelink_app_server::screens::ScreenPriority::A0
        })
        .flat_map(|s| s.bff_contracts.iter().cloned())
        .collect();
    assert!(
        !a1_screen_bff_refs.is_empty(),
        "A0-A1 screens must reference BFF contracts"
    );

    let all_screens: Vec<_> = a0_a1_screens()
        .into_iter()
        .chain(a2_screens())
        .chain(a3_screens())
        .chain(a4_screens())
        .collect();
    let all_screen_routes: Vec<String> = all_screens.iter().map(|s| s.route.clone()).collect();
    for i in 0..all_screen_routes.len() {
        for j in (i + 1)..all_screen_routes.len() {
            assert_ne!(
                all_screen_routes[i], all_screen_routes[j],
                "duplicate screen route"
            );
        }
    }

    let manifest_keys: Vec<String> = manifest
        .endpoints
        .iter()
        .map(|e| format!("{} {}", e.method, e.path))
        .collect();
    for i in 0..manifest_keys.len() {
        for j in (i + 1)..manifest_keys.len() {
            assert_ne!(
                manifest_keys[i], manifest_keys[j],
                "duplicate BFF contract method+path"
            );
        }
    }

    let json = serde_json::to_string(&manifest).unwrap();
    assert!(!json.is_empty());
    let parsed: onelink_app_server::contract_freeze::ContractFreezeManifest =
        serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.endpoints.len(), manifest.endpoints.len());
}

#[tokio::test]
async fn test_compliance_data_rights_e2e() {
    let mut mock = mock_bff::MockBff::start().await;
    let auth = "Bearer rights-token";

    mock.set_response(
        "GET",
        "/api/v1/bff/compliance/summary",
        200,
        serde_json::json!({
            "user_id": "00000000-0000-0000-0000-000000000001",
            "data_export_available": true,
            "data_delete_available": true,
            "data_correction_available": true,
            "pending_requests": [],
            "profile_facts": [{"fact_id": "f1", "fact_text": "你喜欢户外运动", "source": "ai_inference", "confidence": 0.85}],
            "memory_summaries": [{"summary_id": "s1", "summary_text": "聊过旅行经历"}],
            "key_artifacts": [{"artifact_id": "a1", "artifact_type": "preference", "content_preview": "喜欢咖啡"}],
            "settings": {"locale": "zh-CN", "region": "CN", "timezone": "Asia/Shanghai", "content_language": "zh-CN", "notification_language": "zh-CN"},
            "consent_records": [{"consent_id": "c1", "scope": "crossborder", "granted_at": "2026-01-01T00:00:00Z"}]
        }),
    ).await;

    mock.set_response(
        "POST",
        "/api/v1/bff/compliance/export",
        200,
        serde_json::json!({
            "request_id": "00000000-0000-0000-0000-000000000060",
            "action_type": "export",
            "status": "processing",
            "created_at": "2026-05-25T00:00:00Z"
        }),
    )
    .await;

    mock.set_response(
        "POST",
        "/api/v1/bff/compliance/correction",
        200,
        serde_json::json!({
            "request_id": "00000000-0000-0000-0000-000000000061",
            "action_type": "correction",
            "status": "processing",
            "created_at": "2026-05-25T00:01:00Z"
        }),
    )
    .await;

    mock.set_response(
        "POST",
        "/api/v1/bff/compliance/delete",
        200,
        serde_json::json!({
            "request_id": "00000000-0000-0000-0000-000000000062",
            "action_type": "delete",
            "status": "processing",
            "created_at": "2026-05-25T00:02:00Z"
        }),
    )
    .await;

    let app = app_with_bff(&mock.url());

    let resp_view = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/bff/compliance/summary")
                .header("Authorization", auth)
                .header("Accept-Language", "zh-CN")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_view.status(), StatusCode::OK);
    let view_bytes = resp_view.into_body().collect().await.unwrap().to_bytes();
    let view_json: serde_json::Value = serde_json::from_slice(&view_bytes).unwrap();
    assert_eq!(view_json["data_export_available"], true);
    assert!(!view_json["profile_facts"].as_array().unwrap().is_empty());
    assert!(!view_json["memory_summaries"].as_array().unwrap().is_empty());
    assert!(!view_json["key_artifacts"].as_array().unwrap().is_empty());
    assert!(view_json["settings"].is_object());
    assert!(!view_json["consent_records"].as_array().unwrap().is_empty());
    println!(
        "[rights-1-view] facts={}, summaries={}, artifacts={}, consent={}",
        view_json["profile_facts"].as_array().unwrap().len(),
        view_json["memory_summaries"].as_array().unwrap().len(),
        view_json["key_artifacts"].as_array().unwrap().len(),
        view_json["consent_records"].as_array().unwrap().len()
    );

    let resp_export = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/bff/compliance/export")
                .header("Authorization", auth)
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"action_type": "export", "export_format": "json"})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_export.status(), StatusCode::OK);
    let export_bytes = resp_export.into_body().collect().await.unwrap().to_bytes();
    let export_json: serde_json::Value = serde_json::from_slice(&export_bytes).unwrap();
    assert_eq!(export_json["action_type"], "export");
    println!(
        "[rights-2-export] request_id={}, status={}",
        export_json["request_id"], export_json["status"]
    );

    let resp_correction = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/bff/compliance/correction")
                .header("Authorization", auth)
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"action_type": "correction", "field_name": "nickname"})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_correction.status(), StatusCode::OK);
    let correction_bytes = resp_correction
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let correction_json: serde_json::Value = serde_json::from_slice(&correction_bytes).unwrap();
    assert_eq!(correction_json["action_type"], "correction");
    println!(
        "[rights-3-correction] request_id={}, status={}",
        correction_json["request_id"], correction_json["status"]
    );

    let resp_delete = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/bff/compliance/delete")
                .header("Authorization", auth)
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"action_type": "delete", "scope": "all"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_delete.status(), StatusCode::OK);
    let delete_bytes = resp_delete.into_body().collect().await.unwrap().to_bytes();
    let delete_json: serde_json::Value = serde_json::from_slice(&delete_bytes).unwrap();
    assert_eq!(delete_json["action_type"], "delete");
    println!(
        "[rights-4-delete] request_id={}, status={}",
        delete_json["request_id"], delete_json["status"]
    );

    mock.shutdown();
}

#[tokio::test]
async fn test_compliance_export_blocked_for_restricted_region() {
    let mut mock = mock_bff::MockBff::start().await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/bff/compliance/export")
        .header("Authorization", "Bearer test-token")
        .header("X-User-Region", "EU")
        .header("Accept-Language", "en")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({"action_type": "export", "export_format": "json"}).to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["code"], "safety_blocked");
    assert!(json["localized_message"]
        .as_str()
        .unwrap()
        .contains("residency"));
    println!(
        "[region-gate-compliance] export blocked for EU, localized_message={}",
        json["localized_message"]
    );
    mock.shutdown();
}

#[tokio::test]
async fn test_compliance_delete_blocked_for_restricted_region() {
    let mut mock = mock_bff::MockBff::start().await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/bff/compliance/delete")
        .header("Authorization", "Bearer test-token")
        .header("X-User-Region", "EU")
        .header("Accept-Language", "zh-CN")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({"action_type": "delete", "scope": "all"}).to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["code"], "safety_blocked");
    assert!(json["localized_message"].as_str().unwrap().contains("驻留"));
    println!(
        "[region-gate-compliance] delete blocked for EU (zh-CN), localized_message={}",
        json["localized_message"]
    );
    mock.shutdown();
}

#[tokio::test]
async fn test_compliance_correction_blocked_for_restricted_region() {
    let mut mock = mock_bff::MockBff::start().await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/bff/compliance/correction")
        .header("Authorization", "Bearer test-token")
        .header("X-User-Region", "EU")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({"action_type": "correction", "field_name": "nickname"}).to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    mock.shutdown();
}

#[tokio::test]
async fn test_compliance_export_allowed_for_domestic_region() {
    let mut mock = mock_bff::MockBff::start().await;
    mock.set_response(
        "POST",
        "/api/v1/bff/compliance/export",
        200,
        serde_json::json!({
            "request_id": "00000000-0000-0000-0000-000000000070",
            "action_type": "export",
            "status": "processing",
            "created_at": "2026-01-01T00:00:00Z"
        }),
    )
    .await;
    let app = app_with_bff(&mock.url());
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/bff/compliance/export")
        .header("Authorization", "Bearer test-token")
        .header("X-User-Region", "CN")
        .header("Content-Type", "application/json")
        .body(Body::from(
            serde_json::json!({"action_type": "export", "export_format": "json"}).to_string(),
        ))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["action_type"], "export");
    println!(
        "[region-gate-compliance] export allowed for CN, request_id={}",
        json["request_id"]
    );
    mock.shutdown();
}

#[tokio::test]
async fn test_i18n_fields_end_to_end_via_settings_and_me_summary() {
    let mut mock = mock_bff::MockBff::start().await;

    let mut home_resp = bff_home_response();
    home_resp["user"]["locale"] = serde_json::json!("en");
    home_resp["user"]["region"] = serde_json::json!("US");
    home_resp["profile"]["locale"] = serde_json::json!("en");
    home_resp["profile"]["region"] = serde_json::json!("US");
    home_resp["profile"]["timezone"] = serde_json::json!("America/New_York");
    home_resp["profile"]["content_language"] = serde_json::json!("en");
    home_resp["profile"]["notification_language"] = serde_json::json!("zh-CN");

    mock.set_response(
        "GET",
        "/api/v1/bff/auth/session/refresh",
        200,
        serde_json::json!({"access_token": "t", "refresh_token": "r", "expires_at": "2026-01-01T00:00:00Z"}),
    ).await;
    mock.set_response("GET", "/api/v1/bff/home", 200, home_resp)
        .await;

    let app = app_with_bff(&mock.url());

    let req_me = Request::builder()
        .uri("/api/v1/bff/me/summary")
        .header("Authorization", "Bearer test-token")
        .header("Accept-Language", "en")
        .body(Body::empty())
        .unwrap();
    let resp_me = app.clone().oneshot(req_me).await.unwrap();
    assert_eq!(resp_me.status(), StatusCode::OK);
    let me_bytes = resp_me.into_body().collect().await.unwrap().to_bytes();
    let me_json: serde_json::Value = serde_json::from_slice(&me_bytes).unwrap();
    assert_eq!(me_json["locale"].as_str(), Some("en"));
    assert_eq!(me_json["region"].as_str(), Some("US"));
    println!(
        "[i18n-e2e-me] locale={}, region={}",
        me_json["locale"], me_json["region"]
    );

    let req_settings = Request::builder()
        .uri("/api/v1/bff/settings/summary")
        .header("Authorization", "Bearer test-token")
        .header("Accept-Language", "en")
        .body(Body::empty())
        .unwrap();
    let resp_settings = app.clone().oneshot(req_settings).await.unwrap();
    assert_eq!(resp_settings.status(), StatusCode::OK);
    let settings_bytes = resp_settings
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let settings_json: serde_json::Value = serde_json::from_slice(&settings_bytes).unwrap();
    let s = &settings_json["settings"];
    assert_eq!(s["locale"].as_str(), Some("en"));
    assert_eq!(s["region"].as_str(), Some("US"));
    assert_eq!(s["timezone"].as_str(), Some("America/New_York"));
    assert_eq!(s["content_language"].as_str(), Some("en"));
    assert_eq!(s["notification_language"].as_str(), Some("zh-CN"));
    println!(
        "[i18n-e2e-settings] locale={}, region={}, tz={}, content_lang={}, notif_lang={}",
        s["locale"], s["region"], s["timezone"], s["content_language"], s["notification_language"]
    );

    mock.shutdown();
}

#[tokio::test]
async fn test_region_gate_full_compliance_loop_restricted() {
    let mut mock = mock_bff::MockBff::start().await;

    mock.set_response(
        "GET",
        "/api/v1/bff/compliance/summary",
        200,
        serde_json::json!({
            "user_id": "00000000-0000-0000-0000-000000000001",
            "data_export_available": true,
            "data_delete_available": true,
            "data_correction_available": true,
            "pending_requests": [],
            "profile_facts": [],
            "memory_summaries": [],
            "key_artifacts": [],
            "settings": {"locale": "en", "region": "EU"},
            "consent_records": []
        }),
    )
    .await;

    let app = app_with_bff(&mock.url());
    let auth = "Bearer eu-user-token";

    let resp_gate = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/bff/region/gate")
                .header("X-User-Region", "EU")
                .header("Accept-Language", "en")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_gate.status(), StatusCode::OK);
    let gate_bytes = resp_gate.into_body().collect().await.unwrap().to_bytes();
    let gate_json: serde_json::Value = serde_json::from_slice(&gate_bytes).unwrap();
    assert_eq!(gate_json["data_zone"], "restricted");
    assert_eq!(gate_json["degradation"], "read_only");
    assert!(gate_json["localized_notice"]
        .as_str()
        .unwrap()
        .contains("residency"));
    println!(
        "[region-loop-1] gate: zone={}, degradation={}",
        gate_json["data_zone"], gate_json["degradation"]
    );

    let resp_summary = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/bff/compliance/summary")
                .header("Authorization", auth)
                .header("X-User-Region", "EU")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_summary.status(), StatusCode::OK);
    let summary_bytes = resp_summary.into_body().collect().await.unwrap().to_bytes();
    let summary_json: serde_json::Value = serde_json::from_slice(&summary_bytes).unwrap();
    assert_eq!(summary_json["data_export_available"], false);
    assert_eq!(summary_json["data_delete_available"], false);
    assert_eq!(summary_json["data_correction_available"], false);
    println!(
        "[region-loop-2] summary: export={}, delete={}, correction={}",
        summary_json["data_export_available"],
        summary_json["data_delete_available"],
        summary_json["data_correction_available"]
    );

    let resp_export = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/bff/compliance/export")
                .header("Authorization", auth)
                .header("X-User-Region", "EU")
                .header("Accept-Language", "en")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({"action_type": "export", "export_format": "json"})
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp_export.status(), StatusCode::FORBIDDEN);
    let export_bytes = resp_export.into_body().collect().await.unwrap().to_bytes();
    let export_json: serde_json::Value = serde_json::from_slice(&export_bytes).unwrap();
    assert_eq!(export_json["code"], "safety_blocked");
    println!(
        "[region-loop-3] export blocked: code={}",
        export_json["code"]
    );

    mock.shutdown();
}
