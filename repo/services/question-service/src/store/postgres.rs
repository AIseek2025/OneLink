use chrono::{SecondsFormat, Utc};
use deadpool_postgres::{Config, Pool, Runtime};
use serde_json::{json, Value};
use tokio_postgres::NoTls;
use uuid::Uuid;

use super::StoredAnswer;

#[derive(Debug)]
pub enum StoreError {
    Connection(String),
    Query(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Connection(s) => write!(f, "connection error: {s}"),
            StoreError::Query(s) => write!(f, "query error: {s}"),
        }
    }
}

impl std::error::Error for StoreError {}

pub struct PostgresStore {
    pool: Pool,
}

impl PostgresStore {
    pub async fn connect(database_url: &str) -> Result<Self, StoreError> {
        let mut cfg = Config::new();
        cfg.url = Some(database_url.to_string());
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1), NoTls)
            .map_err(|e| StoreError::Connection(e.to_string()))?;
        let conn = pool
            .get()
            .await
            .map_err(|e| StoreError::Connection(e.to_string()))?;
        conn.execute("SELECT 1", &[])
            .await
            .map_err(|e| StoreError::Connection(e.to_string()))?;
        tracing::info!("question-service: Postgres pool connected");
        Ok(Self { pool })
    }

    pub async fn ensure_seed_catalog(&self) -> Result<(), StoreError> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| StoreError::Query(e.to_string()))?;
        let rows = conn
            .query("SELECT COUNT(*) FROM question_catalog", &[])
            .await
            .map_err(|e| StoreError::Query(e.to_string()))?;
        let count: i64 = rows[0].get(0);
        if count > 0 {
            return Ok(());
        }
        let seeds = super::seed_catalog();
        for s in &seeds {
            conn.execute(
                "INSERT INTO question_catalog (id, question_key, question_text, category, is_active) VALUES ($1, $2, $3, $4, $5)",
                &[
                    &Uuid::parse_str(s.variant_id).unwrap(),
                    &s.variant_id,
                    &s.question_text,
                    &s.requirement_tier,
                    &true,
                ],
            )
            .await
            .map_err(|e| StoreError::Query(e.to_string()))?;
        }
        tracing::info!(count = seeds.len(), "seeded question_catalog");
        Ok(())
    }

    pub async fn status_json(&self, user_id: &str) -> Result<Value, StoreError> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| StoreError::Query(e.to_string()))?;
        let user_uuid = Uuid::parse_str(user_id)
            .map_err(|e| StoreError::Query(format!("invalid user_id: {e}")))?;
        let starter_rows = conn
            .query(
                "SELECT d.id, a.id AS answer_id FROM question_deliveries d LEFT JOIN question_answers a ON a.delivery_id = d.id JOIN question_catalog q ON q.id = d.question_id WHERE d.user_id = $1 AND q.category = 'starter_required'",
                &[&user_uuid],
            )
            .await
            .map_err(|e| StoreError::Query(e.to_string()))?;
        let starter_total = starter_rows.len();
        let starter_done = starter_rows
            .iter()
            .filter(|r| r.get::<_, Option<Uuid>>("answer_id").is_some())
            .count();
        let profile_rows = conn
            .query(
                "SELECT d.id, a.id AS answer_id FROM question_deliveries d LEFT JOIN question_answers a ON a.delivery_id = d.id JOIN question_catalog q ON q.id = d.question_id WHERE d.user_id = $1 AND q.category = 'profile_required'",
                &[&user_uuid],
            )
            .await
            .map_err(|e| StoreError::Query(e.to_string()))?;
        let profile_total = profile_rows.len();
        let profile_done = profile_rows
            .iter()
            .filter(|r| r.get::<_, Option<Uuid>>("answer_id").is_some())
            .count();
        let optional_rows = conn
            .query(
                "SELECT a.id FROM question_answers a JOIN question_deliveries d ON d.id = a.delivery_id JOIN question_catalog q ON q.id = d.question_id WHERE d.user_id = $1 AND q.category = 'optional'",
                &[&user_uuid],
            )
            .await
            .map_err(|e| StoreError::Query(e.to_string()))?;
        let optional_done = optional_rows.len();
        let can_proceed = starter_done >= starter_total && starter_total > 0;
        Ok(json!({
            "starter_required_count": starter_done,
            "starter_required_total": starter_total,
            "profile_required_count": profile_done,
            "profile_required_total": profile_total,
            "optional_count": optional_done,
            "can_proceed_to_find": can_proceed,
        }))
    }

    pub async fn pending_json(&self, user_id: &str, limit: usize) -> Result<Value, StoreError> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| StoreError::Query(e.to_string()))?;
        let user_uuid = Uuid::parse_str(user_id)
            .map_err(|e| StoreError::Query(format!("invalid user_id: {e}")))?;
        let deliveries = self.ensure_deliveries(&conn, &user_uuid).await?;
        let mut items = Vec::new();
        for d in &deliveries {
            let answered: bool = conn
                .query_one(
                    "SELECT EXISTS(SELECT 1 FROM question_answers WHERE delivery_id = $1)",
                    &[&d.id],
                )
                .await
                .map_err(|e| StoreError::Query(e.to_string()))?
                .get(0);
            if answered {
                continue;
            }
            let catalog = conn
                .query_one(
                    "SELECT question_key, question_text, category FROM question_catalog WHERE id = $1",
                    &[&d.question_id],
                )
                .await
                .map_err(|e| StoreError::Query(e.to_string()))?;
            let question_key: String = catalog.get("question_key");
            let question_text: String = catalog.get("question_text");
            let category: String = catalog.get("category");
            items.push(json!({
                "delivery_id": d.id.to_string(),
                "variant_id": question_key,
                "question_text": question_text,
                "question_style": "single_choice",
                "options": [],
                "requirement_tier": category,
            }));
            if items.len() >= limit {
                break;
            }
        }
        Ok(json!({ "items": items }))
    }

    pub async fn submit_answer(
        &self,
        user_id: &str,
        delivery_id: &str,
        variant_id: &str,
        answer_payload: Value,
        answer_state: &str,
    ) -> Result<(StoredAnswer, bool), StoreError> {
        let conn = self
            .pool
            .get()
            .await
            .map_err(|e| StoreError::Query(e.to_string()))?;
        let user_uuid = Uuid::parse_str(user_id)
            .map_err(|e| StoreError::Query(format!("invalid user_id: {e}")))?;
        let delivery_uuid = Uuid::parse_str(delivery_id)
            .map_err(|e| StoreError::Query(format!("invalid delivery_id: {e}")))?;
        let existing = conn
            .query_opt(
                "SELECT a.id, a.answer_text, a.answered_at, d.id AS delivery_id, q.question_key, q.question_text, q.category FROM question_answers a JOIN question_deliveries d ON d.id = a.delivery_id JOIN question_catalog q ON q.id = d.question_id WHERE a.delivery_id = $1",
                &[&delivery_uuid],
            )
            .await
            .map_err(|e| StoreError::Query(e.to_string()))?;
        if let Some(row) = existing {
            let q_key: String = row.get("question_key");
            if q_key != variant_id {
                return Err(StoreError::Query(
                    "variant_id mismatch for delivery".to_string(),
                ));
            }
            return Ok((
                StoredAnswer {
                    answer_id: row.get::<_, Uuid>("id").to_string(),
                    delivery_id: row.get::<_, Uuid>("delivery_id").to_string(),
                    variant_id: q_key,
                    answered_at: row
                        .get::<_, chrono::DateTime<Utc>>("answered_at")
                        .to_rfc3339_opts(SecondsFormat::Millis, true),
                    answer_payload: answer_payload.clone(),
                    answer_state: answer_state.to_string(),
                    question_text: row.get("question_text"),
                    requirement_tier: row.get("category"),
                    question_style: "single_choice".to_string(),
                    answer_text: row.get("answer_text"),
                },
                false,
            ));
        }
        let delivery_row = conn
            .query_opt(
                "SELECT d.id, d.question_id, q.question_key, q.question_text, q.category FROM question_deliveries d JOIN question_catalog q ON q.id = d.question_id WHERE d.id = $1 AND d.user_id = $2",
                &[&delivery_uuid, &user_uuid],
            )
            .await
            .map_err(|e| StoreError::Query(e.to_string()))?;
        let delivery_row =
            delivery_row.ok_or_else(|| StoreError::Query("unknown delivery_id".to_string()))?;
        let q_key: String = delivery_row.get("question_key");
        if q_key != variant_id {
            return Err(StoreError::Query(
                "variant_id does not match delivery".to_string(),
            ));
        }
        let question_text: String = delivery_row.get("question_text");
        let category: String = delivery_row.get("category");
        let answer_text =
            super::build_answer_text("single_choice", &answer_payload, &question_text);
        if answer_state == "answered" && answer_text.trim().is_empty() {
            return Err(StoreError::Query(
                "empty answer for answered state".to_string(),
            ));
        }
        let answer_id = Uuid::new_v4();
        let answered_at = Utc::now();
        conn.execute(
            "INSERT INTO question_answers (id, delivery_id, answer_text, answered_at) VALUES ($1, $2, $3, $4)",
            &[&answer_id, &delivery_uuid, &answer_text, &answered_at],
        )
        .await
        .map_err(|e| StoreError::Query(e.to_string()))?;
        conn.execute(
            "UPDATE question_deliveries SET status = 'answered', answered_at = $1 WHERE id = $2",
            &[&answered_at, &delivery_uuid],
        )
        .await
        .map_err(|e| StoreError::Query(e.to_string()))?;
        Ok((
            StoredAnswer {
                answer_id: answer_id.to_string(),
                delivery_id: delivery_id.to_string(),
                variant_id: variant_id.to_string(),
                answered_at: answered_at.to_rfc3339_opts(SecondsFormat::Millis, true),
                answer_payload,
                answer_state: answer_state.to_string(),
                question_text,
                requirement_tier: category,
                question_style: "single_choice".to_string(),
                answer_text,
            },
            true,
        ))
    }

    async fn ensure_deliveries(
        &self,
        conn: &deadpool_postgres::Object,
        user_uuid: &Uuid,
    ) -> Result<Vec<DeliveryRecord>, StoreError> {
        let existing = conn
            .query(
                "SELECT id, question_id FROM question_deliveries WHERE user_id = $1",
                &[user_uuid],
            )
            .await
            .map_err(|e| StoreError::Query(e.to_string()))?;
        if !existing.is_empty() {
            return Ok(existing
                .iter()
                .map(|r| DeliveryRecord {
                    id: r.get("id"),
                    question_id: r.get("question_id"),
                })
                .collect());
        }
        let catalog = conn
            .query(
                "SELECT id FROM question_catalog WHERE is_active = true",
                &[],
            )
            .await
            .map_err(|e| StoreError::Query(e.to_string()))?;
        let mut records = Vec::new();
        for row in &catalog {
            let question_id: Uuid = row.get("id");
            let delivery_id = Uuid::new_v4();
            conn.execute(
                "INSERT INTO question_deliveries (id, user_id, question_id, status) VALUES ($1, $2, $3, 'pending')",
                &[&delivery_id, user_uuid, &question_id],
            )
            .await
            .map_err(|e| StoreError::Query(e.to_string()))?;
            records.push(DeliveryRecord {
                id: delivery_id,
                question_id,
            });
        }
        Ok(records)
    }
}

struct DeliveryRecord {
    id: Uuid,
    question_id: Uuid,
}
