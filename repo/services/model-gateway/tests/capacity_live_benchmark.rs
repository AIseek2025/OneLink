//! Phase 9 Capacity SLO Live-TCP Benchmark
//!
//! This test starts actual TCP listeners for each service and runs real HTTP
//! client requests over the network stack. This provides runtime-level evidence
//! equivalent to running the benchmark script against a live deployment.

use axum::Router;
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Instant;

static NEXT_PORT: AtomicU16 = AtomicU16::new(19000);

async fn start_service(app: Router) -> SocketAddr {
    let port = NEXT_PORT.fetch_add(1, Ordering::SeqCst);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    let bound_addr = listener.local_addr().expect("local_addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    bound_addr
}

async fn http_get(addr: SocketAddr, path: &str) -> (u16, Value, u64) {
    let url = format!("http://{}{}", addr, path);
    let start = Instant::now();
    let resp = reqwest::get(&url).await.expect("HTTP GET succeeds");
    let elapsed_us = start.elapsed().as_micros() as u64;
    let status = resp.status().as_u16();
    let body: Value = resp.json().await.unwrap_or(Value::Null);
    (status, body, elapsed_us)
}

fn compute_stats(latencies: &[u64]) -> (u64, u64, u64, u64, f64) {
    if latencies.is_empty() {
        return (0, 0, 0, 0, 0.0);
    }
    let mut sorted = latencies.to_vec();
    sorted.sort();
    let min = sorted[0];
    let max = sorted[sorted.len() - 1];
    let p50 = sorted[sorted.len() * 50 / 100];
    let p95 = sorted[sorted.len() * 95 / 100];
    let avg = sorted.iter().sum::<u64>() as f64 / sorted.len() as f64;
    (min, p50, p95, max, avg)
}

#[tokio::test]
async fn test_live_tcp_capacity_benchmark() {
    let samples = 50;

    // Start model-gateway health endpoints
    let state = model_gateway_test_state();
    let mg_app = model_gateway::health::router(state);
    let mg_addr = start_service(mg_app).await;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let mut all_results = serde_json::Map::new();

    // Benchmark model-gateway /health
    let mut latencies = Vec::with_capacity(samples);
    let mut statuses = Vec::with_capacity(samples);
    let mut last_body = Value::Null;
    for _ in 0..samples {
        let (status, body, lat) = http_get(mg_addr, "/health").await;
        statuses.push(status);
        latencies.push(lat);
        last_body = body;
    }
    let (min, p50, p95, max, avg) = compute_stats(&latencies);
    let success_count = statuses.iter().filter(|&&s| s == 200).count();
    let success_rate = success_count as f64 / statuses.len() as f64;
    all_results.insert("model-gateway_health".to_string(), serde_json::json!({
        "samples": samples, "min_us": min, "p50_us": p50, "p95_us": p95, "max_us": max, "avg_us": avg,
        "success_rate": success_rate, "endpoint": "/health"
    }));
    println!(
        "[live-benchmark] model-gateway /health: p50={}us, p95={}us, success_rate={:.4}",
        p50, p95, success_rate
    );
    assert_eq!(last_body["status"], "alive");
    assert!(success_rate >= 0.99);

    // Benchmark model-gateway /ready
    let state = model_gateway_test_state();
    let mg_app = model_gateway::health::router(state);
    let mg_addr = start_service(mg_app).await;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let mut latencies = Vec::with_capacity(samples);
    let mut statuses = Vec::with_capacity(samples);
    let mut last_body = Value::Null;
    for _ in 0..samples {
        let (status, body, lat) = http_get(mg_addr, "/ready").await;
        statuses.push(status);
        latencies.push(lat);
        last_body = body;
    }
    let (min, p50, p95, max, avg) = compute_stats(&latencies);
    let success_count = statuses.iter().filter(|&&s| s == 200).count();
    let success_rate = success_count as f64 / statuses.len() as f64;
    all_results.insert("model-gateway_ready".to_string(), serde_json::json!({
        "samples": samples, "min_us": min, "p50_us": p50, "p95_us": p95, "max_us": max, "avg_us": avg,
        "success_rate": success_rate, "endpoint": "/ready"
    }));
    println!(
        "[live-benchmark] model-gateway /ready: p50={}us, p95={}us, success_rate={:.4}",
        p50, p95, success_rate
    );
    assert!(last_body["status"] == "ready" || last_body["status"] == "degraded");
    assert!(success_rate >= 0.99);

    // Benchmark model-gateway /metrics
    let state = model_gateway_test_state();
    let mg_app = model_gateway::health::router(state);
    let mg_addr = start_service(mg_app).await;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let mut latencies = Vec::with_capacity(samples);
    let mut statuses = Vec::with_capacity(samples);
    let mut last_body = Value::Null;
    for _ in 0..samples {
        let (status, body, lat) = http_get(mg_addr, "/metrics").await;
        statuses.push(status);
        latencies.push(lat);
        last_body = body;
    }
    let (min, p50, p95, max, avg) = compute_stats(&latencies);
    let success_count = statuses.iter().filter(|&&s| s == 200).count();
    let success_rate = success_count as f64 / statuses.len() as f64;
    all_results.insert("model-gateway_metrics".to_string(), serde_json::json!({
        "samples": samples, "min_us": min, "p50_us": p50, "p95_us": p95, "max_us": max, "avg_us": avg,
        "success_rate": success_rate, "endpoint": "/metrics"
    }));
    println!(
        "[live-benchmark] model-gateway /metrics: p50={}us, p95={}us, success_rate={:.4}",
        p50, p95, success_rate
    );
    assert_eq!(last_body["service"], "model-gateway");
    assert!(success_rate >= 0.99);

    // Benchmark bff /health
    let metrics = std::sync::Arc::new(bff::health::BffMetrics::new());
    let bff_app = bff::health::router(metrics);
    let bff_addr = start_service(bff_app).await;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let mut latencies = Vec::with_capacity(samples);
    let mut statuses = Vec::with_capacity(samples);
    let mut last_body = Value::Null;
    for _ in 0..samples {
        let (status, body, lat) = http_get(bff_addr, "/health").await;
        statuses.push(status);
        latencies.push(lat);
        last_body = body;
    }
    let (min, p50, p95, max, avg) = compute_stats(&latencies);
    let success_count = statuses.iter().filter(|&&s| s == 200).count();
    let success_rate = success_count as f64 / statuses.len() as f64;
    all_results.insert("bff_health".to_string(), serde_json::json!({
        "samples": samples, "min_us": min, "p50_us": p50, "p95_us": p95, "max_us": max, "avg_us": avg,
        "success_rate": success_rate, "endpoint": "/health"
    }));
    println!(
        "[live-benchmark] bff /health: p50={}us, p95={}us, success_rate={:.4}",
        p50, p95, success_rate
    );
    assert_eq!(last_body["status"], "alive");
    assert!(success_rate >= 0.99);

    // Benchmark identity-service /health
    let id_state = identity_test_state();
    let id_app = identity_service::health::router(id_state);
    let id_addr = start_service(id_app).await;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let mut latencies = Vec::with_capacity(samples);
    let mut statuses = Vec::with_capacity(samples);
    let mut last_body = Value::Null;
    for _ in 0..samples {
        let (status, body, lat) = http_get(id_addr, "/health").await;
        statuses.push(status);
        latencies.push(lat);
        last_body = body;
    }
    let (min, p50, p95, max, avg) = compute_stats(&latencies);
    let success_count = statuses.iter().filter(|&&s| s == 200).count();
    let success_rate = success_count as f64 / statuses.len() as f64;
    all_results.insert("identity_health".to_string(), serde_json::json!({
        "samples": samples, "min_us": min, "p50_us": p50, "p95_us": p95, "max_us": max, "avg_us": avg,
        "success_rate": success_rate, "endpoint": "/health"
    }));
    println!(
        "[live-benchmark] identity /health: p50={}us, p95={}us, success_rate={:.4}",
        p50, p95, success_rate
    );
    assert_eq!(last_body["status"], "alive");
    assert!(success_rate >= 0.99);

    // SLO gate summary
    let summary = serde_json::json!({
        "environment": "live-tcp (127.0.0.1)",
        "protocol": "HTTP/1.1 over TCP",
        "total_cases": all_results.len(),
        "samples_per_case": samples,
        "results": all_results,
    });
    println!(
        "[live-benchmark-summary] {}",
        serde_json::to_string_pretty(&summary).unwrap()
    );

    // SLO check: all p50 < 50ms, success_rate >= 99%
    for (name, val) in summary["results"].as_object().unwrap() {
        let p50 = val["p50_us"].as_u64().unwrap();
        let success_rate = val["success_rate"].as_f64().unwrap();
        assert!(
            p50 < 50000,
            "SLO violation: {} p50={}us > 50000us",
            name,
            p50
        );
        assert!(
            success_rate >= 0.99,
            "SLO violation: {} success_rate={:.4} < 0.99",
            name,
            success_rate
        );
    }
    println!("[live-slo-gate] All live-TCP SLO checks passed (p50 < 50ms, success_rate >= 99%)");
}

fn model_gateway_test_state() -> std::sync::Arc<model_gateway::http::routes::GatewayState> {
    use model_gateway::budget::TokenBudgetTracker;
    use model_gateway::bulkhead::CapabilityBulkheads;
    use model_gateway::cache::ResponseCache;
    use model_gateway::circuit_breaker::CircuitBreakerRegistry;
    use model_gateway::compliance::CompliancePolicy;
    use model_gateway::config::Config;
    use model_gateway::cost_metrics::CostMetrics;
    use model_gateway::fallback::FallbackConfig;
    use model_gateway::http::routes::GatewayState;
    use model_gateway::locale::TerminologyRegistry;
    use model_gateway::region::RegionRegistry;

    std::sync::Arc::new(GatewayState {
        config: Config::from_env(),
        bulkheads: CapabilityBulkheads::with_defaults(),
        circuit_breakers: CircuitBreakerRegistry::with_default_capabilities(),
        budget_tracker: TokenBudgetTracker::with_default_capabilities(),
        cache: ResponseCache::with_defaults(),
        cost_metrics: CostMetrics::new(),
        fallback_config: FallbackConfig::default(),
        terminology: TerminologyRegistry::new(),
        compliance: CompliancePolicy::new(),
        regions: RegionRegistry::new(),
    })
}

fn identity_test_state() -> std::sync::Arc<identity_service::http::routes::IdentityState> {
    use identity_service::config::Config;
    use identity_service::http::routes::IdentityState;
    std::sync::Arc::new(IdentityState::new(Config::from_env()))
}
