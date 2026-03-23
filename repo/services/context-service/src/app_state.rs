//! Shared HTTP / pipeline state (in-memory dev stores).

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use reqwest::Client;
use uuid::Uuid;

use crate::{
    config::Config,
    memory_store::{
        FailureCaseRecord, MemoryEntityLinkRecord, MemoryEntityRecord, MemorySummaryRecord,
        RoutingMetrics, RoutingObservation, RuntimeCheckpointRecord,
    },
    policy::PolicyConfigStore,
};

/// 记忆工件（供 `profile.memory_projection` 后解析文本用 dev-only internal API）。
#[derive(Debug, Clone)]
pub struct MemoryArtifactRecord {
    pub memory_id: String,
    pub user_id: String,
    pub conversation_id: String,
    pub source_message_id: String,
    pub content: String,
    pub network_type: String,
    pub evidence_type: String,
    pub memory_level: String,
    pub source_type: String,
    pub confidence: f64,
    pub keywords: Vec<String>,
    pub temporal_state: String,
    pub supersedes_previous: bool,
    /// `positive` | `negative` | `neutral`，来自 distiller，参与 L1 排序与埋点
    pub preference_polarity: String,
    pub created_at: String,
}

#[derive(Debug)]
pub struct ContextAppState {
    pub policy: PolicyConfigStore,
    pub config: Config,
    pub client: Client,
    pub artifacts: Mutex<HashMap<String, MemoryArtifactRecord>>,
    pub summaries: Mutex<HashMap<String, MemorySummaryRecord>>,
    pub entities: Mutex<HashMap<String, MemoryEntityRecord>>,
    pub entity_links: Mutex<HashMap<String, MemoryEntityLinkRecord>>,
    pub checkpoints: Mutex<HashMap<String, RuntimeCheckpointRecord>>,
    pub checkpoint_request_index: Mutex<HashMap<String, String>>,
    pub consolidate_event_index: Mutex<HashMap<String, String>>,
    pub routing_metrics: Mutex<RoutingMetrics>,
    pub failure_cases: Mutex<Vec<FailureCaseRecord>>,
}

impl ContextAppState {
    pub fn new(policy: PolicyConfigStore, config: Config) -> Arc<Self> {
        Arc::new(Self {
            policy,
            config,
            client: Client::new(),
            artifacts: Mutex::new(HashMap::new()),
            summaries: Mutex::new(HashMap::new()),
            entities: Mutex::new(HashMap::new()),
            entity_links: Mutex::new(HashMap::new()),
            checkpoints: Mutex::new(HashMap::new()),
            checkpoint_request_index: Mutex::new(HashMap::new()),
            consolidate_event_index: Mutex::new(HashMap::new()),
            routing_metrics: Mutex::new(RoutingMetrics::default()),
            failure_cases: Mutex::new(vec![]),
        })
    }

    pub fn now_string(&self) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        format!("unix-ms:{now}")
    }

    pub fn next_id(&self) -> String {
        Uuid::new_v4().to_string()
    }

    pub fn record_failure(
        &self,
        user_id: &str,
        stage: &str,
        category: &str,
        detail: impl Into<String>,
        trace_id: Option<String>,
        retryable: bool,
        attempt_count: u32,
    ) {
        let detail = detail.into();
        let record = FailureCaseRecord {
            id: self.next_id(),
            user_id: user_id.to_string(),
            stage: stage.to_string(),
            category: category.to_string(),
            detail: detail.clone(),
            trace_id,
            retryable,
            attempt_count,
            created_at: self.now_string(),
        };
        self.failure_cases
            .lock()
            .expect("failure_cases mutex poisoned")
            .push(record);
        tracing::warn!(
            user_id,
            stage,
            category,
            detail,
            retryable,
            attempt_count,
            "asmr-lite failure case recorded"
        );
    }

    pub fn record_routing(&self, observation: RoutingObservation) {
        let mut metrics = self
            .routing_metrics
            .lock()
            .expect("routing_metrics mutex poisoned");
        metrics.total_requests += 1;
        if observation.executed_route == "L1" {
            metrics.l1_requests += 1;
        }
        match observation.candidate_route.as_str() {
            "L2" => metrics.l2_candidates += 1,
            "L3" => metrics.l3_candidates += 1,
            _ => {}
        }
        if observation.degraded {
            metrics.degraded_requests += 1;
        }
        metrics.total_conflicts += observation.conflict_count as u64;
        metrics.last_observation = Some(observation);
    }
}
