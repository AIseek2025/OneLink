//! In-memory store (legacy dev path).

use std::collections::HashMap;
use std::sync::Mutex;

use crate::{
    app_state::MemoryArtifactRecord,
    memory_store::{
        MemoryEntityLinkRecord, MemoryEntityRecord, MemorySummaryRecord, RuntimeCheckpointRecord,
    },
    store::{ObsCounts, UserMemorySnapshot},
};

#[derive(Debug)]
pub struct InMemoryStore {
    pub artifacts: Mutex<HashMap<String, MemoryArtifactRecord>>,
    pub summaries: Mutex<HashMap<String, MemorySummaryRecord>>,
    pub entities: Mutex<HashMap<String, MemoryEntityRecord>>,
    pub entity_links: Mutex<HashMap<String, MemoryEntityLinkRecord>>,
    pub checkpoints: Mutex<HashMap<String, RuntimeCheckpointRecord>>,
    pub checkpoint_request_index: Mutex<HashMap<String, String>>,
    pub consolidate_event_index: Mutex<HashMap<String, String>>,
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            artifacts: Mutex::new(HashMap::new()),
            summaries: Mutex::new(HashMap::new()),
            entities: Mutex::new(HashMap::new()),
            entity_links: Mutex::new(HashMap::new()),
            checkpoints: Mutex::new(HashMap::new()),
            checkpoint_request_index: Mutex::new(HashMap::new()),
            consolidate_event_index: Mutex::new(HashMap::new()),
        }
    }

    pub fn insert_artifact(&self, rec: MemoryArtifactRecord) {
        let mut g = self.artifacts.lock().expect("artifacts mutex poisoned");
        g.insert(rec.memory_id.clone(), rec);
    }

    pub fn insert_summary(&self, rec: MemorySummaryRecord) {
        let mut g = self.summaries.lock().expect("summaries mutex poisoned");
        g.insert(rec.summary_id.clone(), rec);
    }

    pub fn upsert_entity(&self, rec: MemoryEntityRecord) {
        let mut g = self.entities.lock().expect("entities mutex poisoned");
        let id = rec.id.clone();
        g.entry(id).or_insert(rec);
    }

    pub fn upsert_entity_link(&self, rec: MemoryEntityLinkRecord) {
        let mut g = self
            .entity_links
            .lock()
            .expect("entity_links mutex poisoned");
        let id = rec.id.clone();
        g.entry(id).or_insert(rec);
    }

    pub fn user_snapshot(&self, user_id: &str) -> UserMemorySnapshot {
        let summaries = self.summaries.lock().expect("summaries mutex poisoned");
        let artifacts = self.artifacts.lock().expect("artifacts mutex poisoned");
        let entities = self.entities.lock().expect("entities mutex poisoned");
        let entity_links = self
            .entity_links
            .lock()
            .expect("entity_links mutex poisoned");
        UserMemorySnapshot {
            summaries: summaries
                .iter()
                .filter(|(_, s)| s.user_id == user_id)
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            artifacts: artifacts
                .iter()
                .filter(|(_, a)| a.user_id == user_id)
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            entities: entities
                .iter()
                .filter(|(_, e)| e.user_id == user_id)
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            entity_links: entity_links
                .iter()
                .filter(|(_, l)| l.user_id == user_id)
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        }
    }

    pub fn get_artifacts_by_ids(&self, ids: &[String]) -> Vec<MemoryArtifactRecord> {
        let g = self.artifacts.lock().expect("artifacts mutex poisoned");
        ids.iter().filter_map(|id| g.get(id).cloned()).collect()
    }

    pub fn get_artifact(&self, id: &str) -> Option<MemoryArtifactRecord> {
        let g = self.artifacts.lock().expect("artifacts mutex poisoned");
        g.get(id).cloned()
    }

    pub fn touch_artifacts(&self, ids: &[String], touched_at: &str) {
        let mut g = self.artifacts.lock().expect("artifacts mutex poisoned");
        for id in ids {
            if let Some(artifact) = g.get_mut(id) {
                artifact.access_count = artifact.access_count.saturating_add(1);
                artifact.last_accessed_at = touched_at.to_string();
            }
        }
    }

    pub fn checkpoint_lookup(&self, dedupe_key: &str) -> Option<String> {
        let g = self
            .checkpoint_request_index
            .lock()
            .expect("checkpoint_request_index mutex poisoned");
        g.get(dedupe_key).cloned()
    }

    pub fn checkpoint_insert(&self, dedupe_key: &str, rec: RuntimeCheckpointRecord) -> String {
        let idx = self
            .checkpoint_request_index
            .lock()
            .expect("checkpoint_request_index mutex poisoned");
        if let Some(existing) = idx.get(dedupe_key).cloned() {
            return existing;
        }
        let id = rec.checkpoint_id.clone();
        drop(idx);
        let mut cps = self.checkpoints.lock().expect("checkpoints mutex poisoned");
        cps.insert(id.clone(), rec);
        drop(cps);
        let mut idx = self
            .checkpoint_request_index
            .lock()
            .expect("checkpoint_request_index mutex poisoned");
        idx.insert(dedupe_key.to_string(), id.clone());
        id
    }

    pub fn consolidate_has_event(&self, event_id: &str) -> bool {
        let g = self
            .consolidate_event_index
            .lock()
            .expect("consolidate_event_index mutex poisoned");
        g.contains_key(event_id)
    }

    pub fn consolidate_record(&self, event_id: &str, summary_id: &str) {
        let mut g = self
            .consolidate_event_index
            .lock()
            .expect("consolidate_event_index mutex poisoned");
        g.insert(event_id.to_string(), summary_id.to_string());
    }

    pub fn observability_counts(&self) -> ObsCounts {
        let artifact_count = self
            .artifacts
            .lock()
            .expect("artifacts mutex poisoned")
            .len();
        let (summary_count, latest_summary_policy_version) = {
            let summaries = self.summaries.lock().expect("summaries mutex poisoned");
            let count = summaries.len();
            let latest = summaries
                .values()
                .max_by(|a, b| a.updated_at.cmp(&b.updated_at))
                .map(|s| s.policy_version.clone());
            (count, latest)
        };
        let entity_count = self.entities.lock().expect("entities mutex poisoned").len();
        let link_count = self
            .entity_links
            .lock()
            .expect("entity_links mutex poisoned")
            .len();
        let checkpoint_count = self
            .checkpoints
            .lock()
            .expect("checkpoints mutex poisoned")
            .len();
        ObsCounts {
            artifact_count,
            summary_count,
            entity_count,
            link_count,
            checkpoint_count,
            latest_summary_policy_version,
        }
    }
}
