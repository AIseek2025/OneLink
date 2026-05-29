//! Context assembly: query-aware context build with compressible evaluation.
//!
//! Phase 4 enhancement — turns the V2 placeholder into a functional context
//! assembly pipeline that:
//! 1. Produces a structured `ContextAssemblyPlan` from L1 evidence.
//! 2. Assembles query-aware `memory_context` text with intent-tagged sections.
//! 3. Supports an optional compressible evaluation step (togglable via policy).

use crate::l1_policy;
use crate::policy::PolicyConfigStore;

#[derive(Debug, Clone, Default)]
pub struct ContextAssemblyPlan {
    pub selected_summary_ids: Vec<String>,
    pub selected_memory_ids: Vec<String>,
    pub retrieval_used: Vec<String>,
    pub degraded: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompressionMode {
    #[default]
    None,
    Lite,
    Aggressive,
}

#[derive(Debug, Clone, Default)]
pub struct CompressionEvaluation {
    pub enabled: bool,
    pub compression_mode: CompressionMode,
    pub max_context_tokens: usize,
    pub budget_policy_version: String,
    pub evaluation_summary: String,
    pub quality_score: f64,
    pub compression_ratio: f64,
}

pub struct AssembledContext {
    pub memory_context: String,
    pub plan: ContextAssemblyPlan,
    pub compression: CompressionEvaluation,
}

pub fn assemble_query_aware_context(
    summary_texts: &[(&str, &str, f64)],
    artifact_texts: &[(&str, &str, &str, f64)],
    query: &str,
    retrieval_modes: &[String],
    policy: &PolicyConfigStore,
    max_tokens: i32,
) -> AssembledContext {
    let intent = parse_query_intent(query);
    let mut sections: Vec<String> = Vec::new();
    let mut total_estimated_tokens: usize = 0;

    if intent.wants_preference {
        let mut pref_items: Vec<String> = Vec::new();
        for (text, polarity, score) in summary_texts {
            if *polarity != "neutral" && *score > 0.3 {
                pref_items.push(format!("[{}] {}", polarity, text));
            }
        }
        for (text, network_type, polarity, score) in artifact_texts {
            if *polarity != "neutral" && *score > 0.3 {
                pref_items.push(format!("[{}:{}] {}", network_type, polarity, text));
            }
        }
        if !pref_items.is_empty() {
            let section = format!("偏好信息: {}", pref_items.join("; "));
            total_estimated_tokens += estimate_tokens(&section);
            sections.push(section);
        }
    }

    if intent.wants_current || intent.wants_update {
        let mut temporal_items: Vec<String> = Vec::new();
        for (text, temporal_state, _score) in summary_texts {
            if *temporal_state == "current" || *temporal_state == "updated" {
                temporal_items.push(text.to_string());
            }
        }
        for (text, _network_type, _polarity, _score) in artifact_texts {
            temporal_items.push(text.to_string());
        }
        if !temporal_items.is_empty() {
            let section = format!("当前状态: {}", temporal_items.join("; "));
            total_estimated_tokens += estimate_tokens(&section);
            sections.push(section);
        }
    }

    if intent.wants_location {
        let mut location_items: Vec<String> = Vec::new();
        for (text, _temporal_state, _score) in summary_texts {
            if l1_policy::contains_known_city(text) {
                location_items.push(text.to_string());
            }
        }
        for (text, _network_type, _polarity, _score) in artifact_texts {
            if l1_policy::contains_known_city(text) {
                location_items.push(text.to_string());
            }
        }
        if !location_items.is_empty() {
            let section = format!("位置信息: {}", location_items.join("; "));
            total_estimated_tokens += estimate_tokens(&section);
            sections.push(section);
        }
    }

    if intent.wants_connection {
        let mut conn_items: Vec<String> = Vec::new();
        for (text, _temporal_state, _score) in summary_texts {
            if text.contains("投资人") || text.contains("合伙人") {
                conn_items.push(text.to_string());
            }
        }
        for (text, _network_type, _polarity, _score) in artifact_texts {
            if text.contains("投资人") || text.contains("合伙人") {
                conn_items.push(text.to_string());
            }
        }
        if !conn_items.is_empty() {
            let section = format!("连接需求: {}", conn_items.join("; "));
            total_estimated_tokens += estimate_tokens(&section);
            sections.push(section);
        }
    }

    if intent.wants_past {
        let mut past_items: Vec<String> = Vec::new();
        for (text, temporal_state, _score) in summary_texts {
            if *temporal_state == "past" {
                past_items.push(text.to_string());
            }
        }
        for (text, _network_type, _polarity, _score) in artifact_texts {
            if text.contains("之前") || text.contains("以前") || text.contains("曾经") {
                past_items.push(text.to_string());
            }
        }
        if !past_items.is_empty() {
            let section = format!("历史信息: {}", past_items.join("; "));
            total_estimated_tokens += estimate_tokens(&section);
            sections.push(section);
        }
    }

    if intent.wants_remote {
        let mut remote_items: Vec<String> = Vec::new();
        for (text, _temporal_state, _score) in summary_texts {
            if text.contains("远程") || text.contains("办公") {
                remote_items.push(text.to_string());
            }
        }
        for (text, _network_type, _polarity, _score) in artifact_texts {
            if text.contains("远程") || text.contains("办公") {
                remote_items.push(text.to_string());
            }
        }
        if !remote_items.is_empty() {
            let section = format!("办公方式: {}", remote_items.join("; "));
            total_estimated_tokens += estimate_tokens(&section);
            sections.push(section);
        }
    }

    if sections.is_empty() {
        let mut general_items: Vec<String> = Vec::new();
        for (text, _, score) in summary_texts {
            if *score > 0.3 {
                general_items.push(text.to_string());
            }
        }
        for (text, _, _, score) in artifact_texts {
            if *score > 0.3 {
                general_items.push(text.to_string());
            }
        }
        if !general_items.is_empty() {
            let section = format!("相关记忆: {}", general_items.join("; "));
            total_estimated_tokens += estimate_tokens(&section);
            sections.push(section);
        }
    }

    let raw_context = if sections.is_empty() {
        format!(
            "retrieval_modes={}; no structured evidence matched; input_preview={}",
            retrieval_modes.join(","),
            preview(query)
        )
    } else {
        format!(
            "retrieval_modes={}; input_preview={}; {}",
            retrieval_modes.join(","),
            preview(query),
            sections.join(" | ")
        )
    };

    let max_ctx = if max_tokens > 0 {
        max_tokens as usize
    } else {
        usize::MAX
    };
    let compression = evaluate_compression(total_estimated_tokens, max_ctx, policy);

    let memory_context =
        if compression.enabled && compression.compression_mode != CompressionMode::None {
            apply_compression(&raw_context, max_ctx, compression.compression_mode)
        } else {
            raw_context
        };

    let plan = ContextAssemblyPlan {
        selected_summary_ids: Vec::new(),
        selected_memory_ids: Vec::new(),
        retrieval_used: retrieval_modes.to_vec(),
        degraded: sections.is_empty(),
    };

    AssembledContext {
        memory_context,
        plan,
        compression,
    }
}

fn evaluate_compression(
    estimated_tokens: usize,
    max_tokens: usize,
    policy: &PolicyConfigStore,
) -> CompressionEvaluation {
    let enabled = policy.compressible_evaluation_enabled;
    if !enabled {
        return CompressionEvaluation {
            enabled: false,
            compression_mode: CompressionMode::None,
            max_context_tokens: max_tokens,
            budget_policy_version: policy.policy_version_label(),
            evaluation_summary: "compressible_evaluation disabled by policy".to_string(),
            quality_score: 1.0,
            compression_ratio: 1.0,
        };
    }

    if max_tokens == 0 || estimated_tokens == 0 {
        return CompressionEvaluation {
            enabled: true,
            compression_mode: CompressionMode::None,
            max_context_tokens: max_tokens,
            budget_policy_version: policy.policy_version_label(),
            evaluation_summary: "no compression needed (zero budget or zero content)".to_string(),
            quality_score: 1.0,
            compression_ratio: 1.0,
        };
    }

    let ratio = estimated_tokens as f64 / max_tokens as f64;
    let (mode, quality, summary) = if ratio <= 1.0 {
        (
            CompressionMode::None,
            1.0,
            "within budget; no compression".to_string(),
        )
    } else if ratio <= 2.0 {
        (
            CompressionMode::Lite,
            0.85,
            format!("lite compression: estimated={estimated_tokens} max={max_tokens}"),
        )
    } else {
        (
            CompressionMode::Aggressive,
            0.6,
            format!("aggressive compression: estimated={estimated_tokens} max={max_tokens}"),
        )
    };

    CompressionEvaluation {
        enabled: true,
        compression_mode: mode,
        max_context_tokens: max_tokens,
        budget_policy_version: policy.policy_version_label(),
        evaluation_summary: summary,
        quality_score: quality,
        compression_ratio: ratio,
    }
}

fn apply_compression(context: &str, max_tokens: usize, mode: CompressionMode) -> String {
    let max_chars = match mode {
        CompressionMode::None => return context.to_string(),
        CompressionMode::Lite => (max_tokens * 4).max(64),
        CompressionMode::Aggressive => (max_tokens * 3).max(32),
    };
    if context.chars().count() <= max_chars {
        return context.to_string();
    }
    let truncated: String = context.chars().take(max_chars).collect();
    format!("{}…[compressed]", truncated)
}

#[derive(Debug, Default)]
#[allow(dead_code)]
struct QueryIntent {
    wants_current: bool,
    wants_past: bool,
    wants_update: bool,
    wants_location: bool,
    wants_preference: bool,
    wants_connection: bool,
    wants_remote: bool,
}

fn parse_query_intent(query: &str) -> QueryIntent {
    QueryIntent {
        wants_current: l1_policy::INTENT_WANTS_CURRENT
            .iter()
            .any(|m| query.contains(*m)),
        wants_past: l1_policy::INTENT_WANTS_PAST
            .iter()
            .any(|m| query.contains(*m)),
        wants_update: l1_policy::INTENT_WANTS_UPDATE
            .iter()
            .any(|m| query.contains(*m)),
        wants_location: l1_policy::INTENT_WANTS_LOCATION
            .iter()
            .any(|m| query.contains(*m)),
        wants_preference: l1_policy::INTENT_WANTS_PREFERENCE
            .iter()
            .any(|m| query.contains(*m)),
        wants_connection: l1_policy::INTENT_WANTS_CONNECTION
            .iter()
            .any(|m| query.contains(*m)),
        wants_remote: l1_policy::INTENT_WANTS_REMOTE
            .iter()
            .any(|m| query.contains(*m)),
    }
}

fn estimate_tokens(text: &str) -> usize {
    (text.chars().count() / 4).max(1)
}

fn preview(value: &str) -> String {
    value.chars().take(48).collect()
}

#[cfg(test)]
mod context_builder_tests {
    use super::*;
    use crate::policy::PolicyConfigStore;

    fn policy_with_compression(enabled: bool) -> PolicyConfigStore {
        PolicyConfigStore {
            compressible_evaluation_enabled: enabled,
            ..PolicyConfigStore::default()
        }
    }

    #[test]
    fn assemble_preference_query_produces_preference_section() {
        let policy = PolicyConfigStore::default();
        let result = assemble_query_aware_context(
            &[("不喜欢推销式沟通", "negative", 0.8)],
            &[],
            "用户喜欢什么样的沟通方式？",
            &["structured".to_string()],
            &policy,
            4096,
        );
        assert!(
            result.memory_context.contains("偏好信息"),
            "expected preference section, got: {}",
            result.memory_context
        );
    }

    #[test]
    fn assemble_current_query_produces_current_state_section() {
        let policy = PolicyConfigStore::default();
        let result = assemble_query_aware_context(
            &[("当前所在城市是上海", "current", 0.9)],
            &[],
            "用户现在在哪个城市？",
            &["structured".to_string()],
            &policy,
            4096,
        );
        assert!(
            result.memory_context.contains("当前状态"),
            "expected current-state section, got: {}",
            result.memory_context
        );
    }

    #[test]
    fn assemble_location_query_produces_location_section() {
        let policy = PolicyConfigStore::default();
        let result = assemble_query_aware_context(
            &[("之前所在城市是北京", "past", 0.8)],
            &[],
            "用户在哪座城市？",
            &["structured".to_string()],
            &policy,
            4096,
        );
        assert!(
            result.memory_context.contains("位置信息"),
            "expected location section, got: {}",
            result.memory_context
        );
    }

    #[test]
    fn assemble_connection_query_produces_connection_section() {
        let policy = PolicyConfigStore::default();
        let result = assemble_query_aware_context(
            &[("希望认识投资人", "timeless", 0.8)],
            &[],
            "用户想认识谁？",
            &["structured".to_string()],
            &policy,
            4096,
        );
        assert!(
            result.memory_context.contains("连接需求"),
            "expected connection section, got: {}",
            result.memory_context
        );
    }

    #[test]
    fn assemble_generic_query_produces_general_section() {
        let policy = PolicyConfigStore::default();
        let result = assemble_query_aware_context(
            &[("关注 AI 创业", "timeless", 0.7)],
            &[],
            "用户兴趣",
            &["structured".to_string()],
            &policy,
            4096,
        );
        assert!(
            result.memory_context.contains("相关记忆"),
            "expected general memory section, got: {}",
            result.memory_context
        );
    }

    #[test]
    fn assemble_empty_evidence_produces_degraded() {
        let policy = PolicyConfigStore::default();
        let result = assemble_query_aware_context(
            &[],
            &[],
            "用户兴趣",
            &["structured".to_string()],
            &policy,
            4096,
        );
        assert!(
            result.plan.degraded,
            "expected degraded=true for empty evidence"
        );
        assert!(result
            .memory_context
            .contains("no structured evidence matched"));
    }

    #[test]
    fn compression_disabled_by_default() {
        let policy = PolicyConfigStore::default();
        let result = assemble_query_aware_context(
            &[("一些文本", "timeless", 0.5)],
            &[],
            "查询",
            &["structured".to_string()],
            &policy,
            4096,
        );
        assert!(
            !result.compression.enabled,
            "compression should be disabled by default"
        );
        assert_eq!(result.compression.compression_mode, CompressionMode::None);
    }

    #[test]
    fn compression_enabled_within_budget() {
        let policy = policy_with_compression(true);
        let result = assemble_query_aware_context(
            &[("短文本", "timeless", 0.5)],
            &[],
            "查询",
            &["structured".to_string()],
            &policy,
            100000,
        );
        assert!(result.compression.enabled);
        assert_eq!(result.compression.compression_mode, CompressionMode::None);
        assert!((result.compression.quality_score - 1.0).abs() < 0.01);
    }

    #[test]
    fn compression_lite_when_exceeding_budget() {
        let policy = policy_with_compression(true);
        let long_text: String = "测试".repeat(500);
        let result = assemble_query_aware_context(
            &[(&long_text, "timeless", 0.8)],
            &[],
            "查询",
            &["structured".to_string()],
            &policy,
            10,
        );
        assert!(result.compression.enabled);
        assert!(result.memory_context.contains("[compressed]"));
    }

    #[test]
    fn compression_aggressive_when_far_exceeding_budget() {
        let policy = policy_with_compression(true);
        let very_long: String = "内容".repeat(2000);
        let result = assemble_query_aware_context(
            &[(&very_long, "timeless", 0.8)],
            &[],
            "查询",
            &["structured".to_string()],
            &policy,
            5,
        );
        assert!(result.compression.enabled);
        assert!(result.compression.quality_score < 0.7);
        assert!(result.memory_context.contains("[compressed]"));
    }
}
