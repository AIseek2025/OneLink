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
    pub fn mvp_enabled(self) -> bool {
        matches!(self, Self::Structured | Self::Semantic | Self::Temporal)
    }
}
