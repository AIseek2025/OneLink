use axum::body::Body;
use axum::http::StatusCode;
use http_body_util::BodyExt;
use match_service::config::Config;
use match_service::http::routes::{router, MatchState};
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
async fn create_find_request_returns_completed() {
    let config = test_config();
    let state = MatchState::new(config, None);
    let app = router(state);

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/match/find-requests")
        .header("content-type", "application/json")
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::from(
            serde_json::json!({
                "user_id": "u-1",
                "raw_query": "找创业合伙人",
                "intent_tags": ["cofounder"]
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body();
    let bytes = body.collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["status"], "completed");
    assert!(!json["find_request_id"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn create_find_request_rejects_empty_query() {
    let config = test_config();
    let state = MatchState::new(config, None);
    let app = router(state);

    let request = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/match/find-requests")
        .header("content-type", "application/json")
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::from(
            serde_json::json!({
                "user_id": "u-1",
                "raw_query": "   "
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn get_find_request_after_create() {
    let config = test_config();
    let state = MatchState::new(config, None);
    let app = router(state);

    let create_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/match/find-requests")
        .header("content-type", "application/json")
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::from(
            serde_json::json!({
                "user_id": "u-2",
                "raw_query": "找技术合伙人"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(create_req).await.unwrap();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let fr_id = json["find_request_id"].as_str().unwrap();

    let get_req = axum::http::Request::builder()
        .uri(format!("/api/v1/match/find-requests/{fr_id}"))
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(get_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["find_request_id"], fr_id);
    assert_eq!(json["status"], "completed");
}

#[tokio::test]
async fn get_candidates_after_create() {
    let config = test_config();
    let state = MatchState::new(config, None);
    let app = router(state);

    let create_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/match/find-requests")
        .header("content-type", "application/json")
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::from(
            serde_json::json!({
                "user_id": "u-3",
                "raw_query": "找设计师",
                "intent_tags": ["designer"]
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(create_req).await.unwrap();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let fr_id = json["find_request_id"].as_str().unwrap();

    let cand_req = axum::http::Request::builder()
        .uri(format!("/api/v1/match/find-requests/{fr_id}/candidates"))
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(cand_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let candidates = json["candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), 5);
    for c in candidates {
        assert_eq!(c["status"], "suggested");
        assert!(c["score"].as_f64().unwrap() > 0.0);
    }
}

#[tokio::test]
async fn submit_feedback_like_updates_candidate() {
    let config = test_config();
    let state = MatchState::new(config, None);
    let app = router(state);

    let create_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/match/find-requests")
        .header("content-type", "application/json")
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::from(
            serde_json::json!({
                "user_id": "u-4",
                "raw_query": "找合伙人"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.clone().oneshot(create_req).await.unwrap();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let fr_id = json["find_request_id"].as_str().unwrap();

    let cand_req = axum::http::Request::builder()
        .uri(format!(
            "/api/v1/match/find-requests/{fr_id}/candidates?limit=1"
        ))
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(cand_req).await.unwrap();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let candidate_id = json["candidates"][0]["candidate_user_id"].as_str().unwrap();

    let fb_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/match/find-requests/{fr_id}/feedback")
        .header("content-type", "application/json")
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::from(
            serde_json::json!({
                "user_id": "u-4",
                "candidate_user_id": candidate_id,
                "action": "like"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(fb_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["action"], "like");
}

#[tokio::test]
async fn list_find_requests_filters_by_user() {
    let config = test_config();
    let state = MatchState::new(config, None);
    let app = router(state);

    let create_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/match/find-requests")
        .header("content-type", "application/json")
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::from(
            serde_json::json!({
                "user_id": "u-list-test",
                "raw_query": "找产品经理"
            })
            .to_string(),
        ))
        .unwrap();

    let _ = app.clone().oneshot(create_req).await.unwrap();

    let list_req = axum::http::Request::builder()
        .uri("/api/v1/match/find-requests?user_id=u-list-test&limit=10")
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(list_req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let frs = json["find_requests"].as_array().unwrap();
    assert!(!frs.is_empty());
    for fr in frs {
        assert_eq!(fr["user_id"], "u-list-test");
    }
}

#[tokio::test]
async fn mutual_like_creates_match_e2e() {
    let config = test_config();
    let state = MatchState::new(config, None);
    let app = router(state);

    let create_a = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/match/find-requests")
        .header("content-type", "application/json")
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::from(
            serde_json::json!({
                "user_id": "u-mutual-a",
                "raw_query": "找合伙人"
            })
            .to_string(),
        ))
        .unwrap();

    let resp_a = app.clone().oneshot(create_a).await.unwrap();
    let bytes_a = resp_a.into_body().collect().await.unwrap().to_bytes();
    let json_a: serde_json::Value = serde_json::from_slice(&bytes_a).unwrap();
    let fr_id_a = json_a["find_request_id"].as_str().unwrap();

    let cand_req_a = axum::http::Request::builder()
        .uri(format!(
            "/api/v1/match/find-requests/{fr_id_a}/candidates?limit=1"
        ))
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::empty())
        .unwrap();

    let resp_cand = app.clone().oneshot(cand_req_a).await.unwrap();
    let bytes_cand = resp_cand.into_body().collect().await.unwrap().to_bytes();
    let json_cand: serde_json::Value = serde_json::from_slice(&bytes_cand).unwrap();
    let candidate_id = json_cand["candidates"][0]["candidate_user_id"]
        .as_str()
        .unwrap();

    let like_a = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/match/find-requests/{fr_id_a}/feedback")
        .header("content-type", "application/json")
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::from(
            serde_json::json!({
                "user_id": "u-mutual-a",
                "candidate_user_id": candidate_id,
                "action": "like",
                "find_request_id": fr_id_a
            })
            .to_string(),
        ))
        .unwrap();

    let _ = app.clone().oneshot(like_a).await.unwrap();

    let like_b = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/match/find-requests/{fr_id_a}/feedback")
        .header("content-type", "application/json")
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::from(
            serde_json::json!({
                "user_id": candidate_id,
                "candidate_user_id": "u-mutual-a",
                "action": "like"
            })
            .to_string(),
        ))
        .unwrap();

    let resp_like_b = app.clone().oneshot(like_b).await.unwrap();
    let bytes_like_b = resp_like_b.into_body().collect().await.unwrap().to_bytes();
    let json_like_b: serde_json::Value = serde_json::from_slice(&bytes_like_b).unwrap();
    assert_eq!(json_like_b["match_created"], true);
    assert!(!json_like_b["match_id"].as_str().unwrap().is_empty());

    let match_id = json_like_b["match_id"].as_str().unwrap();

    let get_match_req = axum::http::Request::builder()
        .uri(format!("/api/v1/match/matches/{match_id}"))
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::empty())
        .unwrap();

    let resp_match = app.oneshot(get_match_req).await.unwrap();
    assert_eq!(resp_match.status(), StatusCode::OK);
    let bytes_match = resp_match.into_body().collect().await.unwrap().to_bytes();
    let json_match: serde_json::Value = serde_json::from_slice(&bytes_match).unwrap();
    assert_eq!(json_match["match_type"], "mutual_like");
    assert_eq!(json_match["status"], "active");
}

#[tokio::test]
async fn block_unmatches_existing_match() {
    let config = test_config();
    let state = MatchState::new(config, None);
    let app = router(state);

    let create_a = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/match/find-requests")
        .header("content-type", "application/json")
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::from(
            serde_json::json!({
                "user_id": "u-block-a",
                "raw_query": "找合伙人"
            })
            .to_string(),
        ))
        .unwrap();

    let resp_a = app.clone().oneshot(create_a).await.unwrap();
    let bytes_a = resp_a.into_body().collect().await.unwrap().to_bytes();
    let json_a: serde_json::Value = serde_json::from_slice(&bytes_a).unwrap();
    let fr_id_a = json_a["find_request_id"].as_str().unwrap();

    let cand_req = axum::http::Request::builder()
        .uri(format!(
            "/api/v1/match/find-requests/{fr_id_a}/candidates?limit=1"
        ))
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::empty())
        .unwrap();

    let resp_cand = app.clone().oneshot(cand_req).await.unwrap();
    let bytes_cand = resp_cand.into_body().collect().await.unwrap().to_bytes();
    let json_cand: serde_json::Value = serde_json::from_slice(&bytes_cand).unwrap();
    let candidate_id = json_cand["candidates"][0]["candidate_user_id"]
        .as_str()
        .unwrap();

    let like_a = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/match/find-requests/{fr_id_a}/feedback")
        .header("content-type", "application/json")
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::from(
            serde_json::json!({
                "user_id": "u-block-a",
                "candidate_user_id": candidate_id,
                "action": "like"
            })
            .to_string(),
        ))
        .unwrap();

    let _ = app.clone().oneshot(like_a).await.unwrap();

    let like_b = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/match/find-requests/{fr_id_a}/feedback")
        .header("content-type", "application/json")
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::from(
            serde_json::json!({
                "user_id": candidate_id,
                "candidate_user_id": "u-block-a",
                "action": "like"
            })
            .to_string(),
        ))
        .unwrap();

    let resp_like_b = app.clone().oneshot(like_b).await.unwrap();
    let bytes_like_b = resp_like_b.into_body().collect().await.unwrap().to_bytes();
    let json_like_b: serde_json::Value = serde_json::from_slice(&bytes_like_b).unwrap();
    let match_id = json_like_b["match_id"].as_str().unwrap();

    let block_req = axum::http::Request::builder()
        .method("POST")
        .uri("/api/v1/match/find-requests/{fr_id_a}/feedback")
        .header("content-type", "application/json")
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::from(
            serde_json::json!({
                "user_id": "u-block-a",
                "candidate_user_id": candidate_id,
                "action": "block"
            })
            .to_string(),
        ))
        .unwrap();

    let _ = app.clone().oneshot(block_req).await.unwrap();

    let get_match_req = axum::http::Request::builder()
        .uri(format!("/api/v1/match/matches/{match_id}"))
        .header(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!",
        )
        .body(Body::empty())
        .unwrap();

    let resp_match = app.oneshot(get_match_req).await.unwrap();
    assert_eq!(resp_match.status(), StatusCode::OK);
    let bytes_match = resp_match.into_body().collect().await.unwrap().to_bytes();
    let json_match: serde_json::Value = serde_json::from_slice(&bytes_match).unwrap();
    assert_eq!(json_match["status"], "unmatched");
}
