//! Minimal policy config store placeholder for V2.

#[derive(Debug, Clone)]
pub struct PolicyConfigStore {
    pub memory_policy_version: String,
    pub session_policy_version: String,
    pub retrieval_policy_version: String,
    pub default_reply_style: String,
    pub enabled_retrieval_modes: Vec<String>,
    pub graph_enabled: bool,
    pub rerank_enabled: bool,
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
        }
    }
}

impl PolicyConfigStore {
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
