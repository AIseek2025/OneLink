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

    pub async fn insert_report(
        &self,
        ticket_id: &Uuid,
        reporter_user_id: &Uuid,
        target_type: &str,
        target_id: &Uuid,
        reason_code: &str,
        status: &str,
    ) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        client
            .execute(
                "INSERT INTO report_tickets (id, reporter_user_id, target_type, target_id, reason_code, status, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, now())
                 ON CONFLICT (id) DO NOTHING",
                &[ticket_id, reporter_user_id, &target_type, target_id, &reason_code, &status.to_string()],
            )
            .await?;
        Ok(())
    }

    pub async fn insert_block(
        &self,
        block_id: &Uuid,
        blocker_user_id: &Uuid,
        blocked_user_id: &Uuid,
        source: &str,
    ) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        client
            .execute(
                "INSERT INTO user_blocks (id, blocker_user_id, blocked_user_id, source, created_at)
                 VALUES ($1, $2, $3, $4, now())
                 ON CONFLICT DO NOTHING",
                &[
                    block_id,
                    blocker_user_id,
                    blocked_user_id,
                    &source.to_string(),
                ],
            )
            .await?;
        Ok(())
    }

    pub async fn remove_block(
        &self,
        blocker_user_id: &Uuid,
        blocked_user_id: &Uuid,
    ) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        client
            .execute(
                "UPDATE user_blocks SET released_at = now() WHERE blocker_user_id = $1 AND blocked_user_id = $2 AND released_at IS NULL",
                &[blocker_user_id, blocked_user_id],
            )
            .await?;
        Ok(())
    }

    pub async fn insert_risk_assessment(
        &self,
        assessment_id: &Uuid,
        target_type: &str,
        target_id: &str,
        risk_level: &str,
        risk_codes: &[String],
        decision: &str,
    ) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        client
            .execute(
                "INSERT INTO risk_assessments (id, target_type, target_id, risk_level, risk_codes, rule_version, model_version, decision, created_at)
                 VALUES ($1, $2, $3, $4, $5, 'v1', 'rule-v1', $6, now())",
                &[assessment_id, &target_type, &target_id, &risk_level, &risk_codes, &decision.to_string()],
            )
            .await?;
        Ok(())
    }

    pub async fn insert_moderation_action(
        &self,
        action_id: &Uuid,
        target_user_id: &Uuid,
        source_ticket_id: &Uuid,
        action_type: &str,
        reason_summary: &str,
    ) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        client
            .execute(
                "INSERT INTO moderation_actions (id, target_user_id, source_ticket_id, action_type, reason_summary, executed_at)
                 VALUES ($1, $2, $3, $4, $5, now())
                 ON CONFLICT (id) DO NOTHING",
                &[action_id, target_user_id, source_ticket_id, &action_type.to_string(), &reason_summary],
            )
            .await?;
        Ok(())
    }

    pub async fn insert_risk_flag(
        &self,
        flag_id: &Uuid,
        user_id: &Uuid,
        flag_type: &str,
        source: &str,
        description: &str,
        severity: &str,
    ) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        client
            .execute(
                "INSERT INTO risk_flags (id, user_id, flag_type, source, description, severity, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, now())
                 ON CONFLICT (id) DO NOTHING",
                &[flag_id, user_id, &flag_type.to_string(), &source.to_string(), &description.to_string(), &severity.to_string()],
            )
            .await?;
        Ok(())
    }

    pub async fn insert_report_action(
        &self,
        action_id: &Uuid,
        ticket_id: &Uuid,
        action_type: &str,
        actor_user_id: &Uuid,
    ) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        client
            .execute(
                "INSERT INTO report_actions (id, ticket_id, action_type, actor_user_id, created_at)
                 VALUES ($1, $2, $3, $4, now())
                 ON CONFLICT (id) DO NOTHING",
                &[
                    action_id,
                    ticket_id,
                    &action_type.to_string(),
                    actor_user_id,
                ],
            )
            .await?;
        Ok(())
    }

    pub async fn insert_screening(
        &self,
        screening_id: &Uuid,
        target_type: &str,
        target_id: &Uuid,
        verdict: &str,
        risk_score: f64,
        reason: Option<&str>,
    ) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        let reason_val: Option<&str> = reason;
        client
            .execute(
                "INSERT INTO safety_screenings (id, target_type, target_id, verdict, risk_score, reason, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, now())
                 ON CONFLICT (id) DO NOTHING",
                &[screening_id, &target_type.to_string(), target_id, &verdict.to_string(), &risk_score, &reason_val],
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

    pub async fn get_report(&self, ticket_id: &Uuid) -> Option<ReportRow> {
        let client = self.pool.get().await.ok()?;
        let rows = client
            .query(
                "SELECT id, reporter_user_id, target_type, target_id, reason_code, status, created_at FROM report_tickets WHERE id = $1",
                &[ticket_id],
            )
            .await
            .ok()?;
        rows.into_iter().next().map(|row| ReportRow {
            id: row.get(0),
            reporter_user_id: row.get(1),
            target_type: row.get(2),
            target_id: row.get(3),
            reason_code: row.get(4),
            status: row.get(5),
            created_at: row.get(6),
        })
    }

    pub async fn list_blocks_by_blocker(&self, blocker_user_id: &Uuid) -> Vec<BlockRow> {
        let client = match self.pool.get().await {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let rows = client
            .query(
                "SELECT id, blocker_user_id, blocked_user_id, source, created_at, released_at FROM user_blocks WHERE blocker_user_id = $1 AND released_at IS NULL ORDER BY created_at DESC",
                &[blocker_user_id],
            )
            .await
            .unwrap_or_default();
        rows.into_iter()
            .map(|row| BlockRow {
                id: row.get(0),
                blocker_user_id: row.get(1),
                blocked_user_id: row.get(2),
                source: row.get(3),
                created_at: row.get(4),
            })
            .collect()
    }

    pub async fn is_blocked(&self, blocker_user_id: &Uuid, blocked_user_id: &Uuid) -> bool {
        let client = match self.pool.get().await {
            Ok(c) => c,
            Err(_) => return false,
        };
        client
            .query(
                "SELECT 1 FROM user_blocks WHERE blocker_user_id = $1 AND blocked_user_id = $2 AND released_at IS NULL",
                &[blocker_user_id, blocked_user_id],
            )
            .await
            .map(|rows| !rows.is_empty())
            .unwrap_or(false)
    }
}

pub struct ReportRow {
    pub id: Uuid,
    pub reporter_user_id: Uuid,
    pub target_type: String,
    pub target_id: Uuid,
    pub reason_code: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct BlockRow {
    pub id: Uuid,
    pub blocker_user_id: Uuid,
    pub blocked_user_id: Uuid,
    pub source: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
