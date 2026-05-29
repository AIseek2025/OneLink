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

    pub async fn insert_thread(
        &self,
        thread_id: &Uuid,
        thread_type: &str,
        status: &str,
    ) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        client
            .execute(
                "INSERT INTO dm_threads (id, thread_type, status, created_at)
                 VALUES ($1, $2, $3, now())
                 ON CONFLICT (id) DO NOTHING",
                &[thread_id, &thread_type, &status.to_string()],
            )
            .await?;
        Ok(())
    }

    pub async fn insert_participant(
        &self,
        participant_id: &Uuid,
        thread_id: &Uuid,
        user_id: &Uuid,
        role: &str,
        status: &str,
    ) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        client
            .execute(
                "INSERT INTO dm_participants (id, thread_id, user_id, role, status, joined_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, now(), now())
                 ON CONFLICT (id) DO NOTHING",
                &[participant_id, thread_id, user_id, &role.to_string(), &status.to_string()],
            )
            .await?;
        Ok(())
    }

    pub async fn insert_message(
        &self,
        message_id: &Uuid,
        thread_id: &Uuid,
        sender_user_id: &Uuid,
        content_type: &str,
        content_text: &str,
        review_status: &str,
    ) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        client
            .execute(
                "INSERT INTO dm_messages (id, thread_id, sender_user_id, content_type, content_text, review_status, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, now())",
                &[message_id, thread_id, sender_user_id, &content_type, &content_text, &review_status],
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

    pub async fn get_thread(&self, thread_id: &Uuid) -> Option<ThreadRow> {
        let client = self.pool.get().await.ok()?;
        let rows = client
            .query(
                "SELECT id, thread_type, status, created_at FROM dm_threads WHERE id = $1",
                &[thread_id],
            )
            .await
            .ok()?;
        rows.into_iter().next().map(|row| ThreadRow {
            id: row.get(0),
            thread_type: row.get(1),
            status: row.get(2),
            created_at: row.get(3),
        })
    }

    pub async fn list_threads_by_user(&self, user_id: &Uuid, limit: i64) -> Vec<ThreadRow> {
        let client = match self.pool.get().await {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let rows = client
            .query(
                "SELECT t.id, t.thread_type, t.status, t.created_at FROM dm_threads t JOIN dm_participants p ON p.thread_id = t.id WHERE p.user_id = $1 AND p.status = 'active' ORDER BY t.created_at DESC LIMIT $2",
                &[user_id, &limit],
            )
            .await
            .unwrap_or_default();
        rows.into_iter()
            .map(|row| ThreadRow {
                id: row.get(0),
                thread_type: row.get(1),
                status: row.get(2),
                created_at: row.get(3),
            })
            .collect()
    }

    pub async fn list_messages_by_thread(&self, thread_id: &Uuid, limit: i64) -> Vec<MessageRow> {
        let client = match self.pool.get().await {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let rows = client
            .query(
                "SELECT id, thread_id, sender_user_id, content_type, content_text, review_status, created_at FROM dm_messages WHERE thread_id = $1 ORDER BY created_at DESC LIMIT $2",
                &[thread_id, &limit],
            )
            .await
            .unwrap_or_default();
        rows.into_iter()
            .map(|row| MessageRow {
                id: row.get(0),
                thread_id: row.get(1),
                sender_user_id: row.get(2),
                content_type: row.get(3),
                content_text: row.get(4),
                review_status: row.get(5),
                created_at: row.get(6),
            })
            .collect()
    }
}

pub struct ThreadRow {
    pub id: Uuid,
    pub thread_type: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct MessageRow {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub sender_user_id: Uuid,
    pub content_type: String,
    pub content_text: String,
    pub review_status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
