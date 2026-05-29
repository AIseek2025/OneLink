use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use serde_json::{json, Value};

#[derive(Debug)]
struct MetricsInner {
    request_count: AtomicU64,
    error_count: AtomicU64,
    fallback_count: AtomicU64,
    degraded_count: AtomicU64,
    total_latency_us: AtomicU64,
    p50_latency_us: AtomicU64,
    p95_latency_us: AtomicU64,
    p99_latency_us: AtomicU64,
}

#[derive(Debug, Clone)]
pub struct CapacityMetrics {
    inner: Arc<MetricsInner>,
    service_name: String,
}

impl CapacityMetrics {
    pub fn new(service_name: &str) -> Self {
        Self {
            inner: Arc::new(MetricsInner {
                request_count: AtomicU64::new(0),
                error_count: AtomicU64::new(0),
                fallback_count: AtomicU64::new(0),
                degraded_count: AtomicU64::new(0),
                total_latency_us: AtomicU64::new(0),
                p50_latency_us: AtomicU64::new(0),
                p95_latency_us: AtomicU64::new(0),
                p99_latency_us: AtomicU64::new(0),
            }),
            service_name: service_name.to_string(),
        }
    }

    pub fn record_request(&self) {
        self.inner.request_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_error(&self) {
        self.inner.error_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_fallback(&self) {
        self.inner.fallback_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_degraded(&self) {
        self.inner.degraded_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_latency_us(&self, latency_us: u64) {
        self.inner
            .total_latency_us
            .fetch_add(latency_us, Ordering::Relaxed);
    }

    pub fn update_percentiles(&self, p50: u64, p95: u64, p99: u64) {
        self.inner.p50_latency_us.store(p50, Ordering::Relaxed);
        self.inner.p95_latency_us.store(p95, Ordering::Relaxed);
        self.inner.p99_latency_us.store(p99, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> Value {
        let requests = self.inner.request_count.load(Ordering::Relaxed);
        let errors = self.inner.error_count.load(Ordering::Relaxed);
        let fallbacks = self.inner.fallback_count.load(Ordering::Relaxed);
        let degraded = self.inner.degraded_count.load(Ordering::Relaxed);
        let total_latency = self.inner.total_latency_us.load(Ordering::Relaxed);
        let avg_latency_us = total_latency.checked_div(requests).unwrap_or(0);

        json!({
            "service": self.service_name,
            "request_count": requests,
            "error_count": errors,
            "error_rate": if requests > 0 { errors as f64 / requests as f64 } else { 0.0 },
            "fallback_count": fallbacks,
            "degraded_count": degraded,
            "latency_us": {
                "avg": avg_latency_us,
                "p50": self.inner.p50_latency_us.load(Ordering::Relaxed),
                "p95": self.inner.p95_latency_us.load(Ordering::Relaxed),
                "p99": self.inner.p99_latency_us.load(Ordering::Relaxed),
            },
        })
    }

    pub fn service_name(&self) -> &str {
        &self.service_name
    }
}

pub struct RequestTimer {
    start: Instant,
    metrics: CapacityMetrics,
}

impl RequestTimer {
    pub fn start(metrics: &CapacityMetrics) -> Self {
        metrics.record_request();
        Self {
            start: Instant::now(),
            metrics: metrics.clone(),
        }
    }

    pub fn succeed(self) {
        let elapsed = self.start.elapsed().as_micros() as u64;
        self.metrics.record_latency_us(elapsed);
    }

    pub fn fail(self) {
        let elapsed = self.start.elapsed().as_micros() as u64;
        self.metrics.record_latency_us(elapsed);
        self.metrics.record_error();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_snapshot() {
        let m = CapacityMetrics::new("test-service");
        m.record_request();
        m.record_request();
        m.record_error();
        m.record_fallback();
        m.record_degraded();
        m.record_latency_us(100);
        m.record_latency_us(200);
        m.update_percentiles(120, 180, 200);

        let snap = m.snapshot();
        assert_eq!(snap["service"], "test-service");
        assert_eq!(snap["request_count"], 2);
        assert_eq!(snap["error_count"], 1);
        assert_eq!(snap["fallback_count"], 1);
        assert_eq!(snap["degraded_count"], 1);
        assert_eq!(snap["latency_us"]["avg"], 150);
        assert_eq!(snap["latency_us"]["p50"], 120);
        assert_eq!(snap["latency_us"]["p95"], 180);
        assert_eq!(snap["latency_us"]["p99"], 200);
        println!(
            "[metrics-snapshot] {}",
            serde_json::to_string_pretty(&snap).unwrap()
        );
    }

    #[test]
    fn test_error_rate_calculation() {
        let m = CapacityMetrics::new("test-service");
        for _ in 0..100 {
            m.record_request();
        }
        for _ in 0..5 {
            m.record_error();
        }
        let snap = m.snapshot();
        let error_rate = snap["error_rate"].as_f64().unwrap();
        assert!((error_rate - 0.05).abs() < 0.001);
        println!(
            "[error-rate] {} requests, {} errors, rate={}",
            snap["request_count"], snap["error_count"], error_rate
        );
    }

    #[test]
    fn test_request_timer() {
        let m = CapacityMetrics::new("timer-service");
        let timer = RequestTimer::start(&m);
        timer.succeed();
        let snap = m.snapshot();
        assert_eq!(snap["request_count"], 1);
        assert_eq!(snap["error_count"], 0);
        println!(
            "[timer-success] {}",
            serde_json::to_string_pretty(&snap).unwrap()
        );

        let timer = RequestTimer::start(&m);
        timer.fail();
        let snap = m.snapshot();
        assert_eq!(snap["request_count"], 2);
        assert_eq!(snap["error_count"], 1);
        println!(
            "[timer-fail] {}",
            serde_json::to_string_pretty(&snap).unwrap()
        );
    }
}
