//! Capacity observability: health, readiness, metrics.

use axum::{routing::get, Json, Router};
use serde_json::json;

pub fn router() -> Router {
    Router::new()
        .route(
            "/health",
            get(|| async { Json(json!({"status": "alive", "service": "dm-service"})) }),
        )
        .route(
            "/ready",
            get(|| async { Json(json!({"status": "ready", "service": "dm-service"})) }),
        )
        .route(
            "/metrics",
            get(|| async { Json(json!({"service": "dm-service"})) }),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_returns_alive() {
        let app = router();
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["status"], "alive");
        println!("[health] {}", json);
    }

    #[tokio::test]
    async fn test_ready_returns_status() {
        let app = router();
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(json["status"] == "ready" || json["status"] == "degraded");
        println!("[ready] {}", json);
    }

    #[tokio::test]
    async fn test_metrics_returns_service() {
        let app = router();
        let resp = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["service"], "dm-service");
        println!("[metrics] {}", json);
    }
}
