use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudgetConfig {
    pub max_tokens_per_request: u64,
    pub daily_budget_tokens: u64,
}

impl Default for TokenBudgetConfig {
    fn default() -> Self {
        Self {
            max_tokens_per_request: 4096,
            daily_budget_tokens: 10_000_000,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TokenUsage {
    pub capability: String,
    pub total_tokens_consumed: u64,
    pub request_count: u64,
    pub budget_limit: u64,
}

#[derive(Debug)]
struct TokenBudgetSlot {
    consumed: AtomicU64,
    request_count: AtomicU64,
    config: TokenBudgetConfig,
}

#[derive(Debug, Clone)]
pub struct TokenBudgetTracker {
    slots: Arc<Vec<(String, TokenBudgetSlot)>>,
}

impl TokenBudgetTracker {
    pub fn with_default_capabilities() -> Self {
        let configs = vec![
            (
                "chat.respond".to_string(),
                TokenBudgetConfig {
                    max_tokens_per_request: 8192,
                    daily_budget_tokens: 5_000_000,
                },
            ),
            (
                "match.recommend".to_string(),
                TokenBudgetConfig {
                    max_tokens_per_request: 4096,
                    daily_budget_tokens: 2_000_000,
                },
            ),
            (
                "safety.review".to_string(),
                TokenBudgetConfig {
                    max_tokens_per_request: 2048,
                    daily_budget_tokens: 1_000_000,
                },
            ),
        ];
        Self::from_configs(configs)
    }

    pub fn from_configs(configs: Vec<(String, TokenBudgetConfig)>) -> Self {
        let slots = configs
            .into_iter()
            .map(|(name, config)| {
                (
                    name,
                    TokenBudgetSlot {
                        consumed: AtomicU64::new(0),
                        request_count: AtomicU64::new(0),
                        config,
                    },
                )
            })
            .collect();
        Self {
            slots: Arc::new(slots),
        }
    }

    pub fn try_consume(&self, capability: &str, tokens: u64) -> Result<u64, TokenBudgetExceeded> {
        if let Some((_, slot)) = self.slots.iter().find(|(n, _)| n == capability) {
            if tokens > slot.config.max_tokens_per_request {
                return Err(TokenBudgetExceeded::PerRequestLimit {
                    requested: tokens,
                    limit: slot.config.max_tokens_per_request,
                });
            }
            let current = slot.consumed.load(Ordering::Relaxed);
            if current + tokens > slot.config.daily_budget_tokens {
                return Err(TokenBudgetExceeded::DailyBudget {
                    requested: tokens,
                    remaining: slot.config.daily_budget_tokens.saturating_sub(current),
                });
            }
            slot.consumed.fetch_add(tokens, Ordering::SeqCst);
            slot.request_count.fetch_add(1, Ordering::SeqCst);
            let new_total = slot.consumed.load(Ordering::Relaxed);
            return Ok(new_total);
        }
        Err(TokenBudgetExceeded::UnknownCapability {
            capability: capability.to_string(),
        })
    }

    pub fn usage_snapshot(&self) -> Vec<TokenUsage> {
        self.slots
            .iter()
            .map(|(name, slot)| TokenUsage {
                capability: name.clone(),
                total_tokens_consumed: slot.consumed.load(Ordering::Relaxed),
                request_count: slot.request_count.load(Ordering::Relaxed),
                budget_limit: slot.config.daily_budget_tokens,
            })
            .collect()
    }

    pub fn snapshot(&self) -> serde_json::Map<String, serde_json::Value> {
        let usage = self.usage_snapshot();
        let mut map = serde_json::Map::new();
        for u in usage {
            let remaining = u.budget_limit.saturating_sub(u.total_tokens_consumed);
            let daily_remaining = remaining as f64 / u.budget_limit as f64;
            map.insert(
                u.capability.clone(),
                serde_json::json!({
                    "consumed": u.total_tokens_consumed,
                    "requests": u.request_count,
                    "budget_limit": u.budget_limit,
                    "remaining": remaining,
                    "daily_remaining_ratio": daily_remaining,
                }),
            );
        }
        map
    }

    pub fn get_config(&self, capability: &str) -> Option<&TokenBudgetConfig> {
        self.slots
            .iter()
            .find(|(n, _)| n == capability)
            .map(|(_, slot)| &slot.config)
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum TokenBudgetExceeded {
    PerRequestLimit { requested: u64, limit: u64 },
    DailyBudget { requested: u64, remaining: u64 },
    UnknownCapability { capability: String },
}

impl std::fmt::Display for TokenBudgetExceeded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PerRequestLimit { requested, limit } => {
                write!(
                    f,
                    "per-request token limit exceeded: requested {requested}, limit {limit}"
                )
            }
            Self::DailyBudget {
                requested,
                remaining,
            } => {
                write!(
                    f,
                    "daily token budget exceeded: requested {requested}, remaining {remaining}"
                )
            }
            Self::UnknownCapability { capability } => {
                write!(f, "unknown capability: {capability}")
            }
        }
    }
}

impl std::error::Error for TokenBudgetExceeded {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_allows_within_limits() {
        let tracker = TokenBudgetTracker::from_configs(vec![(
            "test.cap".to_string(),
            TokenBudgetConfig {
                max_tokens_per_request: 100,
                daily_budget_tokens: 1000,
            },
        )]);
        let result = tracker.try_consume("test.cap", 50);
        assert!(result.is_ok());
    }

    #[test]
    fn budget_rejects_per_request_exceed() {
        let tracker = TokenBudgetTracker::from_configs(vec![(
            "test.cap".to_string(),
            TokenBudgetConfig {
                max_tokens_per_request: 100,
                daily_budget_tokens: 1000,
            },
        )]);
        let result = tracker.try_consume("test.cap", 200);
        assert!(result.is_err());
    }

    #[test]
    fn budget_rejects_daily_exceed() {
        let tracker = TokenBudgetTracker::from_configs(vec![(
            "test.cap".to_string(),
            TokenBudgetConfig {
                max_tokens_per_request: 500,
                daily_budget_tokens: 100,
            },
        )]);
        let result = tracker.try_consume("test.cap", 50);
        assert!(result.is_ok());
        let result = tracker.try_consume("test.cap", 60);
        assert!(result.is_err());
    }

    #[test]
    fn budget_rejects_unknown_capability() {
        let tracker = TokenBudgetTracker::with_default_capabilities();
        let result = tracker.try_consume("unknown.cap", 10);
        assert!(result.is_err());
    }

    #[test]
    fn usage_snapshot_tracks_consumption() {
        let tracker = TokenBudgetTracker::from_configs(vec![(
            "test.cap".to_string(),
            TokenBudgetConfig {
                max_tokens_per_request: 500,
                daily_budget_tokens: 10000,
            },
        )]);
        tracker.try_consume("test.cap", 100).unwrap();
        tracker.try_consume("test.cap", 200).unwrap();
        let snap = tracker.usage_snapshot();
        assert_eq!(snap[0].total_tokens_consumed, 300);
        assert_eq!(snap[0].request_count, 2);
    }
}
