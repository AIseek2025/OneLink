//! Storage model placeholders for V2 memory objects.

use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct MemoryEntityRecord {
    pub id: String,
    pub user_id: String,
    pub entity_type: String,
    pub name: String,
}

#[derive(Debug, Clone, Default)]
pub struct MemoryEntityLinkRecord {
    pub id: String,
    pub user_id: String,
    pub entity_id: String,
    pub memory_id: String,
    pub relationship: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Default)]
pub struct MemorySummaryRecord {
    pub summary_id: String,
    pub user_id: String,
    pub conversation_id: String,
    pub summary_type: String,
    pub summary_text: String,
    pub memory_ids: Vec<String>,
    pub source_message_ids: Vec<String>,
    pub keywords: Vec<String>,
    pub temporal_state: String,
    pub supersedes_previous: bool,
    /// 与 `memory_distiller` 一致：`positive` | `negative` | `neutral`
    pub preference_polarity: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Default)]
pub struct ForgettingDecisionRecord {
    pub id: String,
    pub user_id: String,
    pub target_type: String,
    pub target_id: String,
    pub decision: String,
}

#[derive(Debug, Clone, Default)]
pub struct FailureCaseRecord {
    pub id: String,
    pub user_id: String,
    pub stage: String,
    pub category: String,
    pub detail: String,
    pub trace_id: Option<String>,
    pub retryable: bool,
    pub attempt_count: u32,
    pub created_at: String,
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeCheckpointRecord {
    pub checkpoint_id: String,
    pub agent_id: String,
    pub user_id: String,
    pub conversation_id: Option<String>,
    pub schema_version: i32,
    pub working_summary_ref: Option<String>,
    pub runtime_state_blob: Value,
    pub policy_versions: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Default)]
pub struct RoutingObservation {
    pub executed_route: String,
    pub candidate_route: String,
    pub escalation_reasons: Vec<String>,
    pub upgraded: bool,
    pub evidence_count: usize,
    pub summary_hits: usize,
    pub artifact_hits: usize,
    pub entity_hits: usize,
    pub conflict_count: usize,
    pub route_confidence: f64,
    pub estimated_llm_calls: u32,
    pub estimated_tokens: usize,
    pub query_preview: String,
    pub degraded: bool,
    pub elapsed_ms: u128,
    /// 查询侧推断的偏好极性（非偏好类查询为 `neutral`）
    pub query_preference_polarity: String,
    /// 当前选中的最高分证据上的 `preference_polarity`（无证据为 `neutral`）
    pub evidence_preference_polarity: String,
}

#[derive(Debug, Clone, Default)]
pub struct RoutingMetrics {
    pub total_requests: u64,
    pub l1_requests: u64,
    pub l2_candidates: u64,
    pub l3_candidates: u64,
    pub degraded_requests: u64,
    pub total_conflicts: u64,
    pub last_observation: Option<RoutingObservation>,
}
