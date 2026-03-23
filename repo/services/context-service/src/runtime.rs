//! Runtime checkpoint placeholder for logical-agent wake/sleep.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeCheckpoint {
    pub agent_id: String,
    pub user_id: String,
    pub conversation_id: Option<String>,
    pub schema_version: i32,
    pub working_summary_ref: Option<String>,
    #[serde(default)]
    pub runtime_state_blob: serde_json::Value,
    pub policy_versions: serde_json::Value,
}
