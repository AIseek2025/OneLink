//! BFF Capacity observability: health, readiness, metrics.

use axum::{routing::get, Json, Router};
use onelink_capacity_metrics::CapacityMetrics;
use serde_json::json;
use std::sync::Arc;

pub struct BffMetrics {
    pub capacity: CapacityMetrics,
}

impl Default for BffMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl BffMetrics {
    pub fn new() -> Self {
        Self {
            capacity: CapacityMetrics::new("bff"),
        }
    }
}

pub fn router(metrics: Arc<BffMetrics>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/metrics", get(metrics_handler))
        .with_state(metrics)
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "alive",
        "service": "bff"
    }))
}

async fn ready() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ready",
        "service": "bff",
        "downstream_check": "not_configured_in_dev"
    }))
}

async fn metrics_handler(
    axum::extract::State(metrics): axum::extract::State<Arc<BffMetrics>>,
) -> Json<serde_json::Value> {
    let capacity = metrics.capacity.snapshot();
    Json(json!({
        "service": "bff",
        "aggregation_layer": true,
        "downstream_services": [
            "identity-service",
            "profile-service",
            "ai-chat-service",
            "context-service",
            "match-service",
            "dm-service",
            "safety-service",
            "model-gateway"
        ],
        "capacity_metrics": capacity,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn test_metrics() -> Arc<BffMetrics> {
        Arc::new(BffMetrics::new())
    }

    #[tokio::test]
    async fn test_health_returns_alive() {
        let metrics = test_metrics();
        let app = router(metrics);
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
        assert_eq!(json["service"], "bff");
        println!(
            "[health] status={}, service={}",
            json["status"], json["service"]
        );
    }

    #[tokio::test]
    async fn test_ready_returns_status() {
        let metrics = test_metrics();
        let app = router(metrics);
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
        println!(
            "[ready] status={}, service={}",
            json["status"], json["service"]
        );
    }

    #[tokio::test]
    async fn test_metrics_returns_capacity_info() {
        let metrics = test_metrics();
        metrics.capacity.record_request();
        metrics.capacity.record_request();
        metrics.capacity.record_error();
        metrics.capacity.record_latency_us(150);
        metrics.capacity.update_percentiles(120, 180, 200);
        let app = router(metrics.clone());
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
        assert_eq!(json["service"], "bff");
        assert!(json["downstream_services"].is_array());
        assert!(json["capacity_metrics"].is_object());
        let cm = json["capacity_metrics"].as_object().unwrap();
        assert_eq!(cm["request_count"], 2);
        assert_eq!(cm["error_count"], 1);
        println!(
            "[metrics] service={}, request_count={}, error_count={}, error_rate={}",
            json["service"], cm["request_count"], cm["error_count"], cm["error_rate"]
        );
    }
}
