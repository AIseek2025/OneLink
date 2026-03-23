//! Context assembly placeholder for V2.

#[derive(Debug, Clone, Default)]
pub struct ContextAssemblyPlan {
    pub selected_summary_ids: Vec<String>,
    pub selected_memory_ids: Vec<String>,
    pub retrieval_used: Vec<String>,
    pub degraded: bool,
}
