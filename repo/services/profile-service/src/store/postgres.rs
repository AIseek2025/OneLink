use chrono::{DateTime, Utc};
use deadpool_postgres::{Manager, Pool, Runtime};
use serde_json::Value as JsonValue;
use tokio_postgres::{types::Json, NoTls};
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

    pub async fn ensure_user_exists(&self, user_id: &str) -> Result<(), StoreError> {
        let uid = parse_user_uuid(user_id)?;
        let client = self.pool.get().await?;
        client
            .execute(
                "INSERT INTO users (id, status, password_hash) VALUES ($1, 'active', '')
                 ON CONFLICT (id) DO NOTHING",
                &[&uid],
            )
            .await?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_profile(
        &self,
        user_id: &str,
        display_name: Option<&str>,
        avatar_url: Option<&str>,
        headline: Option<&str>,
        bio: Option<&str>,
        city_level_location: Option<&str>,
        languages: &[String],
        is_searchable: bool,
        allow_discovery: bool,
    ) -> Result<(), StoreError> {
        let uid = parse_user_uuid(user_id)?;
        let client = self.pool.get().await?;
        client
            .execute(
                "INSERT INTO profiles (user_id, display_name, avatar_url, headline, bio, city_level_location, languages, is_searchable, allow_discovery)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                 ON CONFLICT (user_id) DO UPDATE SET
                     display_name = COALESCE(EXCLUDED.display_name, profiles.display_name),
                     avatar_url = COALESCE(EXCLUDED.avatar_url, profiles.avatar_url),
                     headline = COALESCE(EXCLUDED.headline, profiles.headline),
                     bio = COALESCE(EXCLUDED.bio, profiles.bio),
                     city_level_location = COALESCE(EXCLUDED.city_level_location, profiles.city_level_location),
                     languages = CASE WHEN array_length(EXCLUDED.languages, 1) IS NULL THEN profiles.languages ELSE EXCLUDED.languages END,
                     is_searchable = EXCLUDED.is_searchable,
                     allow_discovery = EXCLUDED.allow_discovery,
                     updated_at = now()",
                &[
                    &uid,
                    &display_name,
                    &avatar_url,
                    &headline,
                    &bio,
                    &city_level_location,
                    &languages,
                    &is_searchable,
                    &allow_discovery,
                ],
            )
            .await?;
        Ok(())
    }

    pub async fn find_profile(&self, user_id: &str) -> Result<Option<ProfileRow>, StoreError> {
        let uid = match Uuid::parse_str(user_id.trim()) {
            Ok(u) => u,
            Err(_) => return Ok(None),
        };
        let client = self.pool.get().await?;
        let row = client
            .query_opt(
                "SELECT user_id, display_name, avatar_url, headline, bio, city_level_location, languages, is_searchable, allow_discovery, updated_at
                 FROM profiles WHERE user_id = $1",
                &[&uid],
            )
            .await?;
        Ok(row.map(|r| ProfileRow {
            user_id: r.get::<_, Uuid>(0).to_string(),
            display_name: r.get(1),
            avatar_url: r.get(2),
            headline: r.get(3),
            bio: r.get(4),
            city_level_location: r.get(5),
            languages: r.get(6),
            is_searchable: r.get(7),
            allow_discovery: r.get(8),
            updated_at: ts_to_label(r.get(9)),
        }))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_fact(
        &self,
        user_id: &str,
        fact_type: &str,
        fact_key: &str,
        fact_value_json: &JsonValue,
        source_type: &str,
        source_ref_id: Option<Uuid>,
        confidence: Option<f64>,
        status: &str,
        effective_time: Option<&DateTime<Utc>>,
    ) -> Result<(), StoreError> {
        let uid = parse_user_uuid(user_id)?;
        let client = self.pool.get().await?;
        let fact_value = Json(fact_value_json.clone());
        let source_type_value = source_type.to_string();
        let confidence_value = confidence.unwrap_or(0.5);
        let status_value = status.to_string();
        let effective_time_value = effective_time.cloned();
        match source_ref_id {
            Some(ref_id) => {
                let ref_id_opt = Some(ref_id);
                client
                    .execute(
                        "INSERT INTO profile_facts (user_id, fact_type, fact_key, fact_value_json, source_type, source_ref_id, confidence, status, effective_time)
                         VALUES ($1, $2, $3, $4, $5, $6, $7::double precision, $8, $9)
                         ON CONFLICT (user_id, fact_key) DO UPDATE SET
                             fact_type = EXCLUDED.fact_type,
                             fact_value_json = EXCLUDED.fact_value_json,
                             source_type = EXCLUDED.source_type,
                             source_ref_id = EXCLUDED.source_ref_id,
                             confidence = EXCLUDED.confidence,
                             status = EXCLUDED.status,
                             effective_time = EXCLUDED.effective_time,
                             captured_at = now()",
                        &[
                            &uid,
                            &fact_type,
                            &fact_key,
                            &fact_value,
                            &source_type_value,
                            &ref_id_opt,
                            &confidence_value,
                            &status_value,
                            &effective_time_value,
                        ],
                    )
                    .await?;
            }
            None => {
                client
                    .execute(
                        "INSERT INTO profile_facts (user_id, fact_type, fact_key, fact_value_json, source_type, confidence, status, effective_time)
                         VALUES ($1, $2, $3, $4, $5, $6::double precision, $7, $8)
                         ON CONFLICT (user_id, fact_key) DO UPDATE SET
                             fact_type = EXCLUDED.fact_type,
                             fact_value_json = EXCLUDED.fact_value_json,
                             source_type = EXCLUDED.source_type,
                             confidence = EXCLUDED.confidence,
                             status = EXCLUDED.status,
                             effective_time = EXCLUDED.effective_time,
                             captured_at = now()",
                        &[
                            &uid,
                            &fact_type,
                            &fact_key,
                            &fact_value,
                            &source_type_value,
                            &confidence_value,
                            &status_value,
                            &effective_time_value,
                        ],
                    )
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn list_facts(&self, user_id: &str) -> Result<Vec<FactRow>, StoreError> {
        let uid = match Uuid::parse_str(user_id.trim()) {
            Ok(u) => u,
            Err(_) => return Ok(vec![]),
        };
        let client = self.pool.get().await?;
        let rows = client
            .query(
                "SELECT id, user_id, fact_type, fact_key, fact_value_json, source_type, source_ref_id, confidence::double precision AS confidence, status, effective_time, captured_at
                 FROM profile_facts WHERE user_id = $1 AND status = 'active'
                 ORDER BY captured_at",
                &[&uid],
            )
            .await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(FactRow {
                id: r.get::<_, Uuid>(0).to_string(),
                user_id: r.get::<_, Uuid>(1).to_string(),
                fact_type: r.get(2),
                fact_key: r.get(3),
                fact_value_json: r.get::<_, tokio_postgres::types::Json<JsonValue>>(4).0,
                source_type: r.get(5),
                source_ref_id: r.get(6),
                confidence: r.get(7),
                status: r.get(8),
                effective_time: r.get(9),
                captured_at: ts_to_label(r.get(10)),
            });
        }
        Ok(out)
    }

    pub async fn upsert_trait(
        &self,
        user_id: &str,
        trait_type: &str,
        trait_key: &str,
        trait_score: Option<f64>,
        model_version: Option<&str>,
    ) -> Result<(), StoreError> {
        let uid = parse_user_uuid(user_id)?;
        let client = self.pool.get().await?;
        client
            .execute(
                "INSERT INTO profile_traits (user_id, trait_type, trait_key, trait_score, model_version)
                 VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT (user_id, trait_type, trait_key) DO UPDATE SET
                     trait_score = EXCLUDED.trait_score,
                     model_version = EXCLUDED.model_version,
                     updated_at = now()",
                &[&uid, &trait_type, &trait_key, &trait_score, &model_version],
            )
            .await?;
        Ok(())
    }

    pub async fn list_traits(&self, user_id: &str) -> Result<Vec<TraitRow>, StoreError> {
        let uid = match Uuid::parse_str(user_id.trim()) {
            Ok(u) => u,
            Err(_) => return Ok(vec![]),
        };
        let client = self.pool.get().await?;
        let rows = client
            .query(
                "SELECT id, user_id, trait_type, trait_key, trait_score, model_version, updated_at
                 FROM profile_traits WHERE user_id = $1
                 ORDER BY updated_at",
                &[&uid],
            )
            .await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            out.push(TraitRow {
                id: r.get::<_, Uuid>(0).to_string(),
                user_id: r.get::<_, Uuid>(1).to_string(),
                trait_type: r.get(2),
                trait_key: r.get(3),
                trait_score: r.get(4),
                model_version: r.get(5),
                updated_at: ts_to_label(r.get(6)),
            });
        }
        Ok(out)
    }

    pub async fn count_profiles(&self) -> Result<i64, StoreError> {
        let client = self.pool.get().await?;
        let row = client
            .query_one("SELECT COUNT(*) FROM profiles", &[])
            .await?;
        Ok(row.get(0))
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

#[derive(Debug, Clone)]
pub struct ProfileRow {
    pub user_id: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub headline: Option<String>,
    pub bio: Option<String>,
    pub city_level_location: Option<String>,
    pub languages: Option<Vec<String>>,
    pub is_searchable: bool,
    pub allow_discovery: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct FactRow {
    pub id: String,
    pub user_id: String,
    pub fact_type: String,
    pub fact_key: String,
    pub fact_value_json: JsonValue,
    pub source_type: String,
    pub source_ref_id: Option<Uuid>,
    pub confidence: Option<f64>,
    pub status: String,
    pub effective_time: Option<DateTime<Utc>>,
    pub captured_at: String,
}

#[derive(Debug, Clone)]
pub struct TraitRow {
    pub id: String,
    pub user_id: String,
    pub trait_type: String,
    pub trait_key: String,
    pub trait_score: Option<f64>,
    pub model_version: Option<String>,
    pub updated_at: String,
}

fn parse_user_uuid(user_id: &str) -> Result<Uuid, StoreError> {
    Uuid::parse_str(user_id.trim()).map_err(|_| StoreError::InvalidUserId(user_id.to_string()))
}

fn ts_to_label(ts: DateTime<Utc>) -> String {
    ts.to_rfc3339()
}
