use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use serde_json::json;
use std::sync::Arc;

use crate::http::routes::IdentityState;

pub fn router(state: Arc<IdentityState>) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler))
        .route("/metrics", get(metrics_handler))
        .with_state(state)
}

async fn health_handler(
    State(state): State<Arc<IdentityState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if let Some(pg) = &state.pg {
        match pg.ping().await {
            Ok(()) => Ok(Json(json!({
                "status": "alive",
                "service": "identity-service",
                "backend": "postgres",
                "db_healthy": true,
            }))),
            Err(e) => Ok(Json(json!({
                "status": "alive",
                "service": "identity-service",
                "backend": "postgres",
                "db_healthy": false,
                "db_error": format!("{e}"),
            }))),
        }
    } else {
        Ok(Json(json!({
            "status": "alive",
            "service": "identity-service",
            "backend": "in-memory",
            "db_healthy": null,
        })))
    }
}

async fn ready_handler(
    State(state): State<Arc<IdentityState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let db_ready = if let Some(pg) = &state.pg {
        pg.ping().await.is_ok()
    } else {
        true
    };

    Ok(Json(json!({
        "status": if db_ready { "ready" } else { "degraded" },
        "service": "identity-service",
        "db_ready": db_ready,
    })))
}

async fn metrics_handler(
    State(state): State<Arc<IdentityState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let db_healthy = if let Some(pg) = &state.pg {
        pg.ping().await.is_ok()
    } else {
        true
    };

    Ok(Json(json!({
        "service": "identity-service",
        "backend": if state.pg.is_some() { "postgres" } else { "in-memory" },
        "db_healthy": db_healthy,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::http::routes::IdentityState;
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn test_state() -> Arc<IdentityState> {
        let config = Config::from_env();
        Arc::new(IdentityState::new(config))
    }

    #[tokio::test]
    async fn test_health_returns_alive() {
        let state = test_state();
        let app = router(state);
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["status"], "alive");
        assert_eq!(json["service"], "identity-service");
        println!(
            "[health] status={}, service={}",
            json["status"], json["service"]
        );
    }

    #[tokio::test]
    async fn test_ready_returns_status() {
        let state = test_state();
        let app = router(state);
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(json["status"] == "ready" || json["status"] == "degraded");
        assert_eq!(json["service"], "identity-service");
        println!(
            "[ready] status={}, service={}, db_ready={}",
            json["status"], json["service"], json["db_ready"]
        );
    }

    #[tokio::test]
    async fn test_metrics_returns_service_info() {
        let state = test_state();
        let app = router(state);
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["service"], "identity-service");
        assert!(json["backend"].is_string());
        assert!(json["db_healthy"].is_boolean() || json["db_healthy"].is_null());
        println!(
            "[metrics] service={}, backend={}, db_healthy={}",
            json["service"], json["backend"], json["db_healthy"]
        );
    }
}
