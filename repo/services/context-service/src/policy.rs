//! Minimal policy config store placeholder for V2.

use crate::l1_policy::{
    ACTIVATION_DECAY_RATE_DEFAULT, IMPORTANCE_SCORE_DEFAULT, SCORE_ACTIVATION_WEIGHT_DEFAULT,
};

/// Optional overrides loaded from `policy_configs` (`010_optimization.sql`) at startup.
#[derive(Debug, Clone, Default)]
pub struct PolicyDbOverrides {
    pub memory_policy_version: Option<String>,
    pub session_policy_version: Option<String>,
    pub retrieval_policy_version: Option<String>,
    pub default_reply_style: Option<String>,
    pub enabled_retrieval_modes: Option<Vec<String>>,
    pub graph_enabled: Option<bool>,
    pub rerank_enabled: Option<bool>,
    pub activation_decay_rate: Option<f64>,
    pub score_activation_weight: Option<f64>,
    pub importance_score_default: Option<f64>,
    pub compressible_evaluation_enabled: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct PolicyConfigStore {
    pub memory_policy_version: String,
    pub session_policy_version: String,
    pub retrieval_policy_version: String,
    pub default_reply_style: String,
    pub enabled_retrieval_modes: Vec<String>,
    pub graph_enabled: bool,
    pub rerank_enabled: bool,
    pub activation_decay_rate: f64,
    pub score_activation_weight: f64,
    pub importance_score_default: f64,
    pub compressible_evaluation_enabled: bool,
}

impl Default for PolicyConfigStore {
    fn default() -> Self {
        Self {
            memory_policy_version: "v1".to_string(),
            session_policy_version: "v1".to_string(),
            retrieval_policy_version: "v1".to_string(),
            default_reply_style: "brief".to_string(),
            enabled_retrieval_modes: vec![
                "structured".to_string(),
                "semantic".to_string(),
                "temporal".to_string(),
            ],
            graph_enabled: false,
            rerank_enabled: false,
            activation_decay_rate: ACTIVATION_DECAY_RATE_DEFAULT,
            score_activation_weight: SCORE_ACTIVATION_WEIGHT_DEFAULT,
            importance_score_default: IMPORTANCE_SCORE_DEFAULT,
            compressible_evaluation_enabled: false,
        }
    }
}

impl PolicyConfigStore {
    pub fn apply_db_overrides(&mut self, o: PolicyDbOverrides) {
        if let Some(v) = o.memory_policy_version {
            self.memory_policy_version = v;
        }
        if let Some(v) = o.session_policy_version {
            self.session_policy_version = v;
        }
        if let Some(v) = o.retrieval_policy_version {
            self.retrieval_policy_version = v;
        }
        if let Some(v) = o.default_reply_style {
            self.default_reply_style = v;
        }
        if let Some(v) = o.enabled_retrieval_modes {
            if !v.is_empty() {
                self.enabled_retrieval_modes = v;
            }
        }
        if let Some(v) = o.graph_enabled {
            self.graph_enabled = v;
        }
        if let Some(v) = o.rerank_enabled {
            self.rerank_enabled = v;
        }
        if let Some(v) = o.activation_decay_rate {
            self.activation_decay_rate = v;
        }
        if let Some(v) = o.score_activation_weight {
            self.score_activation_weight = v;
        }
        if let Some(v) = o.importance_score_default {
            self.importance_score_default = v;
        }
        if let Some(v) = o.compressible_evaluation_enabled {
            self.compressible_evaluation_enabled = v;
        }
    }

    /// 写入 `MemorySummaryRecord.policy_version` 等与记忆策略对账用的稳定单值标签（本轮为 `memory_policy_version`，不做多字段拼接）。
    pub fn policy_version_label(&self) -> String {
        self.memory_policy_version.clone()
    }

    pub fn filter_retrieval_modes(&self, requested: &[String]) -> (Vec<String>, bool) {
        let requested_modes = if requested.is_empty() {
            self.enabled_retrieval_modes.clone()
        } else {
            requested.to_vec()
        };

        let mut degraded = false;
        let filtered = requested_modes
            .into_iter()
            .filter_map(|mode| match mode.as_str() {
                "structured" | "semantic" | "temporal" => Some(mode),
                "graph" if self.graph_enabled => Some(mode),
                "rerank" if self.rerank_enabled => Some(mode),
                "graph" | "rerank" => {
                    degraded = true;
                    None
                }
                _ => {
                    degraded = true;
                    None
                }
            })
            .collect();

        (filtered, degraded)
    }
}

#[cfg(test)]
mod policy_store_tests {
    use super::*;

    #[test]
    fn apply_db_overrides_partial_keys_preserve_defaults() {
        let mut s = PolicyConfigStore::default();
        assert!(!s.graph_enabled);
        s.apply_db_overrides(PolicyDbOverrides {
            graph_enabled: Some(true),
            ..Default::default()
        });
        assert!(s.graph_enabled);
        assert!(!s.rerank_enabled);
        assert_eq!(s.memory_policy_version, "v1");
    }

    #[test]
    fn apply_db_overrides_empty_enabled_retrieval_modes_ignored() {
        let mut s = PolicyConfigStore::default();
        let before = s.enabled_retrieval_modes.clone();
        s.apply_db_overrides(PolicyDbOverrides {
            enabled_retrieval_modes: Some(vec![]),
            ..Default::default()
        });
        assert_eq!(s.enabled_retrieval_modes, before);
    }

    #[test]
    fn apply_db_overrides_activation_fields_override_defaults() {
        let mut s = PolicyConfigStore::default();
        s.apply_db_overrides(PolicyDbOverrides {
            activation_decay_rate: Some(0.55),
            score_activation_weight: Some(0.2),
            importance_score_default: Some(0.65),
            ..Default::default()
        });
        assert_eq!(s.activation_decay_rate, 0.55);
        assert_eq!(s.score_activation_weight, 0.2);
        assert_eq!(s.importance_score_default, 0.65);
    }

    #[test]
    fn apply_db_overrides_illegal_bool_none_keeps_default() {
        let mut s = PolicyConfigStore {
            graph_enabled: false,
            ..Default::default()
        };
        s.apply_db_overrides(PolicyDbOverrides {
            graph_enabled: None,
            rerank_enabled: None,
            ..Default::default()
        });
        assert!(!s.graph_enabled);
    }

    #[test]
    fn filter_retrieval_modes_graph_rerank_respect_flags() {
        let mut s = PolicyConfigStore {
            graph_enabled: false,
            rerank_enabled: false,
            ..Default::default()
        };
        let (modes, deg) = s.filter_retrieval_modes(&["structured".into(), "graph".into()]);
        assert_eq!(modes, vec!["structured".to_string()]);
        assert!(deg);

        s.graph_enabled = true;
        s.rerank_enabled = true;
        let (modes2, deg2) = s.filter_retrieval_modes(&["graph".into(), "rerank".into()]);
        assert_eq!(modes2, vec!["graph".to_string(), "rerank".to_string()]);
        assert!(!deg2);
    }
}
