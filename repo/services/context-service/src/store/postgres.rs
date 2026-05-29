//! Postgres-backed store (003_context.sql + 003_context_idempotency.sql).

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use deadpool_postgres::{GenericClient as PgGenericClient, Manager, Pool, Runtime};
use serde_json::{json, Value};
use tokio_postgres::error::SqlState;
use tokio_postgres::types::Json;
use tokio_postgres::NoTls;
use uuid::Uuid;

use crate::{
    app_state::MemoryArtifactRecord,
    memory_store::{
        FailureCaseRecord, MemoryEntityLinkRecord, MemoryEntityRecord, MemorySummaryRecord,
        RoutingObservation, RuntimeCheckpointRecord,
    },
    policy::PolicyDbOverrides,
    store::{
        ContextLogInsert, FailureEventInsert, ForgettingDecisionInsert, ObsCounts,
        PersistentAsmrSnapshot, StoreError, UserMemorySnapshot,
    },
};

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

    pub async fn insert_artifact(&self, rec: &MemoryArtifactRecord) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        let user = parse_user_uuid(&rec.user_id)?;
        let mid = parse_uuid(&rec.memory_id, "memory_id")?;
        let structured = artifact_structured_json(rec);
        let src_type = map_source_type(&rec.source_type);
        let conv = conversation_uuid(&rec.conversation_id);
        let src_ev = Uuid::parse_str(rec.source_message_id.trim()).ok();
        let ts = parse_unix_ms_label(&rec.created_at).unwrap_or_else(Utc::now);
        let structured_json = Json(structured);
        let confidence = rec.confidence;
        let importance_score = rec.importance_score;
        client
            .execute(
                "INSERT INTO memory_artifacts (
                    id, user_id, network_type, evidence_type, memory_level, content,
                    source_type, source_service, source_ref_id, source_event_id,
                    confidence, importance_score, last_accessed_at, access_count,
                    content_structured, created_at, updated_at
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11::double precision, $12::double precision, $13, $14, $15, $16, $16
                )
                ON CONFLICT (id) DO UPDATE SET
                    content = EXCLUDED.content,
                    confidence = EXCLUDED.confidence,
                    importance_score = EXCLUDED.importance_score,
                    last_accessed_at = EXCLUDED.last_accessed_at,
                    access_count = EXCLUDED.access_count,
                    content_structured = EXCLUDED.content_structured,
                    updated_at = EXCLUDED.updated_at",
                &[
                    &mid,
                    &user,
                    &rec.network_type,
                    &rec.evidence_type,
                    &rec.memory_level,
                    &rec.content,
                    &src_type,
                    &"context-service",
                    &conv,
                    &src_ev,
                    &confidence,
                    &importance_score,
                    &parse_unix_ms_label(&rec.last_accessed_at).unwrap_or_else(Utc::now),
                    &(rec.access_count as i32),
                    &structured_json,
                    &ts,
                ],
            )
            .await?;
        Ok(())
    }

    pub async fn insert_summary(&self, rec: &MemorySummaryRecord) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        let user = parse_user_uuid(&rec.user_id)?;
        let sid = parse_uuid(&rec.summary_id, "summary_id")?;
        let conv = conversation_uuid(&rec.conversation_id);
        let key_points = json!({
            "memory_ids": rec.memory_ids,
            "keywords": rec.keywords,
            "temporal_state": rec.temporal_state,
            "supersedes_previous": rec.supersedes_previous,
            "preference_polarity": rec.preference_polarity,
        });
        let src_range = json!({ "source_message_ids": rec.source_message_ids });
        let ts = parse_unix_ms_label(&rec.updated_at).unwrap_or_else(Utc::now);
        let kp_json = Json(key_points);
        let sr_json = Json(src_range);
        client
            .execute(
                "INSERT INTO memory_summaries (
                    id, user_id, conversation_id, summary_type, summary_text,
                    key_points_json, source_message_range, policy_version, updated_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                ON CONFLICT (id) DO UPDATE SET
                    summary_text = EXCLUDED.summary_text,
                    key_points_json = EXCLUDED.key_points_json,
                    source_message_range = EXCLUDED.source_message_range,
                    policy_version = EXCLUDED.policy_version,
                    updated_at = EXCLUDED.updated_at",
                &[
                    &sid,
                    &user,
                    &conv,
                    &rec.summary_type,
                    &rec.summary_text,
                    &kp_json,
                    &sr_json,
                    &rec.policy_version,
                    &ts,
                ],
            )
            .await?;
        Ok(())
    }

    pub async fn upsert_entity(&self, rec: &MemoryEntityRecord) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        let user = parse_user_uuid(&rec.user_id)?;
        let eid = stable_entity_uuid(&rec.id);
        let attrs = json!({ "rust_entity_id": rec.id });
        let attrs_json = Json(attrs);
        client
            .execute(
                "INSERT INTO memory_entities (id, user_id, entity_type, name, attributes, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, now(), now())
                 ON CONFLICT (id) DO UPDATE SET
                    name = EXCLUDED.name,
                    entity_type = EXCLUDED.entity_type,
                    attributes = EXCLUDED.attributes,
                    updated_at = now()",
                &[&eid, &user, &rec.entity_type, &rec.name, &attrs_json],
            )
            .await?;
        Ok(())
    }

    pub async fn upsert_entity_link(&self, rec: &MemoryEntityLinkRecord) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        let user = parse_user_uuid(&rec.user_id)?;
        let source = stable_entity_uuid(&rec.entity_id);
        let mem = parse_uuid(&rec.memory_id, "memory_id")?;
        let lid = link_uuid(&rec.id);
        ensure_memory_pointer(&client, user, mem, &rec.memory_id).await?;
        let ev = Some(mem);
        client
            .execute(
                "INSERT INTO memory_entity_links (
                    id, user_id, source_entity_id, target_entity_id, relation_type, confidence, evidence_artifact_id, is_bidirectional
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, true)
                ON CONFLICT (id) DO NOTHING",
                &[
                    &lid,
                    &user,
                    &source,
                    &mem,
                    &rec.relationship,
                    &rec.confidence,
                    &ev,
                ],
            )
            .await?;
        Ok(())
    }

    pub async fn user_snapshot(&self, user_id: &str) -> Result<UserMemorySnapshot, StoreError> {
        let user = parse_user_uuid(user_id)?;
        let client = self.pool.get().await?;
        let rows = client
            .query(
                "SELECT id, user_id, network_type, evidence_type, memory_level, content,
                        source_type, confidence::double precision AS confidence, importance_score::double precision AS importance_score, last_accessed_at, access_count,
                        content_structured, source_ref_id, source_event_id, created_at
                 FROM memory_artifacts WHERE user_id = $1",
                &[&user],
            )
            .await?;
        let mut artifacts = HashMap::new();
        for row in rows {
            let rec = row_to_artifact(&row)?;
            artifacts.insert(rec.memory_id.clone(), rec);
        }
        let rows = client
            .query(
                "SELECT id, user_id, conversation_id, summary_type, summary_text,
                        key_points_json, source_message_range, policy_version, updated_at
                 FROM memory_summaries WHERE user_id = $1",
                &[&user],
            )
            .await?;
        let mut summaries = HashMap::new();
        for row in rows {
            let rec = row_to_summary(&row)?;
            summaries.insert(rec.summary_id.clone(), rec);
        }
        let rows = client
            .query(
                "SELECT id, user_id, entity_type, name, attributes
                 FROM memory_entities
                 WHERE user_id = $1 AND entity_type <> '_memory_pointer'",
                &[&user],
            )
            .await?;
        let mut entities = HashMap::new();
        for row in rows {
            let rec = row_to_entity(&row)?;
            entities.insert(rec.id.clone(), rec);
        }
        let rows = client
            .query(
                "SELECT mel.id, mel.user_id, msrc.attributes, mtgt.name, mel.relation_type, mel.confidence
                 FROM memory_entity_links mel
                 JOIN memory_entities msrc ON msrc.id = mel.source_entity_id
                 JOIN memory_entities mtgt ON mtgt.id = mel.target_entity_id
                 WHERE mel.user_id = $1
                   AND msrc.entity_type <> '_memory_pointer'
                   AND mtgt.entity_type = '_memory_pointer'",
                &[&user],
            )
            .await?;
        let mut entity_links = HashMap::new();
        for row in rows {
            let rec = row_to_entity_link(&row)?;
            entity_links.insert(rec.id.clone(), rec);
        }
        Ok(UserMemorySnapshot {
            summaries,
            artifacts,
            entities,
            entity_links,
        })
    }

    pub async fn get_artifacts_by_ids(
        &self,
        ids: &[String],
    ) -> Result<Vec<MemoryArtifactRecord>, StoreError> {
        let mut out = Vec::new();
        for id in ids {
            if let Some(a) = self.get_artifact_optional(id).await? {
                out.push(a);
            }
        }
        Ok(out)
    }

    pub async fn get_artifact_optional(
        &self,
        id: &str,
    ) -> Result<Option<MemoryArtifactRecord>, StoreError> {
        let mid = match Uuid::parse_str(id.trim()) {
            Ok(u) => u,
            Err(_) => return Ok(None),
        };
        let client = self.pool.get().await?;
        let row = client
            .query_opt(
                "SELECT id, user_id, network_type, evidence_type, memory_level, content,
                        source_type, confidence, importance_score, last_accessed_at, access_count,
                        content_structured, source_ref_id, source_event_id, created_at
                 FROM memory_artifacts WHERE id = $1",
                &[&mid],
            )
            .await?;
        row.map(|r| row_to_artifact(&r)).transpose()
    }

    pub async fn checkpoint_lookup(&self, dedupe_key: &str) -> Result<Option<String>, StoreError> {
        let client = self.pool.get().await?;
        let row = client
            .query_opt(
                "SELECT checkpoint_id::text FROM context_checkpoint_dedupe WHERE dedupe_key = $1",
                &[&dedupe_key],
            )
            .await?;
        Ok(row.map(|r| r.get::<_, String>(0)))
    }

    pub async fn checkpoint_insert(
        &self,
        dedupe_key: &str,
        rec: RuntimeCheckpointRecord,
    ) -> Result<String, StoreError> {
        let mut client = self.pool.get().await?;
        let tx = client.transaction().await?;
        let existing = tx
            .query_opt(
                "SELECT checkpoint_id::text FROM context_checkpoint_dedupe WHERE dedupe_key = $1",
                &[&dedupe_key],
            )
            .await?;
        if let Some(row) = existing {
            let id: String = row.get(0);
            tx.commit().await?;
            return Ok(id);
        }
        let cid = parse_uuid(&rec.checkpoint_id, "checkpoint_id")?;
        let user = parse_user_uuid(&rec.user_id)?;
        let agent = stable_uuid_label("agent", &rec.agent_id);
        let conv = rec
            .conversation_id
            .as_deref()
            .filter(|s| !s.is_empty())
            .and_then(|s| Uuid::parse_str(s.trim()).ok());
        let wsr = rec
            .working_summary_ref
            .as_deref()
            .filter(|s| !s.is_empty())
            .and_then(|s| Uuid::parse_str(s.trim()).ok());
        let ts = parse_unix_ms_label(&rec.created_at).unwrap_or_else(Utc::now);
        let blob_json = Json(rec.runtime_state_blob.clone());
        let pol_json = Json(rec.policy_versions.clone());
        tx.execute(
            "INSERT INTO agent_runtime_checkpoints (
                id, agent_id, user_id, conversation_id, schema_version,
                working_summary_ref, runtime_state_blob, policy_versions_json, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            &[
                &cid,
                &agent,
                &user,
                &conv,
                &rec.schema_version,
                &wsr,
                &blob_json,
                &pol_json,
                &ts,
            ],
        )
        .await?;
        tx.execute(
            "INSERT INTO context_checkpoint_dedupe (dedupe_key, checkpoint_id) VALUES ($1, $2)",
            &[&dedupe_key, &cid],
        )
        .await?;
        tx.commit().await?;
        Ok(rec.checkpoint_id)
    }

    pub async fn consolidate_has_event(&self, event_id: &str) -> Result<bool, StoreError> {
        let client = self.pool.get().await?;
        let row = client
            .query_opt(
                "SELECT 1 FROM context_memory_consolidate_dedupe WHERE event_id = $1 LIMIT 1",
                &[&event_id],
            )
            .await?;
        Ok(row.is_some())
    }

    pub async fn consolidate_record(
        &self,
        event_id: &str,
        summary_id: &str,
    ) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        let sid = parse_uuid(summary_id, "summary_id")?;
        client
            .execute(
                "INSERT INTO context_memory_consolidate_dedupe (event_id, summary_id) VALUES ($1, $2)
                 ON CONFLICT (event_id) DO NOTHING",
                &[&event_id, &sid],
            )
            .await?;
        Ok(())
    }

    pub async fn observability_counts(&self) -> Result<ObsCounts, StoreError> {
        let client = self.pool.get().await?;
        let artifact_count: i64 = client
            .query_one("SELECT COUNT(*)::bigint FROM memory_artifacts", &[])
            .await?
            .get(0);
        let summary_count: i64 = client
            .query_one("SELECT COUNT(*)::bigint FROM memory_summaries", &[])
            .await?
            .get(0);
        let entity_count: i64 = client
            .query_one(
                "SELECT COUNT(*)::bigint FROM memory_entities WHERE entity_type <> '_memory_pointer'",
                &[],
            )
            .await?
            .get(0);
        let link_count: i64 = client
            .query_one("SELECT COUNT(*)::bigint FROM memory_entity_links", &[])
            .await?
            .get(0);
        let checkpoint_count: i64 = client
            .query_one(
                "SELECT COUNT(*)::bigint FROM agent_runtime_checkpoints",
                &[],
            )
            .await?
            .get(0);
        let latest: Option<String> = match client
            .query_opt(
                "SELECT policy_version FROM memory_summaries ORDER BY updated_at DESC NULLS LAST LIMIT 1",
                &[],
            )
            .await?
        {
            Some(r) => r.get::<_, Option<String>>(0),
            None => None,
        };
        Ok(ObsCounts {
            artifact_count: artifact_count as usize,
            summary_count: summary_count as usize,
            entity_count: entity_count as usize,
            link_count: link_count as usize,
            checkpoint_count: checkpoint_count as usize,
            latest_summary_policy_version: latest,
        })
    }

    pub async fn insert_context_log(&self, row: &ContextLogInsert) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        let user = parse_user_uuid(&row.user_id)?;
        let conv: Option<Uuid> = if row.conversation_id.trim().is_empty() {
            None
        } else {
            Some(conversation_uuid(&row.conversation_id))
        };
        let sum_ids: Vec<Uuid> = row
            .selected_summary_ids
            .iter()
            .map(|s| context_log_summary_uuid(s))
            .collect();
        let mem_ids: Vec<Uuid> = row
            .selected_memory_ids
            .iter()
            .map(|s| context_log_memory_uuid(s))
            .collect();
        let budget = json!({
            "max_tokens": row.max_tokens,
            "memory_limit": row.memory_limit,
            "summary_limit": row.summary_limit,
        });
        let budget_json = Json(budget);
        let task = row.task_type.as_str();
        client
            .execute(
                "INSERT INTO context_logs (
                    user_id, conversation_id, input_ref_id,
                    selected_summary_ids, selected_memory_ids, retrieval_modes,
                    task_type, token_budget_json, model_context_size
                ) VALUES ($1, $2, NULL, $3, $4, $5, $6, $7, NULL)",
                &[
                    &user,
                    &conv,
                    &sum_ids,
                    &mem_ids,
                    &row.retrieval_modes,
                    &task,
                    &budget_json,
                ],
            )
            .await?;
        Ok(())
    }

    pub async fn insert_routing_observation(
        &self,
        user_id: &str,
        obs: &RoutingObservation,
    ) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        let user = parse_user_uuid(user_id)?;
        let esc: Vec<String> = obs.escalation_reasons.clone();
        let modes = obs.retrieval_modes.clone();
        let res = client
            .execute(
                "INSERT INTO context_routing_observations (
                    user_id, executed_route, candidate_route, escalation_reasons, upgraded,
                    evidence_count, summary_hits, artifact_hits, entity_hits, conflict_count,
                    route_confidence, estimated_llm_calls, estimated_tokens, query_preview,
                    degraded, elapsed_ms, query_preference_polarity, evidence_preference_polarity,
                    retrieval_modes
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19
                )",
                &[
                    &user,
                    &obs.executed_route,
                    &obs.candidate_route,
                    &esc,
                    &obs.upgraded,
                    &(obs.evidence_count as i32),
                    &(obs.summary_hits as i32),
                    &(obs.artifact_hits as i32),
                    &(obs.entity_hits as i32),
                    &(obs.conflict_count as i32),
                    &obs.route_confidence,
                    &(obs.estimated_llm_calls as i32),
                    &(obs.estimated_tokens as i32),
                    &obs.query_preview,
                    &obs.degraded,
                    &(obs.elapsed_ms as i64),
                    &obs.query_preference_polarity,
                    &obs.evidence_preference_polarity,
                    &modes,
                ],
            )
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(e) if pg_is_undefined_table(&e) => {
                tracing::warn!("context_routing_observations missing; skip routing persist");
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }

    pub async fn insert_failure_event(&self, row: &FailureEventInsert) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        let user = parse_user_uuid(&row.user_id)?;
        let ts = parse_unix_ms_label(&row.created_at).unwrap_or_else(Utc::now);
        let res = client
            .execute(
                "INSERT INTO context_failure_events (
                    user_id, stage, category, detail, trace_id, retryable, attempt_count, created_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                &[
                    &user,
                    &row.stage,
                    &row.category,
                    &row.detail,
                    &row.trace_id,
                    &row.retryable,
                    &(row.attempt_count as i32),
                    &ts,
                ],
            )
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(e) if pg_is_undefined_table(&e) => {
                tracing::warn!("context_failure_events missing; skip failure persist");
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }

    pub async fn insert_forgetting_decision(
        &self,
        row: &ForgettingDecisionInsert,
    ) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        let user = parse_user_uuid(&row.user_id)?;
        let tid = forgetting_target_uuid(&row.target_id);
        let reasons = Json(row.reason_codes.clone());
        let res = client
            .execute(
                "INSERT INTO forgetting_decisions (
                    user_id, target_type, target_id, decision, reason_codes, policy_version, cold_storage_ref
                ) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                &[
                    &user,
                    &row.target_type,
                    &tid,
                    &row.decision,
                    &reasons,
                    &row.policy_version,
                    &row.cold_storage_ref,
                ],
            )
            .await;
        match res {
            Ok(_) => Ok(()),
            Err(e) if pg_is_undefined_table(&e) => {
                tracing::warn!("forgetting_decisions missing; skip insert");
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }

    pub async fn touch_artifacts(
        &self,
        ids: &[String],
        touched_at: &str,
    ) -> Result<(), StoreError> {
        if ids.is_empty() {
            return Ok(());
        }
        let uuids: Vec<Uuid> = ids
            .iter()
            .filter_map(|id| Uuid::parse_str(id.trim()).ok())
            .collect();
        if uuids.is_empty() {
            return Ok(());
        }
        let touched_at = parse_unix_ms_label(touched_at).unwrap_or_else(Utc::now);
        let client = self.pool.get().await?;
        client
            .execute(
                "UPDATE memory_artifacts
                 SET access_count = access_count + 1,
                     last_accessed_at = $2,
                     updated_at = GREATEST(updated_at, $2)
                 WHERE id = ANY($1)",
                &[&uuids, &touched_at],
            )
            .await?;
        Ok(())
    }

    pub async fn fetch_persistent_asmr_snapshot(
        &self,
    ) -> Result<Option<PersistentAsmrSnapshot>, StoreError> {
        let client = self.pool.get().await?;
        let total_failures: i64 = match client
            .query_one("SELECT COUNT(*)::bigint FROM context_failure_events", &[])
            .await
        {
            Ok(r) => r.get(0),
            Err(e) if pg_is_undefined_table(&e) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let last = match client
            .query_opt(
                "SELECT executed_route, candidate_route, escalation_reasons, upgraded,
                        evidence_count, summary_hits, artifact_hits, entity_hits, conflict_count,
                        route_confidence, estimated_llm_calls, estimated_tokens, query_preview,
                        degraded, elapsed_ms, query_preference_polarity, evidence_preference_polarity,
                        retrieval_modes
                 FROM context_routing_observations ORDER BY created_at DESC LIMIT 1",
                &[],
            )
            .await
        {
            Ok(r) => r,
            Err(e) if pg_is_undefined_table(&e) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let last_observation = last.map(|r| RoutingObservation {
            executed_route: r.get(0),
            candidate_route: r.get(1),
            escalation_reasons: r.get(2),
            upgraded: r.get(3),
            evidence_count: r.get::<_, i32>(4) as usize,
            summary_hits: r.get::<_, i32>(5) as usize,
            artifact_hits: r.get::<_, i32>(6) as usize,
            entity_hits: r.get::<_, i32>(7) as usize,
            conflict_count: r.get::<_, i32>(8) as usize,
            route_confidence: r.get(9),
            estimated_llm_calls: r.get::<_, i32>(10) as u32,
            estimated_tokens: r.get::<_, i64>(11) as usize,
            query_preview: r.get(12),
            degraded: r.get(13),
            elapsed_ms: r.get::<_, i64>(14) as u128,
            query_preference_polarity: r.get(15),
            evidence_preference_polarity: r.get(16),
            retrieval_modes: r.get(17),
        });
        let recent_rows = match client
            .query(
                "SELECT id, user_id, stage, category, detail, trace_id, retryable, attempt_count, created_at
                 FROM context_failure_events ORDER BY created_at DESC LIMIT 10",
                &[],
            )
            .await
        {
            Ok(r) => r,
            Err(e) if pg_is_undefined_table(&e) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let recent_failures: Vec<FailureCaseRecord> = recent_rows
            .iter()
            .map(|r| FailureCaseRecord {
                id: r.get::<_, Uuid>(0).to_string(),
                user_id: r.get::<_, Uuid>(1).to_string(),
                stage: r.get(2),
                category: r.get(3),
                detail: r.get(4),
                trace_id: r.get(5),
                retryable: r.get(6),
                attempt_count: r.get::<_, i32>(7) as u32,
                created_at: ts_to_unix_label(r.get(8)),
            })
            .collect();
        Ok(Some(PersistentAsmrSnapshot {
            last_observation,
            recent_failures,
            total_failures: total_failures as u64,
        }))
    }

    pub async fn load_policy_overrides(&self) -> Result<PolicyDbOverrides, StoreError> {
        let client = self.pool.get().await?;
        let rows = match client
            .query(
                "SELECT policy_key, COALESCE(NULLIF(TRIM(current_value), ''), NULLIF(TRIM(default_value), '')) AS v
                 FROM policy_configs WHERE status = 'active'",
                &[],
            )
            .await
        {
            Ok(r) => r,
            Err(e) if pg_is_undefined_table(&e) => {
                tracing::info!("policy_configs table missing; using in-process policy defaults");
                return Ok(PolicyDbOverrides::default());
            }
            Err(e) => return Err(e.into()),
        };
        let mut o = PolicyDbOverrides::default();
        for row in rows {
            let key: String = row.get(0);
            let val: Option<String> = row.get(1);
            let Some(v) = val.filter(|s| !s.is_empty()) else {
                continue;
            };
            match key.as_str() {
                "memory_policy_version" => o.memory_policy_version = Some(v),
                "session_policy_version" => o.session_policy_version = Some(v),
                "retrieval_policy_version" => o.retrieval_policy_version = Some(v),
                "default_reply_style" => o.default_reply_style = Some(v),
                "enabled_retrieval_modes" => {
                    o.enabled_retrieval_modes = parse_retrieval_modes_value(&v);
                }
                "graph_enabled" => o.graph_enabled = parse_bool_loose(&v),
                "rerank_enabled" => o.rerank_enabled = parse_bool_loose(&v),
                "activation_decay_rate" => o.activation_decay_rate = parse_f64_loose(&v),
                "score_activation_weight" => o.score_activation_weight = parse_f64_loose(&v),
                "importance_score_default" => o.importance_score_default = parse_f64_loose(&v),
                "compressible_evaluation_enabled" => {
                    o.compressible_evaluation_enabled = parse_bool_loose(&v)
                }
                _ => {}
            }
        }
        Ok(o)
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

fn pg_is_undefined_table(e: &tokio_postgres::Error) -> bool {
    e.code() == Some(&SqlState::UNDEFINED_TABLE)
}

fn forgetting_target_uuid(s: &str) -> Uuid {
    Uuid::parse_str(s.trim()).unwrap_or_else(|_| {
        Uuid::new_v5(
            &Uuid::NAMESPACE_DNS,
            format!("onelink:forgetting:target:{s}").as_bytes(),
        )
    })
}

fn parse_bool_loose(s: &str) -> Option<bool> {
    match s.trim().to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn parse_f64_loose(s: &str) -> Option<f64> {
    let value = s.trim().parse::<f64>().ok()?;
    if value.is_finite() {
        Some(value)
    } else {
        None
    }
}

fn parse_retrieval_modes_value(raw: &str) -> Option<Vec<String>> {
    let t = raw.trim();
    if let Ok(v) = serde_json::from_str::<Vec<String>>(t) {
        if v.is_empty() {
            return None;
        }
        return Some(v);
    }
    let split: Vec<String> = t
        .split([',', ' '])
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if split.is_empty() {
        None
    } else {
        Some(split)
    }
}

async fn ensure_memory_pointer(
    client: &impl PgGenericClient,
    user: Uuid,
    mem_id: Uuid,
    memory_id_str: &str,
) -> Result<(), StoreError> {
    client
        .execute(
            "INSERT INTO memory_entities (id, user_id, entity_type, name, attributes, created_at, updated_at)
             VALUES ($1, $2, '_memory_pointer', $3, '{}'::jsonb, now(), now())
             ON CONFLICT (id) DO NOTHING",
            &[&mem_id, &user, &memory_id_str],
        )
        .await?;
    Ok(())
}

fn artifact_structured_json(rec: &MemoryArtifactRecord) -> Value {
    json!({
        "conversation_id": rec.conversation_id,
        "keywords": rec.keywords,
        "temporal_state": rec.temporal_state,
        "supersedes_previous": rec.supersedes_previous,
        "preference_polarity": rec.preference_polarity,
        "source_message_id": rec.source_message_id,
        "importance_score": rec.importance_score,
        "last_accessed_at": rec.last_accessed_at,
        "access_count": rec.access_count,
    })
}

fn map_source_type(s: &str) -> &'static str {
    match s {
        "questionnaire" => "questionnaire",
        "behavior" => "behavior",
        _ => "chat",
    }
}

fn parse_user_uuid(user_id: &str) -> Result<Uuid, StoreError> {
    Uuid::parse_str(user_id.trim()).map_err(|_| StoreError::InvalidUserId(user_id.to_string()))
}

fn parse_uuid(s: &str, ctx: &str) -> Result<Uuid, StoreError> {
    Uuid::parse_str(s.trim()).map_err(|_| StoreError::InvalidId(format!("{ctx}: {s}")))
}

fn stable_entity_uuid(rust_id: &str) -> Uuid {
    Uuid::parse_str(rust_id.trim()).unwrap_or_else(|_| {
        Uuid::new_v5(
            &Uuid::NAMESPACE_OID,
            format!("onelink:entity:{rust_id}").as_bytes(),
        )
    })
}

fn stable_uuid_label(label: &str, s: &str) -> Uuid {
    Uuid::parse_str(s.trim()).unwrap_or_else(|_| {
        Uuid::new_v5(
            &Uuid::NAMESPACE_DNS,
            format!("onelink:{label}:{s}").as_bytes(),
        )
    })
}

fn link_uuid(link_id: &str) -> Uuid {
    Uuid::parse_str(link_id.trim()).unwrap_or_else(|_| {
        Uuid::new_v5(
            &Uuid::NAMESPACE_DNS,
            format!("onelink:entity_link:{link_id}").as_bytes(),
        )
    })
}

fn conversation_uuid(s: &str) -> Uuid {
    Uuid::parse_str(s.trim()).unwrap_or_else(|_| {
        Uuid::new_v5(
            &Uuid::NAMESPACE_DNS,
            format!("onelink:conversation:{s}").as_bytes(),
        )
    })
}

/// Deterministic UUID for `context_logs.selected_summary_ids` when the L1 id string is not a UUID.
fn context_log_summary_uuid(s: &str) -> Uuid {
    Uuid::parse_str(s.trim()).unwrap_or_else(|_| {
        Uuid::new_v5(
            &Uuid::NAMESPACE_DNS,
            format!("onelink:context_log:summary:{s}").as_bytes(),
        )
    })
}

/// Deterministic UUID for `context_logs.selected_memory_ids` when the L1 id string is not a UUID.
fn context_log_memory_uuid(s: &str) -> Uuid {
    Uuid::parse_str(s.trim()).unwrap_or_else(|_| {
        Uuid::new_v5(
            &Uuid::NAMESPACE_DNS,
            format!("onelink:context_log:memory:{s}").as_bytes(),
        )
    })
}

fn parse_unix_ms_label(s: &str) -> Option<DateTime<Utc>> {
    let n = s.strip_prefix("unix-ms:")?.parse::<i64>().ok()?;
    DateTime::from_timestamp_millis(n)
}

fn ts_to_unix_label(ts: DateTime<Utc>) -> String {
    format!("unix-ms:{}", ts.timestamp_millis())
}

fn row_to_artifact(row: &tokio_postgres::Row) -> Result<MemoryArtifactRecord, StoreError> {
    let mid: Uuid = row.get(0);
    let user: Uuid = row.get(1);
    let structured: Option<Value> = row.get(11);
    let conv = structured
        .as_ref()
        .and_then(|v| v.get("conversation_id"))
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let keywords = structured
        .as_ref()
        .and_then(|v| v.get("keywords"))
        .and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let temporal_state = structured
        .as_ref()
        .and_then(|v| v.get("temporal_state"))
        .and_then(|x| x.as_str())
        .unwrap_or("timeless")
        .to_string();
    let supersedes_previous = structured
        .as_ref()
        .and_then(|v| v.get("supersedes_previous"))
        .and_then(|x| x.as_bool())
        .unwrap_or(false);
    let preference_polarity = structured
        .as_ref()
        .and_then(|v| v.get("preference_polarity"))
        .and_then(|x| x.as_str())
        .unwrap_or("neutral")
        .to_string();
    let source_message_id = structured
        .as_ref()
        .and_then(|v| v.get("source_message_id"))
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
        .or_else(|| row.get::<_, Option<Uuid>>(13).map(|u| u.to_string()))
        .unwrap_or_default();
    let last_accessed_at: Option<DateTime<Utc>> = row.get(9);
    let created_at: DateTime<Utc> = row.get(14);
    Ok(MemoryArtifactRecord {
        memory_id: mid.to_string(),
        user_id: user.to_string(),
        conversation_id: conv,
        source_message_id,
        content: row.get(5),
        network_type: row.get(2),
        evidence_type: row.get(3),
        memory_level: row.get(4),
        source_type: row.get(6),
        confidence: row.get::<_, Option<f64>>(7).unwrap_or(0.5),
        importance_score: row.get::<_, Option<f64>>(8).unwrap_or(0.5),
        keywords,
        temporal_state,
        supersedes_previous,
        preference_polarity,
        last_accessed_at: last_accessed_at
            .map(ts_to_unix_label)
            .unwrap_or_else(|| ts_to_unix_label(created_at)),
        access_count: row.get::<_, i32>(10).max(0) as u32,
        created_at: ts_to_unix_label(created_at),
    })
}

fn row_to_summary(row: &tokio_postgres::Row) -> Result<MemorySummaryRecord, StoreError> {
    let sid: Uuid = row.get(0);
    let user: Uuid = row.get(1);
    let conv: Uuid = row.get(2);
    let kp: Option<Value> = row.get(5);
    let smr: Option<Value> = row.get(6);
    let memory_ids = kp
        .as_ref()
        .and_then(|v| v.get("memory_ids"))
        .and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let keywords = kp
        .as_ref()
        .and_then(|v| v.get("keywords"))
        .and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let temporal_state = kp
        .as_ref()
        .and_then(|v| v.get("temporal_state"))
        .and_then(|x| x.as_str())
        .unwrap_or("timeless")
        .to_string();
    let supersedes_previous = kp
        .as_ref()
        .and_then(|v| v.get("supersedes_previous"))
        .and_then(|x| x.as_bool())
        .unwrap_or(false);
    let preference_polarity = kp
        .as_ref()
        .and_then(|v| v.get("preference_polarity"))
        .and_then(|x| x.as_str())
        .unwrap_or("neutral")
        .to_string();
    let source_message_ids = smr
        .as_ref()
        .and_then(|v| v.get("source_message_ids"))
        .and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let updated_at: DateTime<Utc> = row.get(8);
    Ok(MemorySummaryRecord {
        summary_id: sid.to_string(),
        user_id: user.to_string(),
        conversation_id: conv.to_string(),
        summary_type: row.get(3),
        summary_text: row.get(4),
        memory_ids,
        source_message_ids,
        keywords,
        temporal_state,
        supersedes_previous,
        preference_polarity,
        updated_at: ts_to_unix_label(updated_at),
        policy_version: row.get::<_, Option<String>>(7).unwrap_or_default(),
    })
}

fn row_to_entity(row: &tokio_postgres::Row) -> Result<MemoryEntityRecord, StoreError> {
    let attrs: Option<Value> = row.get(4);
    let rust_id = attrs
        .as_ref()
        .and_then(|v| v.get("rust_entity_id"))
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| row.get::<_, Uuid>(0).to_string());
    let user: Uuid = row.get(1);
    Ok(MemoryEntityRecord {
        id: rust_id,
        user_id: user.to_string(),
        entity_type: row.get(2),
        name: row.get(3),
    })
}

fn row_to_entity_link(row: &tokio_postgres::Row) -> Result<MemoryEntityLinkRecord, StoreError> {
    let lid: Uuid = row.get(0);
    let user: Uuid = row.get(1);
    let attrs: Option<Value> = row.get(2);
    let entity_id = attrs
        .as_ref()
        .and_then(|v| v.get("rust_entity_id"))
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let memory_id: String = row.get(3);
    Ok(MemoryEntityLinkRecord {
        id: lid.to_string(),
        user_id: user.to_string(),
        entity_id,
        memory_id,
        relationship: row.get(4),
        confidence: row.get::<_, Option<f64>>(5).unwrap_or(0.0),
    })
}

#[cfg(test)]
mod policy_value_parse_tests {
    use super::*;

    #[test]
    fn parse_bool_loose_accepts_and_rejects() {
        assert_eq!(parse_bool_loose("true"), Some(true));
        assert_eq!(parse_bool_loose("YES"), Some(true));
        assert_eq!(parse_bool_loose("0"), Some(false));
        assert_eq!(parse_bool_loose("maybe"), None);
        assert_eq!(parse_bool_loose(""), None);
        assert_eq!(parse_bool_loose("2"), None);
    }

    #[test]
    fn parse_retrieval_modes_json_and_split() {
        let v = parse_retrieval_modes_value(r#"["structured","graph"]"#).unwrap();
        assert_eq!(v, vec!["structured", "graph"]);
        let v2 = parse_retrieval_modes_value("semantic, temporal").unwrap();
        assert_eq!(v2, vec!["semantic", "temporal"]);
        assert!(parse_retrieval_modes_value("   ").is_none());
        assert!(parse_retrieval_modes_value("[]").is_none());
    }
}
