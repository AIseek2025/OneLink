//! Retrieval mode placeholder for vector / graph expansion evolution.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalMode {
    Structured,
    Semantic,
    Temporal,
    Graph,
    Rerank,
}

impl RetrievalMode {
    /// MVP 默认可用子集（未读 `policy_configs` 时）；`graph` / `rerank` 由 `PolicyConfigStore` 与请求侧 `retrieval_modes` 联合决定。
    pub fn mvp_enabled(self) -> bool {
        matches!(self, Self::Structured | Self::Semantic | Self::Temporal)
    }

    /// 当策略显式打开 graph/rerank 时，在运行态可作为真实检索阶段参与（见 `evidence::collect_l1_evidence`）。
    pub fn requires_policy_gate(self) -> bool {
        matches!(self, Self::Graph | Self::Rerank)
    }
}
