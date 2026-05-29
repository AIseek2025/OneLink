use deadpool_postgres::{Manager, Pool, Runtime};
use tokio_postgres::NoTls;
use uuid::Uuid;

use super::StoreError;

#[derive(Clone, Debug)]
pub struct PostgresStore {
    pool: Pool,
}

impl PostgresStore {
    pub async fn connect(database_url: &str) -> Result<Self, StoreError> {
        let pg = database_url
            .parse::<tokio_postgres::Config>()
            .map_err(|e| StoreError::InvalidId(format!("DATABASE_URL parse: {e}")))?;
        let mgr = Manager::new(pg, NoTls);
        let pool = Pool::builder(mgr)
            .runtime(Runtime::Tokio1)
            .build()
            .map_err(|e| StoreError::InvalidId(format!("pool build: {e}")))?;
        Ok(Self { pool })
    }

    pub async fn insert_find_request(
        &self,
        find_request_id: &Uuid,
        user_id: &Uuid,
        raw_query: &str,
        _intent_tags: &[String],
        status: &str,
    ) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        client
            .execute(
                "INSERT INTO find_requests (id, user_id, raw_query, status, risk_level)
                 VALUES ($1, $2, $3, $4, 'low')
                 ON CONFLICT (id) DO UPDATE SET status = EXCLUDED.status",
                &[find_request_id, user_id, &raw_query, &status.to_string()],
            )
            .await?;
        Ok(())
    }

    pub async fn insert_recommendation_card(
        &self,
        result_set_id: &Uuid,
        candidate_user_id: &Uuid,
        rank_position: i32,
        score: f64,
        reason_summary: &str,
    ) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        client
            .execute(
                "INSERT INTO recommendation_cards (id, result_set_id, candidate_user_id, rank_position, score, reason_summary, served_at)
                 VALUES ($1, $2, $3, $4, $5, $6, now())
                 ON CONFLICT (id) DO NOTHING",
                &[
                    &Uuid::new_v4(),
                    result_set_id,
                    candidate_user_id,
                    &rank_position,
                    &score,
                    &reason_summary,
                ],
            )
            .await?;
        Ok(())
    }

    pub async fn insert_feedback(
        &self,
        feedback_id: &Uuid,
        user_id: &Uuid,
        candidate_user_id: &Uuid,
        feedback_type: &str,
    ) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        client
            .execute(
                "INSERT INTO recommendation_feedbacks (id, card_id, actor_user_id, source_event_id, source_event_name, feedback_type, created_at)
                 VALUES ($1, $2, $3, $4, 'match.feedback', $5, now())",
                &[feedback_id, candidate_user_id, user_id, &Uuid::new_v4(), &feedback_type.to_string()],
            )
            .await?;
        Ok(())
    }

    pub async fn insert_match(
        &self,
        match_id: &Uuid,
        user_a_id: &Uuid,
        user_b_id: &Uuid,
        match_type: &str,
        status: &str,
    ) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        client
            .execute(
                "INSERT INTO matches (id, user_a_id, user_b_id, match_type, status, created_at)
                 VALUES ($1, $2, $3, $4, $5, now())
                 ON CONFLICT (id) DO NOTHING",
                &[
                    match_id,
                    user_a_id,
                    user_b_id,
                    &match_type,
                    &status.to_string(),
                ],
            )
            .await?;
        Ok(())
    }

    pub async fn ping(&self) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        client
            .execute("SELECT 1", &[])
            .await
            .map_err(|e| StoreError::InvalidId(format!("ping: {e}")))?;
        Ok(())
    }

    pub async fn get_find_request(&self, find_request_id: &Uuid) -> Option<FindRequestRow> {
        let client = self.pool.get().await.ok()?;
        let rows = client
            .query(
                "SELECT id, user_id, raw_query, status, risk_level, created_at FROM find_requests WHERE id = $1",
                &[find_request_id],
            )
            .await
            .ok()?;
        rows.into_iter().next().map(|row| FindRequestRow {
            id: row.get(0),
            user_id: row.get(1),
            raw_query: row.get(2),
            status: row.get(3),
            created_at: row.get(5),
        })
    }

    pub async fn list_find_requests_by_user(
        &self,
        user_id: &Uuid,
        limit: i64,
    ) -> Vec<FindRequestRow> {
        let client = match self.pool.get().await {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let rows = client
            .query(
                "SELECT id, user_id, raw_query, status, risk_level, created_at FROM find_requests WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2",
                &[user_id, &limit],
            )
            .await
            .unwrap_or_default();
        rows.into_iter()
            .map(|row| FindRequestRow {
                id: row.get(0),
                user_id: row.get(1),
                raw_query: row.get(2),
                status: row.get(3),
                created_at: row.get(5),
            })
            .collect()
    }
}

pub struct FindRequestRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub raw_query: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
