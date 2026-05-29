use chrono::{DateTime, Utc};
use deadpool_postgres::{Manager, Pool, Runtime};
use tokio_postgres::NoTls;
use uuid::Uuid;

use super::{ConversationRow, MessageRow, StoreError};

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

    pub async fn insert_conversation(&self, conv: &ConversationRow) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        let id = parse_uuid(&conv.conversation_id, "conversation_id")?;
        let user_id = parse_user_uuid(&conv.user_id)?;
        let ts = parse_timestamp_fallback(&conv.created_at);
        client
            .execute(
                "INSERT INTO ai_conversations (id, user_id, status, context_version, created_at)
                 VALUES ($1, $2, $3, 0, $4)
                 ON CONFLICT (id) DO UPDATE SET
                     status = EXCLUDED.status,
                     user_id = EXCLUDED.user_id",
                &[&id, &user_id, &conv.status, &ts],
            )
            .await?;
        Ok(())
    }

    pub async fn find_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Option<ConversationRow>, StoreError> {
        let id = match Uuid::parse_str(conversation_id.trim()) {
            Ok(u) => u,
            Err(_) => return Ok(None),
        };
        let client = self.pool.get().await?;
        let row = client
            .query_opt(
                "SELECT id, user_id, status, created_at FROM ai_conversations WHERE id = $1",
                &[&id],
            )
            .await?;
        Ok(row.map(|r| ConversationRow {
            conversation_id: r.get::<_, Uuid>(0).to_string(),
            user_id: r.get::<_, Uuid>(1).to_string(),
            status: r.get(2),
            created_at: ts_to_label(r.get(3)),
        }))
    }

    pub async fn list_conversations_by_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<ConversationRow>, StoreError> {
        let uid = match Uuid::parse_str(user_id.trim()) {
            Ok(u) => u,
            Err(_) => return Ok(vec![]),
        };
        let client = self.pool.get().await?;
        let rows = client
            .query(
                "SELECT id, user_id, status, created_at FROM ai_conversations WHERE user_id = $1 ORDER BY created_at DESC",
                &[&uid],
            )
            .await?;
        Ok(rows
            .iter()
            .map(|r| ConversationRow {
                conversation_id: r.get::<_, Uuid>(0).to_string(),
                user_id: r.get::<_, Uuid>(1).to_string(),
                status: r.get(2),
                created_at: ts_to_label(r.get(3)),
            })
            .collect())
    }

    pub async fn insert_message_with_content(
        &self,
        conversation_id: &str,
        msg: &MessageRow,
    ) -> Result<(), StoreError> {
        let mut client = self.pool.get().await?;
        let conv_id = parse_uuid(conversation_id, "conversation_id")?;
        let msg_id = parse_uuid(&msg.message_id, "message_id")?;
        let ts = parse_timestamp_fallback(&msg.created_at);
        let tx = client.transaction().await?;
        tx.execute(
            "INSERT INTO ai_messages (id, conversation_id, sender_type, content_type, created_at)
             VALUES ($1, $2, $3, $4, $5)",
            &[&msg_id, &conv_id, &msg.sender_type, &msg.content_type, &ts],
        )
        .await?;
        let content_id = Uuid::new_v5(
            &Uuid::NAMESPACE_DNS,
            format!("onelink:ai_message_content:{}", msg.message_id).as_bytes(),
        );
        let metadata: Option<serde_json::Value> = None;
        let metadata_json = metadata.map(tokio_postgres::types::Json);
        tx.execute(
            "INSERT INTO ai_message_contents (id, message_id, content_text, content_metadata, created_at)
             VALUES ($1, $2, $3, $4, $5)",
            &[&content_id, &msg_id, &msg.content_text, &metadata_json, &ts],
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn list_messages_by_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<MessageRow>, StoreError> {
        let conv_id = match Uuid::parse_str(conversation_id.trim()) {
            Ok(u) => u,
            Err(_) => return Ok(vec![]),
        };
        let client = self.pool.get().await?;
        let rows = client
            .query(
                "SELECT m.id, m.sender_type, m.content_type, m.created_at,
                        mc.content_text
                 FROM ai_messages m
                 LEFT JOIN ai_message_contents mc ON mc.message_id = m.id
                 WHERE m.conversation_id = $1
                 ORDER BY m.created_at",
                &[&conv_id],
            )
            .await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let content_text: Option<String> = r.get(4);
            out.push(MessageRow {
                message_id: r.get::<_, Uuid>(0).to_string(),
                sender_type: r.get(1),
                content_text: content_text.unwrap_or_default(),
                content_type: r.get(2),
                created_at: ts_to_label(r.get(3)),
            });
        }
        Ok(out)
    }

    pub async fn update_conversation_last_message(
        &self,
        conversation_id: &str,
        context_version: i32,
    ) -> Result<(), StoreError> {
        let id = parse_uuid(conversation_id, "conversation_id")?;
        let client = self.pool.get().await?;
        client
            .execute(
                "UPDATE ai_conversations SET last_message_at = now(), context_version = $2 WHERE id = $1",
                &[&id, &context_version],
            )
            .await?;
        Ok(())
    }

    pub async fn count_conversations(&self) -> Result<usize, StoreError> {
        let client = self.pool.get().await?;
        let count: i64 = client
            .query_one("SELECT COUNT(*)::bigint FROM ai_conversations", &[])
            .await?
            .get(0);
        Ok(count as usize)
    }

    pub async fn count_messages(&self) -> Result<usize, StoreError> {
        let client = self.pool.get().await?;
        let count: i64 = client
            .query_one("SELECT COUNT(*)::bigint FROM ai_messages", &[])
            .await?
            .get(0);
        Ok(count as usize)
    }

    pub async fn set_conversation_owner(
        &self,
        conversation_id: &str,
        user_id: &str,
    ) -> Result<(), StoreError> {
        let id = parse_uuid(conversation_id, "conversation_id")?;
        let uid = parse_user_uuid(user_id)?;
        let client = self.pool.get().await?;
        client
            .execute(
                "INSERT INTO ai_conversations (id, user_id, status, context_version, created_at)
                 VALUES ($1, $2, 'active', 0, now())
                 ON CONFLICT (id) DO UPDATE SET user_id = EXCLUDED.user_id",
                &[&id, &uid],
            )
            .await?;
        Ok(())
    }

    pub async fn get_conversation_owner(
        &self,
        conversation_id: &str,
    ) -> Result<Option<String>, StoreError> {
        let id = match Uuid::parse_str(conversation_id.trim()) {
            Ok(u) => u,
            Err(_) => return Ok(None),
        };
        let client = self.pool.get().await?;
        let row = client
            .query_opt("SELECT user_id FROM ai_conversations WHERE id = $1", &[&id])
            .await?;
        Ok(row.map(|r| r.get::<_, Uuid>(0).to_string()))
    }

    pub async fn set_user_primary_conversation(
        &self,
        user_id: &str,
        conversation_id: &str,
    ) -> Result<(), StoreError> {
        let uid = parse_user_uuid(user_id)?;
        let cid = parse_uuid(conversation_id, "conversation_id")?;
        let client = self.pool.get().await?;
        client
            .execute(
                "INSERT INTO ai_conversations (id, user_id, status, context_version, created_at)
                 VALUES ($1, $2, 'active', 0, now())
                 ON CONFLICT (id) DO UPDATE SET user_id = EXCLUDED.user_id",
                &[&cid, &uid],
            )
            .await?;
        Ok(())
    }

    pub async fn get_user_primary_conversation(
        &self,
        user_id: &str,
    ) -> Result<Option<String>, StoreError> {
        let uid = match Uuid::parse_str(user_id.trim()) {
            Ok(u) => u,
            Err(_) => return Ok(None),
        };
        let client = self.pool.get().await?;
        let row = client
            .query_opt(
                "SELECT id FROM ai_conversations WHERE user_id = $1 ORDER BY created_at DESC LIMIT 1",
                &[&uid],
            )
            .await?;
        Ok(row.map(|r| r.get::<_, Uuid>(0).to_string()))
    }
    pub async fn ping(&self) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        client
            .execute("SELECT 1", &[])
            .await
            .map_err(|e| StoreError::InvalidId(format!("ping: {e}")))?;
        Ok(())
    }
}

fn parse_user_uuid(user_id: &str) -> Result<Uuid, StoreError> {
    Uuid::parse_str(user_id.trim()).map_err(|_| StoreError::InvalidUserId(user_id.to_string()))
}

fn parse_uuid(s: &str, ctx: &str) -> Result<Uuid, StoreError> {
    Uuid::parse_str(s.trim()).map_err(|_| StoreError::InvalidId(format!("{ctx}: {s}")))
}

fn parse_timestamp_fallback(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.to_utc())
        .unwrap_or_else(|_| Utc::now())
}

fn ts_to_label(ts: DateTime<Utc>) -> String {
    ts.to_rfc3339()
}
