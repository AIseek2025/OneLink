use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde::Serialize;

#[derive(Debug, Clone, Copy)]
pub enum CostEventKind {
    ModelInvocation,
    CacheHit,
    FallbackUsed,
    BudgetExceeded,
    CircuitBreakerOpen,
    BulkheadRejected,
}

impl CostEventKind {
    #[allow(dead_code)]
    fn as_str(&self) -> &'static str {
        match self {
            Self::ModelInvocation => "model_invocation",
            Self::CacheHit => "cache_hit",
            Self::FallbackUsed => "fallback_used",
            Self::BudgetExceeded => "budget_exceeded",
            Self::CircuitBreakerOpen => "circuit_breaker_open",
            Self::BulkheadRejected => "bulkhead_rejected",
        }
    }
}

#[derive(Debug)]
struct CostCounter {
    model_invocation: AtomicU64,
    cache_hit: AtomicU64,
    fallback_used: AtomicU64,
    budget_exceeded: AtomicU64,
    circuit_breaker_open: AtomicU64,
    bulkhead_rejected: AtomicU64,
    total_tokens_consumed: AtomicU64,
}

#[derive(Debug, Clone)]
pub struct CostMetrics {
    counters: Arc<CostCounter>,
}

impl CostMetrics {
    pub fn new() -> Self {
        Self {
            counters: Arc::new(CostCounter {
                model_invocation: AtomicU64::new(0),
                cache_hit: AtomicU64::new(0),
                fallback_used: AtomicU64::new(0),
                budget_exceeded: AtomicU64::new(0),
                circuit_breaker_open: AtomicU64::new(0),
                bulkhead_rejected: AtomicU64::new(0),
                total_tokens_consumed: AtomicU64::new(0),
            }),
        }
    }

    pub fn record(&self, kind: CostEventKind) {
        match kind {
            CostEventKind::ModelInvocation => {
                self.counters
                    .model_invocation
                    .fetch_add(1, Ordering::Relaxed);
            }
            CostEventKind::CacheHit => {
                self.counters.cache_hit.fetch_add(1, Ordering::Relaxed);
            }
            CostEventKind::FallbackUsed => {
                self.counters.fallback_used.fetch_add(1, Ordering::Relaxed);
            }
            CostEventKind::BudgetExceeded => {
                self.counters
                    .budget_exceeded
                    .fetch_add(1, Ordering::Relaxed);
            }
            CostEventKind::CircuitBreakerOpen => {
                self.counters
                    .circuit_breaker_open
                    .fetch_add(1, Ordering::Relaxed);
            }
            CostEventKind::BulkheadRejected => {
                self.counters
                    .bulkhead_rejected
                    .fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    pub fn record_tokens(&self, tokens: u64) {
        self.counters
            .total_tokens_consumed
            .fetch_add(tokens, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> CostMetricsSnapshot {
        CostMetricsSnapshot {
            model_invocation: self.counters.model_invocation.load(Ordering::Relaxed),
            cache_hit: self.counters.cache_hit.load(Ordering::Relaxed),
            fallback_used: self.counters.fallback_used.load(Ordering::Relaxed),
            budget_exceeded: self.counters.budget_exceeded.load(Ordering::Relaxed),
            circuit_breaker_open: self.counters.circuit_breaker_open.load(Ordering::Relaxed),
            bulkhead_rejected: self.counters.bulkhead_rejected.load(Ordering::Relaxed),
            total_tokens_consumed: self.counters.total_tokens_consumed.load(Ordering::Relaxed),
        }
    }
}

impl Default for CostMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Serialize)]
pub struct CostMetricsSnapshot {
    pub model_invocation: u64,
    pub cache_hit: u64,
    pub fallback_used: u64,
    pub budget_exceeded: u64,
    pub circuit_breaker_open: u64,
    pub bulkhead_rejected: u64,
    pub total_tokens_consumed: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_metrics_starts_at_zero() {
        let metrics = CostMetrics::new();
        let snap = metrics.snapshot();
        assert_eq!(snap.model_invocation, 0);
        assert_eq!(snap.cache_hit, 0);
        assert_eq!(snap.fallback_used, 0);
    }

    #[test]
    fn cost_metrics_records_events() {
        let metrics = CostMetrics::new();
        metrics.record(CostEventKind::ModelInvocation);
        metrics.record(CostEventKind::ModelInvocation);
        metrics.record(CostEventKind::CacheHit);
        let snap = metrics.snapshot();
        assert_eq!(snap.model_invocation, 2);
        assert_eq!(snap.cache_hit, 1);
    }

    #[test]
    fn cost_metrics_records_tokens() {
        let metrics = CostMetrics::new();
        metrics.record_tokens(100);
        metrics.record_tokens(200);
        let snap = metrics.snapshot();
        assert_eq!(snap.total_tokens_consumed, 300);
    }
}
