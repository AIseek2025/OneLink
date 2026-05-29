use chrono::{DateTime, Utc};
use deadpool_postgres::{Manager, Pool, Runtime};
use tokio_postgres::NoTls;
use uuid::Uuid;

use crate::store::memory::{SessionRow, UserRow};
use crate::store::StoreError;

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

    pub async fn insert_user(&self, user: &UserRow) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        let id = Uuid::parse_str(&user.user_id)
            .map_err(|_| StoreError::InvalidId(format!("user_id: {}", user.user_id)))?;
        let created_at = parse_ts(&user.created_at);
        let updated_at = created_at;
        client
            .execute(
                "INSERT INTO users (id, status, primary_region, primary_language, timezone, password_hash, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                &[
                    &id,
                    &user.status,
                    &user.primary_region,
                    &user.primary_language,
                    &user.timezone,
                    &user.password_hash,
                    &created_at,
                    &updated_at,
                ],
            )
            .await?;
        Ok(())
    }

    pub async fn find_user_by_email(
        &self,
        email: &str,
    ) -> Result<Option<(String, UserRow)>, StoreError> {
        let client = self.pool.get().await?;
        let row = client
            .query_opt(
                "SELECT u.id, u.status, u.primary_region, u.primary_language, u.timezone, u.password_hash, u.created_at
                 FROM users u
                 JOIN identity_bindings ib ON ib.user_id = u.id
                 WHERE ib.email_or_phone_hash = $1
                 LIMIT 1",
                &[&email],
            )
            .await?;
        Ok(row.map(|r| row_to_user(&r)))
    }

    pub async fn find_user_by_id(&self, user_id: &str) -> Result<Option<UserRow>, StoreError> {
        let id = Uuid::parse_str(user_id)
            .map_err(|_| StoreError::InvalidId(format!("user_id: {user_id}")))?;
        let client = self.pool.get().await?;
        let row = client
            .query_opt(
                "SELECT id, status, primary_region, primary_language, timezone, password_hash, created_at
                 FROM users WHERE id = $1",
                &[&id],
            )
            .await?;
        Ok(row.map(|r| row_to_user(&r).1))
    }

    pub async fn insert_session(
        &self,
        token: &str,
        session: &SessionRow,
    ) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        let id = Uuid::new_v4();
        let user_id = Uuid::parse_str(&session.user_id)
            .map_err(|_| StoreError::InvalidId(format!("user_id: {}", session.user_id)))?;
        let expires_at = parse_ts(&session.expires_at);
        let created_at = Utc::now();
        client
            .execute(
                "INSERT INTO sessions (id, user_id, token_hash, expires_at, created_at)
                 VALUES ($1, $2, $3, $4, $5)",
                &[&id, &user_id, &token, &expires_at, &created_at],
            )
            .await?;
        Ok(())
    }

    pub async fn find_session(&self, token: &str) -> Result<Option<SessionRow>, StoreError> {
        let client = self.pool.get().await?;
        let row = client
            .query_opt(
                "SELECT user_id, token_hash, expires_at FROM sessions WHERE token_hash = $1",
                &[&token],
            )
            .await?;
        Ok(row.map(|r| SessionRow {
            token: r.get::<_, String>(1),
            user_id: r.get::<_, Uuid>(0).to_string(),
            expires_at: r
                .get::<_, DateTime<Utc>>(2)
                .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        }))
    }

    pub async fn delete_session(&self, token: &str) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        client
            .execute("DELETE FROM sessions WHERE token_hash = $1", &[&token])
            .await?;
        Ok(())
    }

    pub async fn delete_sessions_by_user_id(&self, user_id: &str) -> Result<(), StoreError> {
        let uid = Uuid::parse_str(user_id)
            .map_err(|_| StoreError::InvalidId(format!("user_id: {user_id}")))?;
        let client = self.pool.get().await?;
        client
            .execute("DELETE FROM sessions WHERE user_id = $1", &[&uid])
            .await?;
        Ok(())
    }

    pub async fn count_users(&self) -> Result<usize, StoreError> {
        let client = self.pool.get().await?;
        let count: i64 = client
            .query_one("SELECT COUNT(*)::bigint FROM users", &[])
            .await?
            .get(0);
        Ok(count as usize)
    }

    pub async fn count_sessions(&self) -> Result<usize, StoreError> {
        let client = self.pool.get().await?;
        let count: i64 = client
            .query_one("SELECT COUNT(*)::bigint FROM sessions", &[])
            .await?
            .get(0);
        Ok(count as usize)
    }

    pub async fn insert_email_binding(&self, user_id: &str, email: &str) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        let uid = Uuid::parse_str(user_id)
            .map_err(|_| StoreError::InvalidId(format!("user_id: {user_id}")))?;
        let id = Uuid::new_v4();
        client
            .execute(
                "INSERT INTO identity_bindings (id, user_id, provider, email_or_phone_hash, is_primary, created_at)
                 VALUES ($1, $2, 'email', $3, true, now())",
                &[&id, &uid, &email],
            )
            .await?;
        Ok(())
    }

    pub async fn find_email_by_user_id(&self, user_id: &str) -> Result<Option<String>, StoreError> {
        let uid = Uuid::parse_str(user_id)
            .map_err(|_| StoreError::InvalidId(format!("user_id: {user_id}")))?;
        let client = self.pool.get().await?;
        let row = client
            .query_opt(
                "SELECT email_or_phone_hash FROM identity_bindings WHERE user_id = $1 AND provider = 'email' AND is_primary = true LIMIT 1",
                &[&uid],
            )
            .await?;
        Ok(row.map(|r| r.get::<_, String>(0)))
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

fn row_to_user(row: &tokio_postgres::Row) -> (String, UserRow) {
    let id: Uuid = row.get(0);
    let user_id = id.to_string();
    let created_at: DateTime<Utc> = row.get(6);
    let user = UserRow {
        user_id: user_id.clone(),
        status: row.get(1),
        primary_region: row.get(2),
        primary_language: row.get(3),
        timezone: row.get(4),
        password_hash: row.get(5),
        created_at: created_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    };
    (user_id, user)
}

fn parse_ts(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.to_utc())
        .unwrap_or_else(|_| Utc::now())
}
