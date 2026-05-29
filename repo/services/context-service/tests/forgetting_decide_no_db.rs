//! `POST /internal/memory/forgetting/decide`：无 DATABASE_URL（in-memory store）时返回 noop（任务书 §6.2）。

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use context_service::app_state::ContextAppState;
use context_service::config::Config;
use context_service::http::routes;
use context_service::policy::PolicyConfigStore;
use context_service::store::MemoryBackend;

#[tokio::test]
async fn forgetting_decide_without_postgres_is_noop_no_database() {
    let cfg = Config {
        port: 8099,
        database_url: None,
        default_reply_style: "brief".into(),
        profile_service_base_url: "http://127.0.0.1:1".into(),
        ai_chat_service_base_url: "http://127.0.0.1:1".into(),
        internal_shared_secret: "test-internal-secret".into(),
        env_mode: "dev".into(),
        internal_bind_addr: "127.0.0.1".into(),
    };
    let state = ContextAppState::new(
        PolicyConfigStore::default(),
        cfg,
        MemoryBackend::in_memory(),
    );
    let app = routes::router(state);

    let body = serde_json::json!({
        "user_id": "00000000-0000-0000-0000-000000000001",
        "target_type": "memory_artifact",
        "target_id": "00000000-0000-0000-0000-000000000002",
        "decision": "retain",
        "reason_codes": ["unit_test"],
        "policy_version": "v1",
        "cold_storage_ref": null
    });

    let req = Request::builder()
        .method("POST")
        .uri("/internal/memory/forgetting/decide")
        .header("content-type", "application/json")
        .header("x-internal-token", "test-internal-secret")
        .body(Body::from(body.to_string()))
        .unwrap();

    let response = app.oneshot(req).await.expect("oneshot");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(v["accepted"], false);
    assert_eq!(v["persistence"], "noop_no_database");
}
