use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

const CAPABILITY_BULKHEAD_DEFAULT: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkheadConfig {
    pub capacity: usize,
}

impl Default for BulkheadConfig {
    fn default() -> Self {
        Self {
            capacity: CAPABILITY_BULKHEAD_DEFAULT,
        }
    }
}

#[derive(Debug)]
struct BulkheadSlot {
    active: AtomicU64,
    capacity: usize,
}

impl BulkheadSlot {
    fn new(capacity: usize) -> Self {
        Self {
            active: AtomicU64::new(0),
            capacity,
        }
    }

    fn try_acquire(&self) -> bool {
        loop {
            let current = self.active.load(Ordering::Relaxed);
            if current as usize >= self.capacity {
                return false;
            }
            if self
                .active
                .compare_exchange_weak(current, current + 1, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                return true;
            }
        }
    }

    fn release(&self) {
        self.active.fetch_sub(1, Ordering::Release);
    }

    fn active_count(&self) -> u64 {
        self.active.load(Ordering::Relaxed)
    }
}

#[derive(Debug)]
pub struct BulkheadGuard<'a> {
    slot: &'a BulkheadSlot,
}

impl Drop for BulkheadGuard<'_> {
    fn drop(&mut self) {
        self.slot.release();
    }
}

#[derive(Debug, Clone)]
pub struct CapabilityBulkheads {
    slots: Arc<HashMap<String, BulkheadSlot>>,
}

impl CapabilityBulkheads {
    pub fn new(configs: &HashMap<String, BulkheadConfig>) -> Self {
        let slots = configs
            .iter()
            .map(|(name, cfg)| (name.clone(), BulkheadSlot::new(cfg.capacity)))
            .collect();
        Self {
            slots: Arc::new(slots),
        }
    }

    pub fn with_defaults() -> Self {
        let mut configs = HashMap::new();
        configs.insert("chat.respond".to_string(), BulkheadConfig { capacity: 50 });
        configs.insert(
            "match.recommend".to_string(),
            BulkheadConfig { capacity: 30 },
        );
        configs.insert("safety.review".to_string(), BulkheadConfig { capacity: 20 });
        Self::new(&configs)
    }

    pub fn try_acquire(&self, capability: &str) -> Option<BulkheadGuard<'_>> {
        self.slots.get(capability).and_then(|slot| {
            if slot.try_acquire() {
                Some(BulkheadGuard { slot })
            } else {
                None
            }
        })
    }

    pub fn active_counts(&self) -> HashMap<String, u64> {
        self.slots
            .iter()
            .map(|(name, slot)| (name.clone(), slot.active_count()))
            .collect()
    }

    pub fn is_at_capacity(&self, capability: &str) -> bool {
        self.slots
            .get(capability)
            .map(|slot| slot.active_count() as usize >= slot.capacity)
            .unwrap_or(true)
    }

    pub fn snapshot_all(&self) -> serde_json::Map<String, serde_json::Value> {
        let counts = self.active_counts();
        let mut map = serde_json::Map::new();
        for (name, active) in counts {
            let capacity = self.slots.get(&name).map(|s| s.capacity).unwrap_or(0);
            map.insert(
                name,
                serde_json::json!({
                    "active": active,
                    "capacity": capacity,
                    "utilization": if capacity > 0 { active as f64 / capacity as f64 } else { 0.0 },
                }),
            );
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bulkhead_allows_within_capacity() {
        let configs = HashMap::from([("test.cap".to_string(), BulkheadConfig { capacity: 2 })]);
        let bh = CapabilityBulkheads::new(&configs);
        let g1 = bh.try_acquire("test.cap");
        assert!(g1.is_some());
        let g2 = bh.try_acquire("test.cap");
        assert!(g2.is_some());
    }

    #[test]
    fn bulkhead_rejects_at_capacity() {
        let configs = HashMap::from([("test.cap".to_string(), BulkheadConfig { capacity: 1 })]);
        let bh = CapabilityBulkheads::new(&configs);
        let _g1 = bh.try_acquire("test.cap");
        let g2 = bh.try_acquire("test.cap");
        assert!(g2.is_none());
    }

    #[test]
    fn bulkhead_releases_on_drop() {
        let configs = HashMap::from([("test.cap".to_string(), BulkheadConfig { capacity: 1 })]);
        let bh = CapabilityBulkheads::new(&configs);
        {
            let _g1 = bh.try_acquire("test.cap");
        }
        let g2 = bh.try_acquire("test.cap");
        assert!(g2.is_some());
    }

    #[test]
    fn bulkhead_unknown_capability_rejected() {
        let bh = CapabilityBulkheads::with_defaults();
        let g = bh.try_acquire("unknown.cap");
        assert!(g.is_none());
    }

    #[test]
    fn default_bulkheads_include_chat_match_safety() {
        let bh = CapabilityBulkheads::with_defaults();
        assert!(bh.try_acquire("chat.respond").is_some());
        assert!(bh.try_acquire("match.recommend").is_some());
        assert!(bh.try_acquire("safety.review").is_some());
    }
}
