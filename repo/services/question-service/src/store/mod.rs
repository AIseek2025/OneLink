mod postgres;

use std::collections::HashMap;
use std::fmt;
use std::sync::Mutex;

use chrono::{SecondsFormat, Utc};
use serde_json::{json, Value};
use uuid::Uuid;

pub use postgres::PostgresStore;

#[derive(Debug)]
pub enum PersistenceBackend {
    InMemory,
    Postgres,
}

impl PersistenceBackend {
    pub fn from_config(database_url: Option<&str>) -> Self {
        match database_url {
            Some(url) if !url.trim().is_empty() => {
                tracing::info!(
                    url_prefix = &url[..url.len().min(20)],
                    "question-service: Postgres persistence configured"
                );
                PersistenceBackend::Postgres
            }
            _ => {
                tracing::info!(
                    "question-service: in-memory fallback (DATABASE_URL unset) — dev/smoke only"
                );
                PersistenceBackend::InMemory
            }
        }
    }

    pub async fn from_config_with_connect(database_url: Option<&str>) -> Self {
        match database_url {
            Some(url) if !url.trim().is_empty() => match PostgresStore::connect(url).await {
                Ok(_pg) => {
                    tracing::info!("question-service: connected to Postgres");
                    PersistenceBackend::Postgres
                }
                Err(e) => {
                    tracing::error!(error = %e, "question-service: Postgres connect failed — refusing silent in-memory fallback in shared environment");
                    panic!(
                        "question-service: FATAL: Postgres connect failed (DATABASE_URL was set). \
                         Silent in-memory fallback is forbidden for shared environments. \
                         Fix the database connection or unset DATABASE_URL to explicitly use dev-only in-memory mode. \
                         Error: {e}"
                    );
                }
            },
            _ => {
                tracing::info!(
                    "question-service: no DATABASE_URL, using in-memory store — dev/smoke only"
                );
                PersistenceBackend::InMemory
            }
        }
    }

    pub fn is_postgres(&self) -> bool {
        matches!(self, PersistenceBackend::Postgres)
    }
}

impl fmt::Display for PersistenceBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PersistenceBackend::InMemory => write!(f, "in-memory"),
            PersistenceBackend::Postgres => write!(f, "postgres"),
        }
    }
}

pub(super) fn seed_catalog() -> Vec<SeedQuestion> {
    vec![
        SeedQuestion {
            variant_id: "00000000-0000-4000-8000-000000000001",
            question_text: "你目前所在的主要城市是？",
            question_style: "single_choice",
            requirement_tier: "starter_required",
            options: vec![
                json!("上海"),
                json!("北京"),
                json!("深圳"),
                json!("杭州"),
                json!("其他"),
            ],
        },
        SeedQuestion {
            variant_id: "00000000-0000-4000-8000-000000000002",
            question_text: "你对哪些方向最感兴趣？",
            question_style: "single_choice",
            requirement_tier: "starter_required",
            options: vec![
                json!("AI / 人工智能"),
                json!("创业"),
                json!("产品"),
                json!("投资"),
            ],
        },
        SeedQuestion {
            variant_id: "00000000-0000-4000-8000-000000000003",
            question_text: "你当前最想达成的连接目标是？",
            question_style: "single_choice",
            requirement_tier: "starter_required",
            options: vec![
                json!("希望认识投资人"),
                json!("希望认识技术合伙人"),
                json!("拓展行业人脉"),
            ],
        },
        SeedQuestion {
            variant_id: "00000000-0000-4000-8000-000000000004",
            question_text: "沟通上你更偏好哪种方式？",
            question_style: "open_text",
            requirement_tier: "profile_required",
            options: vec![],
        },
        SeedQuestion {
            variant_id: "00000000-0000-4000-8000-000000000005",
            question_text: "补充一句你的个人简介（可选）",
            question_style: "open_text",
            requirement_tier: "optional",
            options: vec![],
        },
    ]
}

#[derive(Debug, Clone)]
pub(super) struct SeedQuestion {
    variant_id: &'static str,
    question_text: &'static str,
    question_style: &'static str,
    requirement_tier: &'static str,
    options: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct DeliveryRow {
    pub delivery_id: String,
    pub variant_id: String,
    pub question_text: String,
    pub question_style: String,
    pub requirement_tier: String,
    pub options: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct StoredAnswer {
    pub answer_id: String,
    pub delivery_id: String,
    pub variant_id: String,
    pub answered_at: String,
    pub answer_payload: Value,
    pub answer_state: String,
    pub question_text: String,
    pub requirement_tier: String,
    pub question_style: String,
    pub answer_text: String,
}

struct UserState {
    deliveries: Vec<DeliveryRow>,
    answers: HashMap<String, StoredAnswer>,
}

pub struct QuestionStore {
    inner: Mutex<HashMap<String, UserState>>,
}

impl Default for QuestionStore {
    fn default() -> Self {
        Self::new()
    }
}

impl QuestionStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    fn ensure_user(user_states: &mut HashMap<String, UserState>, user_id: &str) {
        if user_states.contains_key(user_id) {
            return;
        }
        let mut deliveries = Vec::new();
        for s in seed_catalog() {
            deliveries.push(DeliveryRow {
                delivery_id: Uuid::new_v4().to_string(),
                variant_id: s.variant_id.to_string(),
                question_text: s.question_text.to_string(),
                question_style: s.question_style.to_string(),
                requirement_tier: s.requirement_tier.to_string(),
                options: s.options.clone(),
            });
        }
        user_states.insert(
            user_id.to_string(),
            UserState {
                deliveries,
                answers: HashMap::new(),
            },
        );
    }

    pub fn status_json(&self, user_id: &str) -> Value {
        let mut g = self.inner.lock().expect("store mutex poisoned");
        Self::ensure_user(&mut g, user_id);
        let u = g.get(user_id).expect("user just seeded");
        let starter_total = u
            .deliveries
            .iter()
            .filter(|d| d.requirement_tier == "starter_required")
            .count();
        let starter_done = u
            .deliveries
            .iter()
            .filter(|d| {
                d.requirement_tier == "starter_required"
                    && u.answers.contains_key(&d.delivery_id)
                    && u.answers[&d.delivery_id].answer_state == "answered"
            })
            .count();
        let profile_total = u
            .deliveries
            .iter()
            .filter(|d| d.requirement_tier == "profile_required")
            .count();
        let profile_done = u
            .deliveries
            .iter()
            .filter(|d| {
                d.requirement_tier == "profile_required"
                    && u.answers.contains_key(&d.delivery_id)
                    && u.answers[&d.delivery_id].answer_state == "answered"
            })
            .count();
        let optional_done = u
            .deliveries
            .iter()
            .filter(|d| {
                d.requirement_tier == "optional"
                    && u.answers.contains_key(&d.delivery_id)
                    && u.answers[&d.delivery_id].answer_state == "answered"
            })
            .count();
        let can_proceed = starter_done >= starter_total && starter_total > 0;
        json!({
            "starter_required_count": starter_done,
            "starter_required_total": starter_total,
            "profile_required_count": profile_done,
            "profile_required_total": profile_total,
            "optional_count": optional_done,
            "can_proceed_to_find": can_proceed,
        })
    }

    pub fn pending_json(&self, user_id: &str, _channel: Option<&str>, limit: usize) -> Value {
        let mut g = self.inner.lock().expect("store mutex poisoned");
        Self::ensure_user(&mut g, user_id);
        let u = g.get(user_id).expect("user");
        let mut items = Vec::new();
        for d in &u.deliveries {
            if u.answers.contains_key(&d.delivery_id) {
                continue;
            }
            items.push(json!({
                "delivery_id": d.delivery_id,
                "variant_id": d.variant_id,
                "question_text": d.question_text,
                "question_style": d.question_style,
                "options": d.options,
                "requirement_tier": d.requirement_tier,
            }));
            if items.len() >= limit {
                break;
            }
        }
        json!({ "items": items })
    }

    pub fn submit_answer(
        &self,
        user_id: &str,
        delivery_id: &str,
        variant_id: &str,
        answer_payload: Value,
        answer_state: &str,
    ) -> Result<(StoredAnswer, bool), String> {
        let mut g = self.inner.lock().expect("store mutex poisoned");
        Self::ensure_user(&mut g, user_id);
        let u = g
            .get_mut(user_id)
            .ok_or_else(|| "user missing after seed".to_string())?;
        if let Some(existing) = u.answers.get(delivery_id) {
            if existing.variant_id != variant_id {
                return Err("variant_id mismatch for delivery".to_string());
            }
            return Ok((existing.clone(), false));
        }
        let delivery = u
            .deliveries
            .iter()
            .find(|d| d.delivery_id == delivery_id)
            .ok_or_else(|| "unknown delivery_id".to_string())?;
        if delivery.variant_id != variant_id {
            return Err("variant_id does not match delivery".to_string());
        }
        let answer_text = build_answer_text(
            &delivery.question_style,
            &answer_payload,
            &delivery.question_text,
        );
        if answer_state == "answered" && answer_text.trim().is_empty() {
            return Err("empty answer for answered state".to_string());
        }
        let answer_id = Uuid::new_v4().to_string();
        let answered_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let stored = StoredAnswer {
            answer_id: answer_id.clone(),
            delivery_id: delivery_id.to_string(),
            variant_id: variant_id.to_string(),
            answered_at: answered_at.clone(),
            answer_payload,
            answer_state: answer_state.to_string(),
            question_text: delivery.question_text.clone(),
            requirement_tier: delivery.requirement_tier.clone(),
            question_style: delivery.question_style.clone(),
            answer_text,
        };
        u.answers.insert(delivery_id.to_string(), stored.clone());
        Ok((stored, true))
    }
}

pub(super) fn build_answer_text(style: &str, payload: &Value, question_text: &str) -> String {
    if let Some(t) = payload.get("text").and_then(|v| v.as_str()) {
        let t = t.trim();
        if !t.is_empty() {
            return format!("{question_text} {t}");
        }
    }
    if let Some(c) = payload.get("choice").and_then(|v| v.as_str()) {
        if !c.trim().is_empty() {
            return format!("{question_text} {c}");
        }
    }
    if style == "open_text" {
        return String::new();
    }
    payload
        .as_str()
        .map(|s| format!("{question_text} {s}"))
        .unwrap_or_else(|| format!("{question_text} {}", payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submit_idempotent_same_delivery() {
        let s = QuestionStore::new();
        let uid = "u1";
        let pending = s.pending_json(uid, None, 5);
        let delivery_id = pending["items"][0]["delivery_id"].as_str().unwrap();
        let variant_id = pending["items"][0]["variant_id"].as_str().unwrap();
        let (a1, new1) = s
            .submit_answer(
                uid,
                delivery_id,
                variant_id,
                json!({ "choice": "上海" }),
                "answered",
            )
            .unwrap();
        assert!(new1);
        let (a2, new2) = s
            .submit_answer(
                uid,
                delivery_id,
                variant_id,
                json!({ "choice": "上海" }),
                "answered",
            )
            .unwrap();
        assert!(!new2);
        assert_eq!(a1.answer_id, a2.answer_id);
    }
}
