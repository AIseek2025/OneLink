use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub ttl_secs: u64,
    pub max_entries: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            ttl_secs: 300,
            max_entries: 1000,
        }
    }
}

struct CacheInner {
    entries: HashMap<String, (String, Instant, u64)>,
    config: CacheConfig,
}

impl std::fmt::Debug for CacheInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CacheInner")
            .field("entries_count", &self.entries.len())
            .field("config", &self.config)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct ResponseCache {
    inner: Arc<RwLock<CacheInner>>,
}

impl ResponseCache {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            inner: Arc::new(RwLock::new(CacheInner {
                entries: HashMap::new(),
                config,
            })),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(CacheConfig::default())
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        let mut inner = self.inner.write().await;
        if let Some((value, created, _hits)) = inner.entries.get(key) {
            if created.elapsed() < Duration::from_secs(inner.config.ttl_secs) {
                let value = value.clone();
                let entry = inner.entries.get_mut(key).unwrap();
                entry.2 += 1;
                return Some(value);
            } else {
                inner.entries.remove(key);
            }
        }
        None
    }

    pub async fn put(&self, key: String, value: String) {
        let mut inner = self.inner.write().await;
        if inner.entries.len() >= inner.config.max_entries {
            if let Some(oldest_key) = inner
                .entries
                .iter()
                .min_by_key(|(_, (_, created, _))| *created)
                .map(|(k, _)| k.clone())
            {
                inner.entries.remove(&oldest_key);
            }
        }
        inner.entries.insert(key, (value, Instant::now(), 0));
    }

    pub async fn invalidate(&self, key: &str) {
        let mut inner = self.inner.write().await;
        inner.entries.remove(key);
    }

    pub async fn len(&self) -> usize {
        let inner = self.inner.read().await;
        inner.entries.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    pub async fn hit_rate(&self) -> f64 {
        let inner = self.inner.read().await;
        let total_hits: u64 = inner.entries.values().map(|(_, _, h)| *h).sum();
        let total_entries = inner.entries.len() as u64;
        if total_entries + total_hits == 0 {
            0.0
        } else {
            total_hits as f64 / (total_entries + total_hits) as f64
        }
    }

    pub async fn stats(&self) -> CacheStats {
        let inner = self.inner.read().await;
        let total_hits: u64 = inner.entries.values().map(|(_, _, h)| *h).sum();
        let total_entries = inner.entries.len();
        CacheStats {
            entry_count: total_entries,
            total_hits,
            max_entries: inner.config.max_entries,
            ttl_secs: inner.config.ttl_secs,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CacheStats {
    pub entry_count: usize,
    pub total_hits: u64,
    pub max_entries: usize,
    pub ttl_secs: u64,
}

pub fn cache_key(capability: &str, payload: &serde_json::Value) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    capability.hash(&mut hasher);
    payload.to_string().hash(&mut hasher);
    format!("{capability}:{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cache_stores_and_retrieves() {
        let cache = ResponseCache::new(CacheConfig {
            ttl_secs: 60,
            max_entries: 100,
        });
        cache.put("key1".to_string(), "value1".to_string()).await;
        let result = cache.get("key1").await;
        assert_eq!(result, Some("value1".to_string()));
    }

    #[tokio::test]
    async fn cache_miss_returns_none() {
        let cache = ResponseCache::with_defaults();
        let result = cache.get("nonexistent").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn cache_evicts_oldest_at_capacity() {
        let cache = ResponseCache::new(CacheConfig {
            ttl_secs: 60,
            max_entries: 2,
        });
        cache.put("k1".to_string(), "v1".to_string()).await;
        cache.put("k2".to_string(), "v2".to_string()).await;
        cache.put("k3".to_string(), "v3".to_string()).await;
        assert!(cache.get("k1").await.is_none());
        assert!(cache.get("k2").await.is_some());
        assert!(cache.get("k3").await.is_some());
    }

    #[tokio::test]
    async fn cache_invalidate_removes_entry() {
        let cache = ResponseCache::with_defaults();
        cache.put("key1".to_string(), "value1".to_string()).await;
        cache.invalidate("key1").await;
        assert!(cache.get("key1").await.is_none());
    }

    #[test]
    fn cache_key_deterministic() {
        let payload = serde_json::json!({"context": "hello"});
        let k1 = cache_key("chat.respond", &payload);
        let k2 = cache_key("chat.respond", &payload);
        assert_eq!(k1, k2);
    }

    #[test]
    fn cache_key_differs_for_different_capability() {
        let payload = serde_json::json!({"context": "hello"});
        let k1 = cache_key("chat.respond", &payload);
        let k2 = cache_key("safety.review", &payload);
        assert_ne!(k1, k2);
    }
}
