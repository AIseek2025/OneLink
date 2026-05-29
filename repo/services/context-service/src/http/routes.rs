//! Internal + dev-only HTTP relay (`OneLink/docs/archive/rules-legacy-2026-05-15/Rules/16` envelope).

use std::sync::Arc;
use std::time::Instant;

use axum::{
    body::Bytes,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use onelink_event_envelope::EventEnvelope;
use onelink_internal_auth::{
    observability_ip_allowlist::ip_allowlist_layer, verify_internal_token, INTERNAL_TOKEN_HEADER,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::time::{sleep, Duration};
use uuid::Uuid;

use crate::{
    app_state::{ContextAppState, MemoryArtifactRecord},
    context_builder::{assemble_query_aware_context, CompressionMode},
    evidence, l1_policy,
    memory_distiller::{distill_message, ConsolidationMode},
    memory_extractor::heuristic_extract,
    memory_store::{
        FailureCaseRecord, MemoryEntityLinkRecord, MemoryEntityRecord, MemorySearchItem,
        MemorySummaryRecord, RoutingObservation, RuntimeCheckpointRecord,
    },
    store::{ContextLogInsert, ForgettingDecisionInsert},
    task_router::{decide_route, TaskType},
};

pub fn router(state: Arc<ContextAppState>) -> Router {
    let internal = Router::new()
        .route("/internal/context/build", post(build_context))
        .route("/internal/session/checkpoint", post(save_checkpoint))
        .route("/internal/memory/write", post(write_memory))
        .route("/internal/memory/search", get(search_memory))
        .route("/internal/memory/consolidate", post(consolidate_memory))
        .route("/internal/memory/resolve", post(memory_resolve))
        .route(
            "/internal/memory/forgetting/decide",
            post(forgetting_decide),
        )
        .route("/internal/events/receive", post(receive_event))
        .with_state(state.clone());

    let observability = Router::new()
        .route(
            "/internal/observability/asmr-lite",
            get(get_asmr_lite_observability),
        )
        .with_state(state)
        .layer(axum::middleware::from_fn(ip_allowlist_layer));

    internal.merge(observability)
}

async fn receive_event(
    State(state): State<Arc<ContextAppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    if let Err(code) = verify_internal_token(&headers, &state.config.internal_shared_secret) {
        return code;
    }

    let envelope: EventEnvelope = match serde_json::from_slice(&body) {
        Ok(e) => e,
        Err(err) => {
            tracing::warn!(error = %err, "context-service /internal/events/receive: invalid EventEnvelope body");
            return StatusCode::BAD_REQUEST;
        }
    };

    tracing::info!(
        event_name = %envelope.event_name,
        producer = %envelope.producer,
        full_envelope = %serde_json::to_string(&envelope).unwrap_or_default(),
        "context-service POST /internal/events/receive"
    );

    if envelope.event_name == "chat.user_message.created.v1" {
        let st = Arc::clone(&state);
        tokio::spawn(async move {
            if let Err(err) = pipeline_chat_user_message(st, envelope).await {
                tracing::warn!(error = %err, "chat.user_message.created.v1 pipeline failed");
            }
        });
    } else if envelope.event_name == "question.answered.v1" {
        let st = Arc::clone(&state);
        tokio::spawn(async move {
            if let Err(err) = pipeline_question_answered(st, envelope).await {
                tracing::warn!(error = %err, "question.answered.v1 pipeline failed");
            }
        });
    }

    StatusCode::ACCEPTED
}

const QUESTIONNAIRE_CONV_ID: &str = "__questionnaire__";

fn questionnaire_extraction_text(payload: &Value) -> Option<String> {
    if let Some(t) = payload.get("answer_text").and_then(|v| v.as_str()) {
        let s = t.trim();
        if !s.is_empty() {
            return Some(s.to_string());
        }
    }
    let q = payload
        .get("question_text")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let ap = payload
        .get("answer_payload")
        .cloned()
        .unwrap_or(Value::Null);
    if q.is_empty() && (ap.is_null() || ap == json!({})) {
        return None;
    }
    Some(format!("问卷：{q} {}", ap))
}

fn initial_importance_score(
    state: &ContextAppState,
    source_type: &str,
    confidence: f64,
    supersedes_previous: bool,
) -> f64 {
    if source_type == "questionnaire" {
        0.9
    } else if supersedes_previous {
        0.8
    } else if source_type == "chat" && confidence >= 0.7 {
        0.7
    } else if source_type == "chat" {
        0.5
    } else if source_type == "behavior" {
        0.6
    } else {
        state.policy.importance_score_default
    }
}

async fn pipeline_question_answered(
    state: Arc<ContextAppState>,
    q_envelope: EventEnvelope,
) -> Result<(), String> {
    let payload = &q_envelope.payload;
    let answer_state = payload
        .get("answer_state")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if answer_state != "answered" {
        tracing::info!("question.answered.v1 skipped (not answered)");
        return Ok(());
    }
    let event_user_id = payload
        .get("user_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing user_id".to_string())?
        .to_string();
    let answer_id = payload
        .get("answer_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing answer_id".to_string())?
        .to_string();
    let trace_id = q_envelope.trace_id.clone();

    let content_text = questionnaire_extraction_text(payload)
        .ok_or_else(|| "missing answer_text / question_text+answer_payload".to_string())?;
    if content_text.trim().is_empty() {
        return Err("empty questionnaire extraction text".to_string());
    }

    if state.store.is_postgres() {
        state
            .store
            .ensure_user_exists(&event_user_id)
            .await
            .map_err(|e| format!("ensure user (questionnaire): {e}"))?;
    }
    let candidates = heuristic_extract(&content_text);
    let distilled = distill_message(&content_text, &candidates);
    let mut memory_ids: Vec<String> = vec![];
    let now = state.now_string();
    let policy_version = state.policy.policy_version_label();
    let conversation_id = QUESTIONNAIRE_CONV_ID.to_string();
    for c in &candidates {
        let memory_id = state.next_id();
        let content = c
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let network_type = c
            .get("network_type")
            .and_then(|v| v.as_str())
            .unwrap_or("experience")
            .to_string();
        let evidence_type = c
            .get("evidence_type")
            .and_then(|v| v.as_str())
            .unwrap_or("fact")
            .to_string();
        let memory_level = c
            .get("memory_level")
            .and_then(|v| v.as_str())
            .unwrap_or("persistent")
            .to_string();
        let confidence = c.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.5);
        let importance_score = initial_importance_score(
            &state,
            "questionnaire",
            confidence,
            distilled.supersedes_previous,
        );
        let rec = MemoryArtifactRecord {
            memory_id: memory_id.clone(),
            user_id: event_user_id.clone(),
            conversation_id: conversation_id.clone(),
            source_message_id: answer_id.clone(),
            content,
            network_type,
            evidence_type,
            memory_level,
            source_type: "questionnaire".to_string(),
            confidence,
            importance_score,
            keywords: distilled.keywords.clone(),
            temporal_state: distilled.temporal_state.clone(),
            supersedes_previous: distilled.supersedes_previous,
            preference_polarity: distilled.preference_polarity.clone(),
            last_accessed_at: now.clone(),
            access_count: 0,
            created_at: now.clone(),
        };
        state
            .store
            .insert_artifact(rec)
            .await
            .map_err(|e| format!("persist artifact (questionnaire): {e}"))?;
        memory_ids.push(memory_id);
    }
    let summary_id = state.next_id();
    state
        .store
        .insert_summary(MemorySummaryRecord {
            summary_id: summary_id.clone(),
            user_id: event_user_id.clone(),
            conversation_id: conversation_id.clone(),
            summary_type: "questionnaire".to_string(),
            summary_text: distilled.summary_text.clone(),
            memory_ids: memory_ids.clone(),
            source_message_ids: vec![answer_id.clone()],
            keywords: distilled.keywords.clone(),
            temporal_state: distilled.temporal_state.clone(),
            supersedes_previous: distilled.supersedes_previous,
            preference_polarity: distilled.preference_polarity.clone(),
            updated_at: now.clone(),
            policy_version: policy_version.clone(),
        })
        .await
        .map_err(|e| format!("persist summary (questionnaire): {e}"))?;
    if !distilled.entities.is_empty() {
        for entity in &distilled.entities {
            let entity_id = format!(
                "entity:{}:{}:{}",
                event_user_id,
                entity.entity_type,
                entity.name.to_lowercase()
            );
            state
                .store
                .upsert_entity(MemoryEntityRecord {
                    id: entity_id.clone(),
                    user_id: event_user_id.clone(),
                    entity_type: entity.entity_type.clone(),
                    name: entity.name.clone(),
                })
                .await
                .map_err(|e| format!("persist entity (questionnaire): {e}"))?;
            for memory_id in &memory_ids {
                let link_id = format!("{entity_id}:{memory_id}");
                state
                    .store
                    .upsert_entity_link(MemoryEntityLinkRecord {
                        id: link_id,
                        user_id: event_user_id.clone(),
                        entity_id: entity_id.clone(),
                        memory_id: memory_id.clone(),
                        relationship: entity.relationship.clone(),
                        confidence: entity.confidence,
                    })
                    .await
                    .map_err(|e| format!("persist entity_link (questionnaire): {e}"))?;
            }
        }
    }

    let extracted = EventEnvelope::new_v1(
        "context.memory.extracted.v1",
        "context-service",
        Some(event_user_id.clone()),
        q_envelope.trace_id.clone(),
        json!({
            "user_id": event_user_id.clone(),
            "conversation_id": conversation_id.clone(),
            "source_event_id": q_envelope.event_id,
            "source_event_name": "question.answered.v1",
            "artifact_candidates": candidates,
        }),
    );
    tracing::info!(
        envelope = %serde_json::to_string(&extracted).unwrap_or_default(),
        "context.memory.extracted.v1 (questionnaire)"
    );

    let summary = EventEnvelope::new_v1(
        "context.memory.summary.updated.v1",
        "context-service",
        Some(event_user_id.clone()),
        extracted.trace_id.clone(),
        json!({
            "user_id": event_user_id.clone(),
            "conversation_id": conversation_id.clone(),
            "summary_id": summary_id.clone(),
            "summary_type": "questionnaire",
            "token_count": content_text.chars().count(),
            "source_message_range": {
                "from_message_id": answer_id.clone(),
                "to_message_id": answer_id.clone()
            }
        }),
    );
    tracing::info!(
        envelope = %serde_json::to_string(&summary).unwrap_or_default(),
        "context.memory.summary.updated.v1 (questionnaire)"
    );

    if memory_ids.is_empty() {
        tracing::info!("no memory ids; skip profile.memory_projection (questionnaire)");
        return Ok(());
    }

    let projection = EventEnvelope::new_v1(
        "profile.memory_projection.requested.v1",
        "context-service",
        Some(event_user_id.clone()),
        extracted.trace_id.clone(),
        json!({
            "user_id": event_user_id.clone(),
            "projection_id": Uuid::new_v4().to_string(),
            "memory_ids": memory_ids,
            "conversation_id": conversation_id,
            "source_event_id": extracted.event_id,
            "projection_reason": "questionnaire_answer"
        }),
    );
    tracing::info!(
        envelope = %serde_json::to_string(&projection).unwrap_or_default(),
        "relay profile.memory_projection.requested.v1 (questionnaire) -> profile-service"
    );

    let relay_url = format!(
        "{}/internal/events/receive",
        state.config.profile_service_base_url
    );
    for attempt in 1..=3_u32 {
        match state
            .client
            .post(relay_url.clone())
            .header(
                INTERNAL_TOKEN_HEADER,
                state.config.internal_shared_secret.as_str(),
            )
            .json(&projection)
            .send()
            .await
        {
            Ok(relay_response) if relay_response.status().is_success() => return Ok(()),
            Ok(relay_response) => {
                state
                    .record_failure(
                        &event_user_id,
                        "context_profile_projection",
                        "profile_projection_non_success",
                        format!(
                            "status={} attempt={} projection_id={} (questionnaire)",
                            relay_response.status(),
                            attempt,
                            projection.event_id
                        ),
                        trace_id.clone(),
                        true,
                        attempt,
                    )
                    .await;
                if attempt == 3 {
                    return Err(format!(
                        "profile relay status {} (questionnaire)",
                        relay_response.status()
                    ));
                }
            }
            Err(error) => {
                state
                    .record_failure(
                        &event_user_id,
                        "context_profile_projection",
                        "profile_projection_request_failed",
                        format!("error={error} attempt={attempt} (questionnaire)"),
                        trace_id.clone(),
                        true,
                        attempt,
                    )
                    .await;
                if attempt == 3 {
                    return Err(format!("profile relay (questionnaire): {error}"));
                }
            }
        }
        if attempt < 3 {
            sleep(Duration::from_millis(120 * u64::from(attempt))).await;
        }
    }

    Ok(())
}

async fn pipeline_chat_user_message(
    state: Arc<ContextAppState>,
    chat_envelope: EventEnvelope,
) -> Result<(), String> {
    let payload = &chat_envelope.payload;
    let conversation_id = payload
        .get("conversation_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing conversation_id".to_string())?
        .to_string();
    let message_id = payload
        .get("message_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let event_user_id = payload
        .get("user_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing user_id".to_string())?
        .to_string();
    let trace_id = chat_envelope.trace_id.clone();

    let url = format!(
        "{}/internal/chat/conversations/{}/messages/{}",
        state.config.ai_chat_service_base_url, conversation_id, message_id
    );
    let content_text = match state
        .client
        .get(&url)
        .header(
            INTERNAL_TOKEN_HEADER,
            state.config.internal_shared_secret.as_str(),
        )
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => match resp.json::<AiChatMessageBody>().await {
            Ok(msg) => {
                if msg.owner_user_id != event_user_id {
                    state
                        .record_failure(
                            &event_user_id,
                            "context_event_validation",
                            "message_owner_mismatch",
                            format!(
                                "conversation_id={} message_id={} event_user_id={} owner_user_id={}",
                                conversation_id, message_id, event_user_id, msg.owner_user_id
                            ),
                            trace_id.clone(),
                            false,
                            1,
                        )
                        .await;
                    return Err(
                        "event user_id does not match ai-chat conversation owner".to_string()
                    );
                }
                msg.content_text
            }
            Err(e) => {
                return Err(format!("ai-chat message decode: {e}"));
            }
        },
        Ok(resp) => {
            let fallback = payload
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !fallback.trim().is_empty() {
                tracing::warn!(
                    status = %resp.status(),
                    "ai-chat message fetch non-success; falling back to payload content"
                );
                fallback.to_string()
            } else {
                state
                    .record_failure(
                        &event_user_id,
                        "context_message_fetch",
                        "message_fetch_non_success",
                        format!("ai-chat message fetch status {}", resp.status()),
                        trace_id.clone(),
                        true,
                        1,
                    )
                    .await;
                return Err(format!("ai-chat message fetch status {}", resp.status()));
            }
        }
        Err(e) => {
            let fallback = payload
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !fallback.trim().is_empty() {
                tracing::warn!(
                    error = %e,
                    "ai-chat fetch failed; falling back to payload content"
                );
                fallback.to_string()
            } else {
                state
                    .record_failure(
                        &event_user_id,
                        "context_message_fetch",
                        "message_fetch_error",
                        format!("ai-chat fetch message: {e}"),
                        trace_id.clone(),
                        true,
                        1,
                    )
                    .await;
                return Err(format!("ai-chat fetch message: {e}"));
            }
        }
    };

    if state.store.is_postgres() {
        state
            .store
            .ensure_user_exists(&event_user_id)
            .await
            .map_err(|e| format!("ensure user (chat): {e}"))?;
    }
    let candidates = heuristic_extract(&content_text);
    let distilled = distill_message(&content_text, &candidates);
    let mut memory_ids: Vec<String> = vec![];
    let now = state.now_string();
    let policy_version = state.policy.policy_version_label();
    for c in &candidates {
        let memory_id = state.next_id();
        let content = c
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let network_type = c
            .get("network_type")
            .and_then(|v| v.as_str())
            .unwrap_or("experience")
            .to_string();
        let evidence_type = c
            .get("evidence_type")
            .and_then(|v| v.as_str())
            .unwrap_or("fact")
            .to_string();
        let memory_level = c
            .get("memory_level")
            .and_then(|v| v.as_str())
            .unwrap_or("working")
            .to_string();
        let source_type = c
            .get("source_type")
            .and_then(|v| v.as_str())
            .unwrap_or("chat")
            .to_string();
        let confidence = c.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.5);
        let importance_score = initial_importance_score(
            &state,
            &source_type,
            confidence,
            distilled.supersedes_previous,
        );
        let rec = MemoryArtifactRecord {
            memory_id: memory_id.clone(),
            user_id: event_user_id.clone(),
            conversation_id: conversation_id.clone(),
            source_message_id: message_id.clone(),
            content,
            network_type,
            evidence_type,
            memory_level,
            source_type,
            confidence,
            importance_score,
            keywords: distilled.keywords.clone(),
            temporal_state: distilled.temporal_state.clone(),
            supersedes_previous: distilled.supersedes_previous,
            preference_polarity: distilled.preference_polarity.clone(),
            last_accessed_at: now.clone(),
            access_count: 0,
            created_at: now.clone(),
        };
        state
            .store
            .insert_artifact(rec)
            .await
            .map_err(|e| format!("persist artifact (chat): {e}"))?;
        memory_ids.push(memory_id);
    }
    let summary_id = state.next_id();
    state
        .store
        .insert_summary(MemorySummaryRecord {
            summary_id: summary_id.clone(),
            user_id: event_user_id.clone(),
            conversation_id: conversation_id.clone(),
            summary_type: "working_memory".to_string(),
            summary_text: distilled.summary_text.clone(),
            memory_ids: memory_ids.clone(),
            source_message_ids: vec![message_id.clone()],
            keywords: distilled.keywords.clone(),
            temporal_state: distilled.temporal_state.clone(),
            supersedes_previous: distilled.supersedes_previous,
            preference_polarity: distilled.preference_polarity.clone(),
            updated_at: now.clone(),
            policy_version: policy_version.clone(),
        })
        .await
        .map_err(|e| format!("persist summary (chat): {e}"))?;
    if !distilled.entities.is_empty() {
        for entity in &distilled.entities {
            let entity_id = format!(
                "entity:{}:{}:{}",
                event_user_id,
                entity.entity_type,
                entity.name.to_lowercase()
            );
            state
                .store
                .upsert_entity(MemoryEntityRecord {
                    id: entity_id.clone(),
                    user_id: event_user_id.clone(),
                    entity_type: entity.entity_type.clone(),
                    name: entity.name.clone(),
                })
                .await
                .map_err(|e| format!("persist entity (chat): {e}"))?;
            for memory_id in &memory_ids {
                let link_id = format!("{entity_id}:{memory_id}");
                state
                    .store
                    .upsert_entity_link(MemoryEntityLinkRecord {
                        id: link_id,
                        user_id: event_user_id.clone(),
                        entity_id: entity_id.clone(),
                        memory_id: memory_id.clone(),
                        relationship: entity.relationship.clone(),
                        confidence: entity.confidence,
                    })
                    .await
                    .map_err(|e| format!("persist entity_link (chat): {e}"))?;
            }
        }
    }

    let extracted = EventEnvelope::new_v1(
        "context.memory.extracted.v1",
        "context-service",
        Some(event_user_id.clone()),
        chat_envelope.trace_id.clone(),
        json!({
            "user_id": event_user_id.clone(),
            "conversation_id": conversation_id.clone(),
            "source_event_id": chat_envelope.event_id,
            "source_event_name": "chat.user_message.created.v1",
            "artifact_candidates": candidates,
        }),
    );
    tracing::info!(
        envelope = %serde_json::to_string(&extracted).unwrap_or_default(),
        "context.memory.extracted.v1 (producer=context-service; self-consume OK)"
    );

    let summary = EventEnvelope::new_v1(
        "context.memory.summary.updated.v1",
        "context-service",
        Some(event_user_id.clone()),
        extracted.trace_id.clone(),
        json!({
            "user_id": event_user_id.clone(),
            "conversation_id": conversation_id.clone(),
            "summary_id": summary_id.clone(),
            "summary_type": "working_memory",
            "token_count": content_text.chars().count(),
            "source_message_range": {
                "from_message_id": message_id.clone(),
                "to_message_id": message_id.clone()
            }
        }),
    );
    tracing::info!(
        envelope = %serde_json::to_string(&summary).unwrap_or_default(),
        "context.memory.summary.updated.v1 (producer=context-service; self-consume OK)"
    );

    if memory_ids.is_empty() {
        tracing::info!("no memory ids; skip profile.memory_projection.requested.v1");
        return Ok(());
    }

    let projection = EventEnvelope::new_v1(
        "profile.memory_projection.requested.v1",
        "context-service",
        Some(event_user_id.clone()),
        extracted.trace_id.clone(),
        json!({
            "user_id": event_user_id.clone(),
            "projection_id": Uuid::new_v4().to_string(),
            "memory_ids": memory_ids,
            "conversation_id": conversation_id,
            "source_event_id": extracted.event_id,
            "projection_reason": "chat_distillation"
        }),
    );
    tracing::info!(
        envelope = %serde_json::to_string(&projection).unwrap_or_default(),
        "relay profile.memory_projection.requested.v1 -> profile-service"
    );

    let relay_url = format!(
        "{}/internal/events/receive",
        state.config.profile_service_base_url
    );
    for attempt in 1..=3_u32 {
        match state
            .client
            .post(relay_url.clone())
            .header(
                INTERNAL_TOKEN_HEADER,
                state.config.internal_shared_secret.as_str(),
            )
            .json(&projection)
            .send()
            .await
        {
            Ok(relay_response) if relay_response.status().is_success() => return Ok(()),
            Ok(relay_response) => {
                state
                    .record_failure(
                        &event_user_id,
                        "context_profile_projection",
                        "profile_projection_non_success",
                        format!(
                            "status={} attempt={} projection_id={}",
                            relay_response.status(),
                            attempt,
                            projection.event_id
                        ),
                        trace_id.clone(),
                        true,
                        attempt,
                    )
                    .await;
                if attempt == 3 {
                    return Err(format!("profile relay status {}", relay_response.status()));
                }
            }
            Err(error) => {
                state
                    .record_failure(
                        &event_user_id,
                        "context_profile_projection",
                        "profile_projection_request_failed",
                        format!("error={error} attempt={attempt}"),
                        trace_id.clone(),
                        true,
                        attempt,
                    )
                    .await;
                if attempt == 3 {
                    return Err(format!("profile relay: {error}"));
                }
            }
        }
        if attempt < 3 {
            sleep(Duration::from_millis(120 * u64::from(attempt))).await;
        }
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
struct AiChatMessageBody {
    owner_user_id: String,
    #[allow(dead_code)]
    conversation_id: String,
    content_text: String,
    #[allow(dead_code)]
    content_type: String,
}

#[derive(Debug, Deserialize)]
struct MemoryResolveRequest {
    memory_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
struct MemoryResolveItem {
    memory_id: String,
    content: String,
    network_type: String,
    keywords: Vec<String>,
    temporal_state: String,
    preference_polarity: String,
    source_message_id: String,
}

#[derive(Debug, Serialize)]
struct MemoryResolveResponse {
    items: Vec<MemoryResolveItem>,
}

async fn forgetting_decide(
    State(state): State<Arc<ContextAppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<ForgettingDecideResponse>, StatusCode> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)?;
    let body: ForgettingDecideRequest =
        serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?;
    if !state.store.is_postgres() {
        return Ok(Json(ForgettingDecideResponse {
            accepted: false,
            persistence: "noop_no_database".into(),
        }));
    }
    let row = ForgettingDecisionInsert {
        user_id: body.user_id,
        target_type: body.target_type,
        target_id: body.target_id,
        decision: body.decision,
        reason_codes: body.reason_codes,
        policy_version: body.policy_version,
        cold_storage_ref: body.cold_storage_ref,
    };
    state
        .store
        .insert_forgetting_decision(&row)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(ForgettingDecideResponse {
        accepted: true,
        persistence: "postgres".into(),
    }))
}

async fn memory_resolve(
    State(state): State<Arc<ContextAppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<MemoryResolveResponse>, StatusCode> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)?;
    let body: MemoryResolveRequest =
        serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?;
    let mut items = vec![];
    for id in body.memory_ids {
        if let Some(a) = state
            .store
            .get_artifact_optional(&id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        {
            items.push(MemoryResolveItem {
                memory_id: a.memory_id.clone(),
                content: a.content.clone(),
                network_type: a.network_type.clone(),
                keywords: a.keywords.clone(),
                temporal_state: a.temporal_state.clone(),
                preference_polarity: a.preference_polarity.clone(),
                source_message_id: a.source_message_id.clone(),
            });
        }
    }
    Ok(Json(MemoryResolveResponse { items }))
}

async fn build_context(
    State(state): State<Arc<ContextAppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<ContextBuildResponse>, StatusCode> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)?;
    let request: ContextBuildRequest =
        serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?;
    let started = Instant::now();
    let (retrieval_used, degraded_from_modes) = state
        .policy
        .filter_retrieval_modes(&request.retrieval_modes);
    let routing = decide_route(TaskType::from(request.task_type.as_str()), &request.input);
    let evidence = evidence::collect_l1_evidence(
        &state.store,
        &state.policy,
        &request.user_id,
        &request.input,
        usize::try_from(request.memory_limit.max(0)).unwrap_or_default(),
        usize::try_from(request.summary_limit.max(0)).unwrap_or_default(),
        &retrieval_used,
        state.policy.graph_enabled,
        state.policy.rerank_enabled,
    )
    .await;
    let degraded = degraded_from_modes
        || retrieval_used.is_empty()
        || (evidence.selected_memory_ids.is_empty() && evidence.selected_summary_ids.is_empty());
    let reply_style = request
        .reply_style
        .clone()
        .unwrap_or_else(|| state.policy.default_reply_style.clone());
    let elapsed_ms = started.elapsed().as_millis();
    let escalation_summary = if routing.escalation_reasons.is_empty() {
        "none".to_string()
    } else {
        routing.escalation_reasons.join(",")
    };
    let upgraded = routing.candidate_route != routing.executed_route;
    let query_pref = l1_policy::query_preference_polarity(&request.input)
        .unwrap_or("neutral")
        .to_string();
    let task_context = format!(
        "route_candidate={}, route_executed={}, upgraded={}, escalation_reasons={}, evidence_count={}, summary_hits={}, artifact_hits={}, entity_hits={}, conflict_count={}, route_confidence={:.2}, estimated_llm_calls={}, estimated_tokens={}, elapsed_ms={}, query_preference_polarity={}, evidence_preference_polarity={}",
        routing.candidate_route,
        routing.executed_route,
        upgraded,
        escalation_summary,
        evidence.evidence_count,
        evidence.summary_hits,
        evidence.artifact_hits,
        evidence.entity_hits,
        evidence.conflict_count,
        evidence.top_confidence,
        if upgraded { 1 } else { 0 },
        evidence.estimated_tokens,
        elapsed_ms,
        query_pref,
        evidence.top_evidence_preference_polarity
    );
    state
        .record_routing(
            &request.user_id,
            RoutingObservation {
                executed_route: routing.executed_route.clone(),
                candidate_route: routing.candidate_route.clone(),
                escalation_reasons: routing.escalation_reasons.clone(),
                upgraded,
                evidence_count: evidence.evidence_count,
                summary_hits: evidence.summary_hits,
                artifact_hits: evidence.artifact_hits,
                entity_hits: evidence.entity_hits,
                conflict_count: evidence.conflict_count,
                route_confidence: evidence.top_confidence,
                estimated_llm_calls: if upgraded { 1 } else { 0 },
                estimated_tokens: evidence.estimated_tokens,
                query_preview: preview(&request.input),
                degraded,
                elapsed_ms,
                query_preference_polarity: query_pref,
                evidence_preference_polarity: evidence.top_evidence_preference_polarity.clone(),
                retrieval_modes: retrieval_used.clone(),
            },
        )
        .await;
    if upgraded {
        state
            .record_failure(
                &request.user_id,
                "context_route_decision",
                "route_escalation_deferred",
                format!(
                    "candidate={} executed={} reasons={}",
                    routing.candidate_route, routing.executed_route, escalation_summary
                ),
                request.trace_id.clone(),
                true,
                1,
            )
            .await;
    }

    if state.store.is_postgres() {
        let log_row = ContextLogInsert {
            user_id: request.user_id.clone(),
            conversation_id: request.conversation_id.clone(),
            selected_summary_ids: evidence.selected_summary_ids.clone(),
            selected_memory_ids: evidence.selected_memory_ids.clone(),
            retrieval_modes: retrieval_used.clone(),
            task_type: request.task_type.clone(),
            max_tokens: request.max_tokens,
            memory_limit: request.memory_limit,
            summary_limit: request.summary_limit,
        };
        if let Err(e) = state.store.insert_context_log(&log_row).await {
            tracing::warn!(error = %e, "context_logs insert failed (best-effort)");
        }
        if !evidence.selected_memory_ids.is_empty() {
            let touched_at = state.now_string();
            if let Err(e) = state
                .store
                .touch_artifacts(&evidence.selected_memory_ids, &touched_at)
                .await
            {
                tracing::warn!(error = %e, "memory_artifacts activation touch failed (best-effort)");
            }
        }
    }

    let summary_texts: Vec<(&str, &str, f64)> = evidence
        .items
        .iter()
        .filter(|item| item.network_type == "summary")
        .map(|item| {
            (
                item.content.as_str(),
                item.preference_polarity.as_str(),
                item.confidence,
            )
        })
        .collect();
    let artifact_texts: Vec<(&str, &str, &str, f64)> = evidence
        .items
        .iter()
        .filter(|item| item.network_type != "summary")
        .map(|item| {
            (
                item.content.as_str(),
                item.network_type.as_str(),
                item.preference_polarity.as_str(),
                item.confidence,
            )
        })
        .collect();
    let assembled = assemble_query_aware_context(
        &summary_texts,
        &artifact_texts,
        &request.input,
        &retrieval_used,
        &state.policy,
        request.max_tokens,
    );

    let memory_context = if retrieval_used.is_empty() {
        format!(
            "degraded: use working memory + recent summaries only; input_preview={}",
            preview(&request.input)
        )
    } else {
        assembled.memory_context
    };
    let compression_enabled = assembled.compression.enabled;
    let compression_mode_label = match assembled.compression.compression_mode {
        CompressionMode::None => "none",
        CompressionMode::Lite => "lite",
        CompressionMode::Aggressive => "aggressive",
    };

    Ok(Json(ContextBuildResponse {
        system_prompt: format!(
            "You are Lumi. task_type={}, reply_style={}",
            request.task_type, reply_style
        ),
        user_context: format!(
            "user_id={}, agent_id={}, conversation_id={}",
            request.user_id, request.agent_id, request.conversation_id
        ),
        memory_context,
        task_context,
        selected_summary_ids: evidence.selected_summary_ids,
        selected_memory_ids: evidence.selected_memory_ids,
        retrieval_used,
        degraded,
        token_budget: TokenBudget {
            max_tokens: request.max_tokens,
            memory_limit: request.memory_limit,
            summary_limit: request.summary_limit,
        },
        compression: CompressionView {
            enabled: compression_enabled,
            mode: compression_mode_label.to_string(),
            quality_score: assembled.compression.quality_score,
            compression_ratio: assembled.compression.compression_ratio,
        },
    }))
}

async fn save_checkpoint(
    State(state): State<Arc<ContextAppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<SessionCheckpointResponse>, StatusCode> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)?;
    let request: SessionCheckpointRequest =
        serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?;
    let dedupe_key = format!(
        "{}|{}|{}|{}|{}|{}|{}",
        request.agent_id,
        request.user_id,
        request.conversation_id.as_deref().unwrap_or(""),
        request.schema_version,
        request.working_summary_ref.as_deref().unwrap_or(""),
        canonical_json_string(&request.runtime_state_blob),
        canonical_json_string(&request.policy_versions)
    );
    if let Some(existing_id) = state
        .store
        .checkpoint_lookup(&dedupe_key)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return Ok(Json(SessionCheckpointResponse {
            accepted: true,
            checkpoint_id: existing_id,
        }));
    }
    let checkpoint_id = state.next_id();
    let rec = RuntimeCheckpointRecord {
        checkpoint_id: checkpoint_id.clone(),
        agent_id: request.agent_id,
        user_id: request.user_id,
        conversation_id: request.conversation_id,
        schema_version: request.schema_version,
        working_summary_ref: request.working_summary_ref,
        runtime_state_blob: request.runtime_state_blob,
        policy_versions: request.policy_versions,
        created_at: state.now_string(),
    };
    let checkpoint_id = state
        .store
        .checkpoint_insert(&dedupe_key, rec)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(SessionCheckpointResponse {
        accepted: true,
        checkpoint_id,
    }))
}

async fn write_memory(
    State(state): State<Arc<ContextAppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<MemoryWriteResponse>, StatusCode> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)?;
    let request: MemoryWriteRequest =
        serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?;
    let raw_text = request.raw_text.unwrap_or_default();
    if raw_text.trim().is_empty() {
        return Ok(Json(MemoryWriteResponse {
            accepted: false,
            write_mode: "noop_missing_raw_text".to_string(),
            trace_id: state.next_id(),
        }));
    }
    let trace_id = request.event_id.clone().unwrap_or_else(|| state.next_id());
    let candidates = heuristic_extract(&raw_text);
    let distilled = distill_message(&raw_text, &candidates);
    let now = state.now_string();
    let policy_version = state.policy.policy_version_label();
    let mut written_ids = vec![];
    for candidate in &candidates {
        let memory_id = state.next_id();
        written_ids.push(memory_id.clone());
        let rec = MemoryArtifactRecord {
            memory_id: memory_id.clone(),
            user_id: request.user_id.clone(),
            conversation_id: request.source_ref_id.clone().unwrap_or_default(),
            source_message_id: request.event_id.clone().unwrap_or_default(),
            content: candidate
                .get("content")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .to_string(),
            network_type: candidate
                .get("network_type")
                .and_then(|value| value.as_str())
                .unwrap_or("experience")
                .to_string(),
            evidence_type: candidate
                .get("evidence_type")
                .and_then(|value| value.as_str())
                .unwrap_or("fact")
                .to_string(),
            memory_level: candidate
                .get("memory_level")
                .and_then(|value| value.as_str())
                .unwrap_or("working")
                .to_string(),
            source_type: request
                .source_type
                .clone()
                .or_else(|| request.source_service.clone())
                .unwrap_or_else(|| "direct_internal".to_string()),
            confidence: request.memory_value_score.unwrap_or_else(|| {
                candidate
                    .get("confidence")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(0.5)
            }),
            importance_score: initial_importance_score(
                &state,
                request
                    .source_type
                    .as_deref()
                    .or(request.source_service.as_deref())
                    .unwrap_or("direct_internal"),
                request.memory_value_score.unwrap_or_else(|| {
                    candidate
                        .get("confidence")
                        .and_then(|value| value.as_f64())
                        .unwrap_or(0.5)
                }),
                distilled.supersedes_previous,
            ),
            keywords: distilled.keywords.clone(),
            temporal_state: distilled.temporal_state.clone(),
            supersedes_previous: distilled.supersedes_previous,
            preference_polarity: distilled.preference_polarity.clone(),
            last_accessed_at: now.clone(),
            access_count: 0,
            created_at: now.clone(),
        };
        state
            .store
            .insert_artifact(rec)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    if !written_ids.is_empty() {
        let summary_id = state.next_id();
        state
            .store
            .insert_summary(MemorySummaryRecord {
                summary_id,
                user_id: request.user_id.clone(),
                conversation_id: request.source_ref_id.clone().unwrap_or_default(),
                summary_type: "direct_internal".to_string(),
                summary_text: distilled.summary_text,
                memory_ids: written_ids,
                source_message_ids: request
                    .event_id
                    .clone()
                    .map(|value| vec![value])
                    .unwrap_or_default(),
                keywords: distilled.keywords,
                temporal_state: distilled.temporal_state,
                supersedes_previous: distilled.supersedes_previous,
                preference_polarity: distilled.preference_polarity,
                updated_at: now,
                policy_version,
            })
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(Json(MemoryWriteResponse {
        accepted: true,
        write_mode: "direct_internal".to_string(),
        trace_id,
    }))
}

async fn search_memory(
    State(state): State<Arc<ContextAppState>>,
    headers: HeaderMap,
    Query(query): Query<MemorySearchQuery>,
) -> Result<Json<MemorySearchResponse>, StatusCode> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)?;
    let (modes, _) = state.policy.filter_retrieval_modes(&[]);
    let evidence = evidence::collect_l1_evidence(
        &state.store,
        &state.policy,
        &query.user_id,
        query.query.as_deref().unwrap_or_default(),
        query.limit.unwrap_or(10) as usize,
        3,
        &modes,
        state.policy.graph_enabled,
        state.policy.rerank_enabled,
    )
    .await;
    let mut items = evidence.items;
    items.truncate(query.limit.unwrap_or(10) as usize);

    Ok(Json(MemorySearchResponse {
        items,
        next_cursor: None,
    }))
}

async fn consolidate_memory(
    State(state): State<Arc<ContextAppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<MemoryConsolidateResponse>, StatusCode> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)?;
    let request: MemoryConsolidateRequest =
        serde_json::from_slice(&body).map_err(|_| StatusCode::BAD_REQUEST)?;
    if state
        .store
        .consolidate_has_event(&request.event_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return Ok(Json(MemoryConsolidateResponse {
            accepted: true,
            replayable: true,
        }));
    }
    let mode = match request.mode.as_deref() {
        Some("replay") => ConsolidationMode::Replay,
        _ => ConsolidationMode::Incremental,
    };
    let selected_ids = request.artifact_ids.unwrap_or_default();
    tracing::info!(
        event_id = %request.event_id,
        user_id = %request.user_id,
        mode = ?mode,
        artifact_count = selected_ids.len(),
        "context-service memory consolidate"
    );
    if !selected_ids.is_empty() {
        let selected = state
            .store
            .get_artifacts_by_ids(&selected_ids)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if selected.is_empty() {
            return Ok(Json(MemoryConsolidateResponse {
                accepted: false,
                replayable: mode.supports_replay(),
            }));
        }
        let summary_id = state.next_id();
        let summary_text = selected
            .iter()
            .map(|artifact| artifact.content.clone())
            .collect::<Vec<_>>()
            .join("；");
        let keywords = selected
            .iter()
            .flat_map(|artifact| artifact.keywords.clone())
            .fold(vec![], |mut acc, keyword| {
                if !acc.iter().any(|existing| existing == &keyword) {
                    acc.push(keyword);
                }
                acc
            });
        let preference_polarity = if selected
            .iter()
            .any(|artifact| artifact.preference_polarity == "negative")
        {
            "negative".to_string()
        } else if selected
            .iter()
            .any(|artifact| artifact.preference_polarity == "positive")
        {
            "positive".to_string()
        } else {
            "neutral".to_string()
        };
        let policy_version = state.policy.policy_version_label();
        state
            .store
            .insert_summary(MemorySummaryRecord {
                summary_id: summary_id.clone(),
                user_id: request.user_id.clone(),
                conversation_id: String::new(),
                summary_type: match mode {
                    ConsolidationMode::Incremental => "incremental".to_string(),
                    ConsolidationMode::Replay => "replay".to_string(),
                },
                summary_text,
                memory_ids: selected_ids.clone(),
                source_message_ids: vec![],
                keywords,
                temporal_state: selected
                    .iter()
                    .find(|artifact| artifact.supersedes_previous)
                    .map(|artifact| artifact.temporal_state.clone())
                    .unwrap_or_else(|| {
                        selected
                            .first()
                            .map(|artifact| artifact.temporal_state.clone())
                            .unwrap_or_else(|| "timeless".to_string())
                    }),
                supersedes_previous: selected.iter().any(|artifact| artifact.supersedes_previous),
                preference_polarity,
                updated_at: state.now_string(),
                policy_version,
            })
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        state
            .store
            .consolidate_record(&request.event_id, &summary_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    } else {
        // Empty selections are treated as a no-op and intentionally do not occupy
        // the event_id idempotency index, so callers may retry later with artifacts.
        return Ok(Json(MemoryConsolidateResponse {
            accepted: false,
            replayable: mode.supports_replay(),
        }));
    }

    Ok(Json(MemoryConsolidateResponse {
        accepted: true,
        replayable: mode.supports_replay(),
    }))
}

async fn get_asmr_lite_observability(
    State(state): State<Arc<ContextAppState>>,
    headers: HeaderMap,
) -> Result<Json<AsmrLiteObservabilityResponse>, StatusCode> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)?;
    let counts = state
        .store
        .observability_counts()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let artifact_count = counts.artifact_count;
    let policy_version_label = state.policy.policy_version_label();
    let summary_count = counts.summary_count;
    let latest_summary_policy_version = counts.latest_summary_policy_version;
    let entity_count = counts.entity_count;
    let link_count = counts.link_count;
    let mut routing = state
        .routing_metrics
        .lock()
        .expect("routing_metrics mutex poisoned")
        .clone();
    let mut failure_cases: Vec<FailureCaseRecord> = state
        .failure_cases
        .lock()
        .expect("failure_cases mutex poisoned")
        .iter()
        .rev()
        .take(10)
        .cloned()
        .collect();
    let mut total_failures = state
        .failure_cases
        .lock()
        .expect("failure_cases mutex poisoned")
        .len();

    if let Ok(Some(snap)) = state.store.fetch_persistent_asmr_snapshot().await {
        if state.store.is_postgres() {
            if snap.last_observation.is_some() {
                routing.last_observation = snap.last_observation;
            }
            failure_cases = snap.recent_failures;
            total_failures = snap.total_failures as usize;
        }
    }

    Ok(Json(AsmrLiteObservabilityResponse {
        artifact_count,
        summary_count,
        entity_count,
        link_count,
        checkpoint_count: counts.checkpoint_count,
        total_failures,
        routing: RoutingMetricsView {
            total_requests: routing.total_requests,
            l1_requests: routing.l1_requests,
            l2_candidates: routing.l2_candidates,
            l3_candidates: routing.l3_candidates,
            degraded_requests: routing.degraded_requests,
            total_conflicts: routing.total_conflicts,
            last_observation: routing
                .last_observation
                .map(|observation| RoutingObservationView {
                    executed_route: observation.executed_route,
                    candidate_route: observation.candidate_route,
                    escalation_reasons: observation.escalation_reasons,
                    upgraded: observation.upgraded,
                    evidence_count: observation.evidence_count,
                    summary_hits: observation.summary_hits,
                    artifact_hits: observation.artifact_hits,
                    entity_hits: observation.entity_hits,
                    conflict_count: observation.conflict_count,
                    route_confidence: observation.route_confidence,
                    estimated_llm_calls: observation.estimated_llm_calls,
                    estimated_tokens: observation.estimated_tokens,
                    query_preview: observation.query_preview,
                    degraded: observation.degraded,
                    elapsed_ms: observation.elapsed_ms,
                    query_preference_polarity: observation.query_preference_polarity,
                    evidence_preference_polarity: observation.evidence_preference_polarity,
                    retrieval_modes: observation.retrieval_modes,
                }),
        },
        policy_version_label,
        latest_summary_policy_version,
        recent_failures: failure_cases
            .into_iter()
            .map(|item| FailureCaseView {
                id: item.id,
                user_id: item.user_id,
                stage: item.stage,
                category: item.category,
                detail: item.detail,
                trace_id: item.trace_id,
                retryable: item.retryable,
                attempt_count: item.attempt_count,
                created_at: item.created_at,
            })
            .collect(),
    }))
}

#[derive(Debug, Deserialize)]
struct ContextBuildRequest {
    user_id: String,
    agent_id: String,
    conversation_id: String,
    input: String,
    task_type: String,
    max_tokens: i32,
    memory_limit: i32,
    summary_limit: i32,
    reply_style: Option<String>,
    trace_id: Option<String>,
    #[serde(default)]
    retrieval_modes: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ContextBuildResponse {
    system_prompt: String,
    user_context: String,
    memory_context: String,
    task_context: String,
    selected_summary_ids: Vec<String>,
    selected_memory_ids: Vec<String>,
    retrieval_used: Vec<String>,
    degraded: bool,
    token_budget: TokenBudget,
    compression: CompressionView,
}

#[derive(Debug, Serialize)]
struct CompressionView {
    enabled: bool,
    mode: String,
    quality_score: f64,
    compression_ratio: f64,
}

#[derive(Debug, Serialize)]
struct TokenBudget {
    max_tokens: i32,
    memory_limit: i32,
    summary_limit: i32,
}

#[derive(Debug, Deserialize)]
struct SessionCheckpointRequest {
    agent_id: String,
    user_id: String,
    conversation_id: Option<String>,
    schema_version: i32,
    working_summary_ref: Option<String>,
    #[serde(default)]
    runtime_state_blob: serde_json::Value,
    #[serde(default)]
    policy_versions: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct SessionCheckpointResponse {
    accepted: bool,
    checkpoint_id: String,
}

#[derive(Debug, Deserialize)]
struct MemoryWriteRequest {
    event_id: Option<String>,
    user_id: String,
    source_type: Option<String>,
    source_service: Option<String>,
    source_ref_id: Option<String>,
    raw_text: Option<String>,
    memory_value_score: Option<f64>,
}

#[derive(Debug, Serialize)]
struct MemoryWriteResponse {
    accepted: bool,
    write_mode: String,
    trace_id: String,
}

#[derive(Debug, Deserialize)]
struct MemorySearchQuery {
    user_id: String,
    query: Option<String>,
    limit: Option<u32>,
}

#[derive(Debug, Serialize)]
struct MemorySearchResponse {
    items: Vec<MemorySearchItem>,
    next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ForgettingDecideRequest {
    user_id: String,
    target_type: String,
    target_id: String,
    decision: String,
    #[serde(default)]
    reason_codes: serde_json::Value,
    policy_version: Option<String>,
    cold_storage_ref: Option<String>,
}

#[derive(Debug, Serialize)]
struct ForgettingDecideResponse {
    accepted: bool,
    persistence: String,
}

#[derive(Debug, Deserialize)]
struct MemoryConsolidateRequest {
    event_id: String,
    user_id: String,
    artifact_ids: Option<Vec<String>>,
    mode: Option<String>,
}

#[derive(Debug, Serialize)]
struct MemoryConsolidateResponse {
    accepted: bool,
    replayable: bool,
}

#[derive(Debug, Serialize)]
struct AsmrLiteObservabilityResponse {
    artifact_count: usize,
    summary_count: usize,
    entity_count: usize,
    link_count: usize,
    checkpoint_count: usize,
    total_failures: usize,
    routing: RoutingMetricsView,
    /// 与 `PolicyConfigStore::policy_version_label` 一致（写入 summary 时使用的单值标签）
    policy_version_label: String,
    /// 按 `updated_at` 取最新一条 summary 的 `policy_version`（无 summary 时为 null）
    latest_summary_policy_version: Option<String>,
    recent_failures: Vec<FailureCaseView>,
}

#[derive(Debug, Serialize)]
struct RoutingMetricsView {
    total_requests: u64,
    l1_requests: u64,
    l2_candidates: u64,
    l3_candidates: u64,
    degraded_requests: u64,
    total_conflicts: u64,
    last_observation: Option<RoutingObservationView>,
}

#[derive(Debug, Serialize)]
struct RoutingObservationView {
    executed_route: String,
    candidate_route: String,
    escalation_reasons: Vec<String>,
    upgraded: bool,
    evidence_count: usize,
    summary_hits: usize,
    artifact_hits: usize,
    entity_hits: usize,
    conflict_count: usize,
    route_confidence: f64,
    estimated_llm_calls: u32,
    estimated_tokens: usize,
    query_preview: String,
    degraded: bool,
    elapsed_ms: u128,
    query_preference_polarity: String,
    evidence_preference_polarity: String,
    retrieval_modes: Vec<String>,
}

#[derive(Debug, Serialize)]
struct FailureCaseView {
    id: String,
    user_id: String,
    stage: String,
    category: String,
    detail: String,
    trace_id: Option<String>,
    retryable: bool,
    attempt_count: u32,
    created_at: String,
}

fn preview(value: &str) -> String {
    value.chars().take(48).collect()
}

fn canonical_json_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(v) => v.to_string(),
        serde_json::Value::Number(v) => v.to_string(),
        serde_json::Value::String(v) => {
            serde_json::to_string(v).unwrap_or_else(|_| "\"\"".to_string())
        }
        serde_json::Value::Array(values) => {
            let items = values
                .iter()
                .map(canonical_json_string)
                .collect::<Vec<_>>()
                .join(",");
            format!("[{items}]")
        }
        serde_json::Value::Object(map) => {
            let mut keys = map.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            let items = keys
                .into_iter()
                .map(|key| {
                    let encoded_key =
                        serde_json::to_string(&key).unwrap_or_else(|_| "\"\"".to_string());
                    let encoded_value = map
                        .get(&key)
                        .map(canonical_json_string)
                        .unwrap_or_else(|| "null".to_string());
                    format!("{encoded_key}:{encoded_value}")
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{items}}}")
        }
    }
}
