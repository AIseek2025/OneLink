//! Standard event envelope per `Rules/16-MVP-EVENT-SCHEMAS-DRAFT.md` §1.

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// 通用事件 envelope（顶层字段与 Rules/16 对齐）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub event_id: String,
    pub event_name: String,
    pub event_version: String,
    pub occurred_at: String,
    pub producer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_user_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_id: Option<String>,
    pub payload: Value,
}

impl EventEnvelope {
    /// 构造 `event_version = "v1"` 的 envelope，`occurred_at` 为当前 UTC RFC3339。
    pub fn new_v1(
        event_name: impl Into<String>,
        producer: impl Into<String>,
        actor_user_id: Option<String>,
        trace_id: Option<String>,
        payload: Value,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4().to_string(),
            event_name: event_name.into(),
            event_version: "v1".to_string(),
            occurred_at: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            producer: producer.into(),
            trace_id,
            region: None,
            actor_user_id,
            subject_id: None,
            payload,
        }
    }
}
