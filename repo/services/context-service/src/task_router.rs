//! Logical task router placeholder.

use crate::l1_policy::{
    ROUTER_EXPLICIT_CONFLICT, ROUTER_MENTIONS_CURRENT, ROUTER_MENTIONS_PAST,
    ROUTER_MENTIONS_UPDATE, ROUTER_REASONING_LIKE,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskType {
    Chat,
    Match,
    Safety,
    Unknown,
}

impl From<&str> for TaskType {
    fn from(value: &str) -> Self {
        match value {
            "chat" => Self::Chat,
            "match" => Self::Match,
            "safety" => Self::Safety,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteLevel {
    L1,
    L2,
    L3,
}

impl RouteLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::L1 => "L1",
            Self::L2 => "L2",
            Self::L3 => "L3",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RoutingDecision {
    pub candidate_route: String,
    pub executed_route: String,
    pub escalation_reasons: Vec<String>,
}

pub fn decide_route(task_type: TaskType, input: &str) -> RoutingDecision {
    let normalized = input.trim();
    let mut candidate = RouteLevel::L1;
    let mut reasons = vec![];

    let mentions_past = ROUTER_MENTIONS_PAST
        .iter()
        .any(|needle| normalized.contains(*needle));
    let mentions_current = ROUTER_MENTIONS_CURRENT
        .iter()
        .any(|needle| normalized.contains(*needle));
    let mentions_update = ROUTER_MENTIONS_UPDATE
        .iter()
        .any(|needle| normalized.contains(*needle));
    if ROUTER_EXPLICIT_CONFLICT
        .iter()
        .any(|needle| normalized.contains(*needle))
        || ((mentions_past && mentions_current) || (mentions_past && mentions_update))
    {
        candidate = RouteLevel::L3;
        reasons.push("timeline_or_update_conflict".to_string());
    }

    if candidate == RouteLevel::L1
        && (normalized.chars().count() > 36
            || has_reasoning_signal(normalized)
            || ROUTER_REASONING_LIKE
                .iter()
                .any(|needle| normalized.contains(*needle)))
    {
        candidate = RouteLevel::L2;
        reasons.push("multi_evidence_reasoning".to_string());
    }

    if matches!(task_type, TaskType::Match) && candidate == RouteLevel::L1 {
        candidate = RouteLevel::L2;
        reasons.push("match_queries_need_reasoning".to_string());
    }

    RoutingDecision {
        candidate_route: candidate.as_str().to_string(),
        // MVP 当前只落 L1，候选升级先记录为埋点，不启用额外在线推理。
        executed_route: RouteLevel::L1.as_str().to_string(),
        escalation_reasons: reasons,
    }
}

fn has_reasoning_signal(input: &str) -> bool {
    input.contains("为什么") && !input.contains("改为什么")
}
