//! Shared HTTP / pipeline state.

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::Client;
use uuid::Uuid;

use crate::{
    config::Config,
    memory_store::{FailureCaseRecord, RoutingMetrics, RoutingObservation},
    policy::PolicyConfigStore,
    store::{FailureEventInsert, MemoryBackend},
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
    pub importance_score: f64,
    pub keywords: Vec<String>,
    pub temporal_state: String,
    pub supersedes_previous: bool,
    /// `positive` | `negative` | `neutral`，来自 distiller，参与 L1 排序与埋点
    pub preference_polarity: String,
    pub last_accessed_at: String,
    pub access_count: u32,
    pub created_at: String,
}

#[derive(Debug)]
pub struct ContextAppState {
    pub policy: PolicyConfigStore,
    pub config: Config,
    pub client: Client,
    pub store: MemoryBackend,
    pub routing_metrics: Mutex<RoutingMetrics>,
    pub failure_cases: Mutex<Vec<FailureCaseRecord>>,
}

impl ContextAppState {
    pub fn new(policy: PolicyConfigStore, config: Config, store: MemoryBackend) -> Arc<Self> {
        Arc::new(Self {
            policy,
            config,
            client: Client::new(),
            store,
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

    #[allow(clippy::too_many_arguments)]
    pub async fn record_failure(
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
            trace_id: trace_id.clone(),
            retryable,
            attempt_count,
            created_at: self.now_string(),
        };
        if self.store.is_postgres() {
            let row = FailureEventInsert {
                user_id: user_id.to_string(),
                stage: record.stage.clone(),
                category: record.category.clone(),
                detail: record.detail.clone(),
                trace_id: record.trace_id.clone(),
                retryable,
                attempt_count,
                created_at: record.created_at.clone(),
            };
            if let Err(e) = self.store.insert_failure_event(&row).await {
                tracing::warn!(error = %e, "context_failure_events insert failed (best-effort)");
            }
        } else {
            self.failure_cases
                .lock()
                .expect("failure_cases mutex poisoned")
                .push(record);
        }
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

    pub async fn record_routing(&self, user_id: &str, observation: RoutingObservation) {
        {
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
            metrics.last_observation = Some(observation.clone());
        }
        if self.store.is_postgres() {
            if let Err(e) = self
                .store
                .insert_routing_observation(user_id, &observation)
                .await
            {
                tracing::warn!(error = %e, "context_routing_observations insert failed (best-effort)");
            }
        }
    }
}

#[cfg(test)]
mod routing_metrics_tests {
    use super::*;
    use crate::config::Config;
    use crate::memory_store::RoutingObservation;
    use crate::policy::PolicyConfigStore;
    use crate::store::MemoryBackend;

    fn obs_l1(degraded: bool, conflicts: usize) -> RoutingObservation {
        RoutingObservation {
            executed_route: "L1".into(),
            candidate_route: "L1".into(),
            escalation_reasons: vec![],
            upgraded: false,
            evidence_count: 1,
            summary_hits: 1,
            artifact_hits: 0,
            entity_hits: 0,
            conflict_count: conflicts,
            route_confidence: 0.5,
            estimated_llm_calls: 0,
            estimated_tokens: 10,
            query_preview: "p".into(),
            degraded,
            elapsed_ms: 1,
            query_preference_polarity: "neutral".into(),
            evidence_preference_polarity: "neutral".into(),
            retrieval_modes: vec!["structured".into()],
        }
    }

    #[tokio::test]
    async fn record_routing_accumulates_process_local_counters() {
        let cfg = Config {
            port: 8099,
            database_url: None,
            default_reply_style: "brief".into(),
            profile_service_base_url: "http://127.0.0.1:1".into(),
            ai_chat_service_base_url: "http://127.0.0.1:1".into(),
            internal_shared_secret: "t".into(),
            env_mode: "dev".into(),
            internal_bind_addr: "127.0.0.1".into(),
        };
        let st = ContextAppState::new(
            PolicyConfigStore::default(),
            cfg,
            MemoryBackend::in_memory(),
        );

        st.record_routing("u1", obs_l1(false, 2)).await;
        st.record_routing("u1", obs_l1(true, 1)).await;

        let m = st
            .routing_metrics
            .lock()
            .expect("routing_metrics mutex poisoned")
            .clone();
        assert_eq!(m.total_requests, 2);
        assert_eq!(m.l1_requests, 2);
        assert_eq!(m.degraded_requests, 1);
        assert_eq!(m.total_conflicts, 3);
        assert!(m.last_observation.is_some());
    }

    #[tokio::test]
    async fn in_memory_store_asmr_snapshot_is_absent() {
        let b = MemoryBackend::in_memory();
        let snap = b.fetch_persistent_asmr_snapshot().await.unwrap();
        assert!(
            snap.is_none(),
            "in-memory path must not fabricate DB snapshots"
        );
    }
}
