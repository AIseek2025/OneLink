use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u64,
    pub recovery_timeout_secs: u64,
    pub half_open_max_calls: u64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout_secs: 30,
            half_open_max_calls: 3,
        }
    }
}

#[derive(Debug)]
struct CircuitBreakerInner {
    state: RwLock<CircuitState>,
    failure_count: AtomicU64,
    success_count: AtomicU64,
    last_failure_time: RwLock<Option<Instant>>,
    half_open_calls: AtomicU64,
    config: CircuitBreakerConfig,
}

#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    inner: Arc<CircuitBreakerInner>,
    name: String,
}

impl CircuitBreaker {
    pub fn new(name: String, config: CircuitBreakerConfig) -> Self {
        Self {
            inner: Arc::new(CircuitBreakerInner {
                state: RwLock::new(CircuitState::Closed),
                failure_count: AtomicU64::new(0),
                success_count: AtomicU64::new(0),
                last_failure_time: RwLock::new(None),
                half_open_calls: AtomicU64::new(0),
                config,
            }),
            name,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub async fn is_available(&self) -> bool {
        let state = *self.inner.state.read().await;
        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                let last_failure = *self.inner.last_failure_time.read().await;
                if let Some(t) = last_failure {
                    if t.elapsed() >= Duration::from_secs(self.inner.config.recovery_timeout_secs) {
                        let mut state_guard = self.inner.state.write().await;
                        *state_guard = CircuitState::HalfOpen;
                        self.inner.half_open_calls.store(0, Ordering::SeqCst);
                        return true;
                    }
                }
                false
            }
            CircuitState::HalfOpen => {
                let current = self.inner.half_open_calls.load(Ordering::SeqCst);
                current < self.inner.config.half_open_max_calls
            }
        }
    }

    pub async fn record_success(&self) {
        let state = *self.inner.state.read().await;
        self.inner.success_count.fetch_add(1, Ordering::Relaxed);
        if state == CircuitState::HalfOpen {
            let mut state_guard = self.inner.state.write().await;
            *state_guard = CircuitState::Closed;
            self.inner.failure_count.store(0, Ordering::SeqCst);
        }
    }

    pub async fn record_failure(&self) {
        self.inner.failure_count.fetch_add(1, Ordering::SeqCst);
        *self.inner.last_failure_time.write().await = Some(Instant::now());

        let state = *self.inner.state.read().await;
        if state == CircuitState::HalfOpen {
            let mut state_guard = self.inner.state.write().await;
            *state_guard = CircuitState::Open;
            return;
        }

        let failures = self.inner.failure_count.load(Ordering::SeqCst);
        if failures >= self.inner.config.failure_threshold {
            let mut state_guard = self.inner.state.write().await;
            *state_guard = CircuitState::Open;
        }
    }

    pub async fn state(&self) -> CircuitState {
        *self.inner.state.read().await
    }

    pub fn failure_count(&self) -> u64 {
        self.inner.failure_count.load(Ordering::Relaxed)
    }

    pub fn success_count(&self) -> u64 {
        self.inner.success_count.load(Ordering::Relaxed)
    }
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerRegistry {
    breakers: Arc<Vec<CircuitBreaker>>,
}

impl CircuitBreakerRegistry {
    pub fn with_default_capabilities() -> Self {
        let breakers = vec![
            CircuitBreaker::new("chat.respond".to_string(), CircuitBreakerConfig::default()),
            CircuitBreaker::new(
                "match.recommend".to_string(),
                CircuitBreakerConfig::default(),
            ),
            CircuitBreaker::new("safety.review".to_string(), CircuitBreakerConfig::default()),
        ];
        Self {
            breakers: Arc::new(breakers),
        }
    }

    pub fn get(&self, capability: &str) -> Option<&CircuitBreaker> {
        self.breakers.iter().find(|b| b.name() == capability)
    }

    pub fn all(&self) -> &[CircuitBreaker] {
        &self.breakers
    }
}

#[derive(Debug, Serialize)]
pub struct CircuitBreakerStatus {
    pub capability: String,
    pub state: CircuitState,
    pub failure_count: u64,
    pub success_count: u64,
}

impl CircuitBreakerRegistry {
    pub async fn status_snapshot(&self) -> Vec<CircuitBreakerStatus> {
        let mut result = Vec::with_capacity(self.breakers.len());
        for b in self.breakers.iter() {
            result.push(CircuitBreakerStatus {
                capability: b.name().to_string(),
                state: b.state().await,
                failure_count: b.failure_count(),
                success_count: b.success_count(),
            });
        }
        result
    }

    pub async fn snapshot_all(&self) -> serde_json::Map<String, serde_json::Value> {
        let statuses = self.status_snapshot().await;
        let mut map = serde_json::Map::new();
        for s in statuses {
            map.insert(
                s.capability,
                serde_json::json!({
                    "state": s.state,
                    "failures": s.failure_count,
                    "successes": s.success_count,
                }),
            );
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn circuit_starts_closed() {
        let cb = CircuitBreaker::new("test".to_string(), CircuitBreakerConfig::default());
        assert_eq!(cb.state().await, CircuitState::Closed);
        assert!(cb.is_available().await);
    }

    #[tokio::test]
    async fn circuit_opens_after_threshold() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let cb = CircuitBreaker::new("test".to_string(), config);
        for _ in 0..3 {
            cb.record_failure().await;
        }
        assert_eq!(cb.state().await, CircuitState::Open);
        assert!(!cb.is_available().await);
    }

    #[tokio::test]
    async fn circuit_half_open_after_recovery_timeout() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            recovery_timeout_secs: 0,
            ..Default::default()
        };
        let cb = CircuitBreaker::new("test".to_string(), config);
        cb.record_failure().await;
        assert_eq!(cb.state().await, CircuitState::Open);
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(cb.is_available().await);
        assert_eq!(cb.state().await, CircuitState::HalfOpen);
    }

    #[tokio::test]
    async fn circuit_closes_on_half_open_success() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            recovery_timeout_secs: 0,
            ..Default::default()
        };
        let cb = CircuitBreaker::new("test".to_string(), config);
        cb.record_failure().await;
        tokio::time::sleep(Duration::from_millis(10)).await;
        let _ = cb.is_available().await;
        cb.record_success().await;
        assert_eq!(cb.state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn circuit_reopens_on_half_open_failure() {
        let config = CircuitBreakerConfig {
            failure_threshold: 1,
            recovery_timeout_secs: 0,
            ..Default::default()
        };
        let cb = CircuitBreaker::new("test".to_string(), config);
        cb.record_failure().await;
        tokio::time::sleep(Duration::from_millis(10)).await;
        let _ = cb.is_available().await;
        cb.record_failure().await;
        assert_eq!(cb.state().await, CircuitState::Open);
    }

    #[tokio::test]
    async fn registry_has_three_capabilities() {
        let reg = CircuitBreakerRegistry::with_default_capabilities();
        assert!(reg.get("chat.respond").is_some());
        assert!(reg.get("match.recommend").is_some());
        assert!(reg.get("safety.review").is_some());
    }
}
