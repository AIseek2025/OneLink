//! Dual-mode memory / checkpoint storage: in-process HashMap vs Postgres.

mod memory;
mod postgres;

use std::collections::HashMap;
use std::sync::Arc;

pub use memory::InMemoryStore;

use crate::{
    app_state::MemoryArtifactRecord,
    config::Config,
    memory_store::{
        FailureCaseRecord, MemoryEntityLinkRecord, MemoryEntityRecord, MemorySummaryRecord,
        RoutingObservation, RuntimeCheckpointRecord,
    },
};

/// Counts for ASMR-Lite observability (persistent path uses DB aggregates when in Postgres mode).
#[derive(Debug, Clone, Default)]
pub struct ObsCounts {
    pub artifact_count: usize,
    pub summary_count: usize,
    pub entity_count: usize,
    pub link_count: usize,
    pub checkpoint_count: usize,
    pub latest_summary_policy_version: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UserMemorySnapshot {
    pub summaries: HashMap<String, MemorySummaryRecord>,
    pub artifacts: HashMap<String, MemoryArtifactRecord>,
    pub entities: HashMap<String, MemoryEntityRecord>,
    pub entity_links: HashMap<String, MemoryEntityLinkRecord>,
}

/// One row for `context_logs` (Postgres path only; in-memory backend ignores).
#[derive(Debug, Clone)]
pub struct ContextLogInsert {
    pub user_id: String,
    pub conversation_id: String,
    pub selected_summary_ids: Vec<String>,
    pub selected_memory_ids: Vec<String>,
    pub retrieval_modes: Vec<String>,
    pub task_type: String,
    pub max_tokens: i32,
    pub memory_limit: i32,
    pub summary_limit: i32,
}

/// Append-only failure row for `context_failure_events` (Postgres only).
#[derive(Debug, Clone)]
pub struct FailureEventInsert {
    pub user_id: String,
    pub stage: String,
    pub category: String,
    pub detail: String,
    pub trace_id: Option<String>,
    pub retryable: bool,
    pub attempt_count: u32,
    pub created_at: String,
}

/// Row for `forgetting_decisions` (Postgres only; in-memory path no-op on store).
#[derive(Debug, Clone)]
pub struct ForgettingDecisionInsert {
    pub user_id: String,
    pub target_type: String,
    pub target_id: String,
    pub decision: String,
    pub reason_codes: serde_json::Value,
    pub policy_version: Option<String>,
    pub cold_storage_ref: Option<String>,
}

/// Cross-restart slice for asmr-lite when `011_runtime_observability.sql` is applied.
#[derive(Debug, Clone, Default)]
pub struct PersistentAsmrSnapshot {
    pub last_observation: Option<RoutingObservation>,
    pub recent_failures: Vec<FailureCaseRecord>,
    pub total_failures: u64,
}

#[derive(Debug)]
pub enum StoreError {
    Postgres(tokio_postgres::Error),
    Pool(deadpool_postgres::PoolError),
    InvalidUserId(String),
    InvalidId(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Postgres(e) => write!(f, "postgres: {e}"),
            StoreError::Pool(e) => write!(f, "pool: {e}"),
            StoreError::InvalidUserId(s) => write!(f, "invalid user_id for postgres: {s}"),
            StoreError::InvalidId(s) => write!(f, "invalid id: {s}"),
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StoreError::Postgres(e) => Some(e),
            StoreError::Pool(e) => Some(e),
            _ => None,
        }
    }
}

impl From<tokio_postgres::Error> for StoreError {
    fn from(e: tokio_postgres::Error) -> Self {
        StoreError::Postgres(e)
    }
}

impl From<deadpool_postgres::PoolError> for StoreError {
    fn from(e: deadpool_postgres::PoolError) -> Self {
        StoreError::Pool(e)
    }
}

#[derive(Clone, Debug)]
pub enum MemoryBackend {
    InMemory(Arc<InMemoryStore>),
    Postgres(postgres::PostgresStore),
}

impl MemoryBackend {
    pub fn in_memory() -> Self {
        Self::InMemory(Arc::new(InMemoryStore::new()))
    }

    pub async fn connect(config: &Config) -> Self {
        if let Some(url) = &config.database_url {
            match postgres::PostgresStore::connect(url).await {
                Ok(pg) => {
                    tracing::info!("context-service: connected to Postgres");
                    Self::Postgres(pg)
                }
                Err(e) => {
                    tracing::error!(
                        "context-service: Postgres connect failed ({e}) — refusing silent in-memory fallback in shared environment"
                    );
                    panic!(
                        "context-service: FATAL: Postgres connect failed (DATABASE_URL was set). \
                         Silent in-memory fallback is forbidden for shared environments. \
                         Fix the database connection or unset DATABASE_URL to explicitly use dev-only in-memory mode. \
                         Error: {e}"
                    );
                }
            }
        } else {
            tracing::info!("context-service: in-memory mode (DATABASE_URL unset) — dev/smoke only");
            Self::in_memory()
        }
    }

    pub fn is_postgres(&self) -> bool {
        matches!(self, Self::Postgres(_))
    }

    pub async fn ensure_user_exists(&self, user_id: &str) -> Result<(), StoreError> {
        match self {
            MemoryBackend::InMemory(_) => Ok(()),
            MemoryBackend::Postgres(p) => p.ensure_user_exists(user_id).await,
        }
    }

    pub async fn insert_artifact(&self, rec: MemoryArtifactRecord) -> Result<(), StoreError> {
        match self {
            MemoryBackend::InMemory(m) => {
                m.insert_artifact(rec);
                Ok(())
            }
            MemoryBackend::Postgres(p) => p.insert_artifact(&rec).await,
        }
    }

    pub async fn insert_summary(&self, rec: MemorySummaryRecord) -> Result<(), StoreError> {
        match self {
            MemoryBackend::InMemory(m) => {
                m.insert_summary(rec);
                Ok(())
            }
            MemoryBackend::Postgres(p) => p.insert_summary(&rec).await,
        }
    }

    pub async fn upsert_entity(&self, rec: MemoryEntityRecord) -> Result<(), StoreError> {
        match self {
            MemoryBackend::InMemory(m) => {
                m.upsert_entity(rec);
                Ok(())
            }
            MemoryBackend::Postgres(p) => p.upsert_entity(&rec).await,
        }
    }

    pub async fn upsert_entity_link(&self, rec: MemoryEntityLinkRecord) -> Result<(), StoreError> {
        match self {
            MemoryBackend::InMemory(m) => {
                m.upsert_entity_link(rec);
                Ok(())
            }
            MemoryBackend::Postgres(p) => p.upsert_entity_link(&rec).await,
        }
    }

    pub async fn user_snapshot(&self, user_id: &str) -> Result<UserMemorySnapshot, StoreError> {
        match self {
            MemoryBackend::InMemory(m) => Ok(m.user_snapshot(user_id)),
            MemoryBackend::Postgres(p) => p.user_snapshot(user_id).await,
        }
    }

    pub async fn get_artifacts_by_ids(
        &self,
        ids: &[String],
    ) -> Result<Vec<MemoryArtifactRecord>, StoreError> {
        match self {
            MemoryBackend::InMemory(m) => Ok(m.get_artifacts_by_ids(ids)),
            MemoryBackend::Postgres(p) => p.get_artifacts_by_ids(ids).await,
        }
    }

    pub async fn get_artifact_optional(
        &self,
        id: &str,
    ) -> Result<Option<MemoryArtifactRecord>, StoreError> {
        match self {
            MemoryBackend::InMemory(m) => Ok(m.get_artifact(id)),
            MemoryBackend::Postgres(p) => p.get_artifact_optional(id).await,
        }
    }

    pub async fn checkpoint_lookup(&self, dedupe_key: &str) -> Result<Option<String>, StoreError> {
        match self {
            MemoryBackend::InMemory(m) => Ok(m.checkpoint_lookup(dedupe_key)),
            MemoryBackend::Postgres(p) => p.checkpoint_lookup(dedupe_key).await,
        }
    }

    /// Returns checkpoint id (existing or newly inserted).
    pub async fn checkpoint_insert(
        &self,
        dedupe_key: &str,
        rec: RuntimeCheckpointRecord,
    ) -> Result<String, StoreError> {
        match self {
            MemoryBackend::InMemory(m) => Ok(m.checkpoint_insert(dedupe_key, rec)),
            MemoryBackend::Postgres(p) => p.checkpoint_insert(dedupe_key, rec).await,
        }
    }

    pub async fn consolidate_has_event(&self, event_id: &str) -> Result<bool, StoreError> {
        match self {
            MemoryBackend::InMemory(m) => Ok(m.consolidate_has_event(event_id)),
            MemoryBackend::Postgres(p) => p.consolidate_has_event(event_id).await,
        }
    }

    pub async fn consolidate_record(
        &self,
        event_id: &str,
        summary_id: &str,
    ) -> Result<(), StoreError> {
        match self {
            MemoryBackend::InMemory(m) => {
                m.consolidate_record(event_id, summary_id);
                Ok(())
            }
            MemoryBackend::Postgres(p) => p.consolidate_record(event_id, summary_id).await,
        }
    }

    pub async fn observability_counts(&self) -> Result<ObsCounts, StoreError> {
        match self {
            MemoryBackend::InMemory(m) => Ok(m.observability_counts()),
            MemoryBackend::Postgres(p) => p.observability_counts().await,
        }
    }

    /// Persist a context build audit row when backed by Postgres. In-memory mode is a no-op.
    pub async fn insert_context_log(&self, row: &ContextLogInsert) -> Result<(), StoreError> {
        match self {
            MemoryBackend::InMemory(_) => Ok(()),
            MemoryBackend::Postgres(p) => p.insert_context_log(row).await,
        }
    }

    pub async fn insert_routing_observation(
        &self,
        user_id: &str,
        obs: &RoutingObservation,
    ) -> Result<(), StoreError> {
        match self {
            MemoryBackend::InMemory(_) => Ok(()),
            MemoryBackend::Postgres(p) => p.insert_routing_observation(user_id, obs).await,
        }
    }

    pub async fn insert_failure_event(&self, row: &FailureEventInsert) -> Result<(), StoreError> {
        match self {
            MemoryBackend::InMemory(_) => Ok(()),
            MemoryBackend::Postgres(p) => p.insert_failure_event(row).await,
        }
    }

    pub async fn insert_forgetting_decision(
        &self,
        row: &ForgettingDecisionInsert,
    ) -> Result<(), StoreError> {
        match self {
            MemoryBackend::InMemory(_) => Ok(()),
            MemoryBackend::Postgres(p) => p.insert_forgetting_decision(row).await,
        }
    }

    pub async fn touch_artifacts(
        &self,
        ids: &[String],
        touched_at: &str,
    ) -> Result<(), StoreError> {
        match self {
            MemoryBackend::InMemory(m) => {
                m.touch_artifacts(ids, touched_at);
                Ok(())
            }
            MemoryBackend::Postgres(p) => p.touch_artifacts(ids, touched_at).await,
        }
    }

    /// `None` when not Postgres or when persistent tables are missing / unreadable.
    pub async fn fetch_persistent_asmr_snapshot(
        &self,
    ) -> Result<Option<PersistentAsmrSnapshot>, StoreError> {
        match self {
            MemoryBackend::InMemory(_) => Ok(None),
            MemoryBackend::Postgres(p) => p.fetch_persistent_asmr_snapshot().await,
        }
    }
}
