//! Internal + dev-only HTTP relay (`Rules/16` envelope).

use std::{cmp::Ordering, sync::Arc, time::Instant};

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use onelink_event_envelope::EventEnvelope;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

use crate::{
    app_state::{ContextAppState, MemoryArtifactRecord},
    l1_policy,
    memory_distiller::{distill_message, ConsolidationMode},
    memory_extractor::heuristic_extract,
    memory_store::{
        MemoryEntityLinkRecord, MemoryEntityRecord, MemorySummaryRecord, RoutingObservation,
    },
    task_router::{decide_route, TaskType},
};

const INTERNAL_TOKEN_HEADER: &str = "x-internal-token";

fn verify_internal_token(headers: &HeaderMap, expected: &str) -> Result<(), StatusCode> {
    if expected.is_empty() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    let ok = headers
        .get(INTERNAL_TOKEN_HEADER)
        .and_then(|v| v.to_str().ok())
        == Some(expected);
    if ok {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

pub fn router(state: Arc<ContextAppState>) -> Router {
    Router::new()
        .route("/internal/context/build", post(build_context))
        .route("/internal/session/checkpoint", post(save_checkpoint))
        .route("/internal/memory/write", post(write_memory))
        .route("/internal/memory/search", get(search_memory))
        .route("/internal/memory/consolidate", post(consolidate_memory))
        .route("/internal/memory/resolve", post(memory_resolve))
        .route(
            "/internal/observability/asmr-lite",
            get(get_asmr_lite_observability),
        )
        .route("/internal/events/receive", post(receive_event))
        .with_state(state)
}

async fn receive_event(
    State(state): State<Arc<ContextAppState>>,
    headers: HeaderMap,
    Json(envelope): Json<EventEnvelope>,
) -> StatusCode {
    if let Err(code) = verify_internal_token(&headers, &state.config.internal_shared_secret) {
        return code;
    }

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
    }

    StatusCode::ACCEPTED
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
        .ok_or_else(|| "missing message_id".to_string())?
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
    let text_resp = state
        .client
        .get(&url)
        .header(
            INTERNAL_TOKEN_HEADER,
            state.config.internal_shared_secret.as_str(),
        )
        .send()
        .await
        .map_err(|e| format!("ai-chat fetch message: {e}"))?;
    if !text_resp.status().is_success() {
        state.record_failure(
            &event_user_id,
            "context_message_fetch",
            "message_fetch_non_success",
            format!("ai-chat message fetch status {}", text_resp.status()),
            trace_id.clone(),
            true,
            1,
        );
        return Err(format!(
            "ai-chat message fetch status {}",
            text_resp.status()
        ));
    }
    let msg: AiChatMessageBody = text_resp
        .json()
        .await
        .map_err(|e| format!("ai-chat message decode: {e}"))?;
    if msg.owner_user_id != event_user_id {
        state.record_failure(
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
        );
        return Err("event user_id does not match ai-chat conversation owner".to_string());
    }

    let candidates = heuristic_extract(&msg.content_text);
    let distilled = distill_message(&msg.content_text, &candidates);
    let mut memory_ids: Vec<String> = vec![];
    let now = state.now_string();
    {
        let mut art = state.artifacts.lock().expect("artifacts mutex poisoned");
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
            art.insert(
                memory_id.clone(),
                MemoryArtifactRecord {
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
                    keywords: distilled.keywords.clone(),
                    temporal_state: distilled.temporal_state.clone(),
                    supersedes_previous: distilled.supersedes_previous,
                    preference_polarity: distilled.preference_polarity.clone(),
                    created_at: now.clone(),
                },
            );
            memory_ids.push(memory_id);
        }
    }
    let summary_id = state.next_id();
    {
        let mut summaries = state.summaries.lock().expect("summaries mutex poisoned");
        summaries.insert(
            summary_id.clone(),
            MemorySummaryRecord {
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
            },
        );
    }
    if !distilled.entities.is_empty() {
        let mut entities = state.entities.lock().expect("entities mutex poisoned");
        let mut links = state
            .entity_links
            .lock()
            .expect("entity_links mutex poisoned");
        for entity in &distilled.entities {
            let entity_id = format!(
                "entity:{}:{}:{}",
                event_user_id,
                entity.entity_type,
                entity.name.to_lowercase()
            );
            entities
                .entry(entity_id.clone())
                .or_insert_with(|| MemoryEntityRecord {
                    id: entity_id.clone(),
                    user_id: event_user_id.clone(),
                    entity_type: entity.entity_type.clone(),
                    name: entity.name.clone(),
                });
            for memory_id in &memory_ids {
                let link_id = format!("{entity_id}:{memory_id}");
                links
                    .entry(link_id.clone())
                    .or_insert_with(|| MemoryEntityLinkRecord {
                        id: link_id,
                        user_id: event_user_id.clone(),
                        entity_id: entity_id.clone(),
                        memory_id: memory_id.clone(),
                        relationship: entity.relationship.clone(),
                        confidence: entity.confidence,
                    });
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
            "token_count": msg.content_text.chars().count(),
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
                state.record_failure(
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
                );
                if attempt == 3 {
                    return Err(format!("profile relay status {}", relay_response.status()));
                }
            }
            Err(error) => {
                state.record_failure(
                    &event_user_id,
                    "context_profile_projection",
                    "profile_projection_request_failed",
                    format!("error={error} attempt={attempt}"),
                    trace_id.clone(),
                    true,
                    attempt,
                );
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
}

#[derive(Debug, Serialize)]
struct MemoryResolveResponse {
    items: Vec<MemoryResolveItem>,
}

async fn memory_resolve(
    State(state): State<Arc<ContextAppState>>,
    headers: HeaderMap,
    Json(body): Json<MemoryResolveRequest>,
) -> Result<Json<MemoryResolveResponse>, StatusCode> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)?;
    let g = state.artifacts.lock().expect("artifacts mutex poisoned");
    let mut items = vec![];
    for id in body.memory_ids {
        if let Some(a) = g.get(&id) {
            items.push(MemoryResolveItem {
                memory_id: a.memory_id.clone(),
                content: a.content.clone(),
                network_type: a.network_type.clone(),
            });
        }
    }
    Ok(Json(MemoryResolveResponse { items }))
}

async fn build_context(
    State(state): State<Arc<ContextAppState>>,
    headers: HeaderMap,
    Json(request): Json<ContextBuildRequest>,
) -> Result<Json<ContextBuildResponse>, StatusCode> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)?;
    let started = Instant::now();
    let (retrieval_used, degraded_from_modes) = state
        .policy
        .filter_retrieval_modes(&request.retrieval_modes);
    let routing = decide_route(TaskType::from(request.task_type.as_str()), &request.input);
    let evidence = collect_l1_evidence(
        &state,
        &request.user_id,
        &request.input,
        usize::try_from(request.memory_limit.max(0)).unwrap_or_default(),
        usize::try_from(request.summary_limit.max(0)).unwrap_or_default(),
    );
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
    state.record_routing(RoutingObservation {
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
    });
    if upgraded {
        state.record_failure(
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
        );
    }

    Ok(Json(ContextBuildResponse {
        system_prompt: format!(
            "You are Lumi. task_type={}, reply_style={}",
            request.task_type, reply_style
        ),
        user_context: format!(
            "user_id={}, agent_id={}, conversation_id={}",
            request.user_id, request.agent_id, request.conversation_id
        ),
        memory_context: if retrieval_used.is_empty() {
            format!(
                "degraded: use working memory + recent summaries only; input_preview={}",
                preview(&request.input)
            )
        } else if evidence.memory_context.is_empty() {
            format!(
                "retrieval_modes={}; no structured evidence matched; input_preview={}",
                retrieval_used.join(","),
                preview(&request.input)
            )
        } else {
            format!(
                "retrieval_modes={}; input_preview={}; {}",
                retrieval_used.join(","),
                preview(&request.input),
                evidence.memory_context
            )
        },
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
    }))
}

async fn save_checkpoint(
    State(state): State<Arc<ContextAppState>>,
    headers: HeaderMap,
    Json(request): Json<SessionCheckpointRequest>,
) -> Result<Json<SessionCheckpointResponse>, StatusCode> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)?;
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
        .checkpoint_request_index
        .lock()
        .expect("checkpoint_request_index mutex poisoned")
        .get(&dedupe_key)
        .cloned()
    {
        return Ok(Json(SessionCheckpointResponse {
            accepted: true,
            checkpoint_id: existing_id,
        }));
    }
    let checkpoint_id = state.next_id();
    state
        .checkpoints
        .lock()
        .expect("checkpoints mutex poisoned")
        .insert(
            checkpoint_id.clone(),
            crate::memory_store::RuntimeCheckpointRecord {
                checkpoint_id: checkpoint_id.clone(),
                agent_id: request.agent_id,
                user_id: request.user_id,
                conversation_id: request.conversation_id,
                schema_version: request.schema_version,
                working_summary_ref: request.working_summary_ref,
                runtime_state_blob: request.runtime_state_blob,
                policy_versions: request.policy_versions,
                created_at: state.now_string(),
            },
        );
    state
        .checkpoint_request_index
        .lock()
        .expect("checkpoint_request_index mutex poisoned")
        .insert(dedupe_key, checkpoint_id.clone());

    Ok(Json(SessionCheckpointResponse {
        accepted: true,
        checkpoint_id,
    }))
}

async fn write_memory(
    State(state): State<Arc<ContextAppState>>,
    headers: HeaderMap,
    Json(request): Json<MemoryWriteRequest>,
) -> Result<Json<MemoryWriteResponse>, StatusCode> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)?;
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
    let mut written_ids = vec![];
    {
        let mut artifacts = state.artifacts.lock().expect("artifacts mutex poisoned");
        for candidate in &candidates {
            let memory_id = state.next_id();
            written_ids.push(memory_id.clone());
            artifacts.insert(
                memory_id.clone(),
                MemoryArtifactRecord {
                    memory_id,
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
                    keywords: distilled.keywords.clone(),
                    temporal_state: distilled.temporal_state.clone(),
                    supersedes_previous: distilled.supersedes_previous,
                    preference_polarity: distilled.preference_polarity.clone(),
                    created_at: now.clone(),
                },
            );
        }
    }
    if !written_ids.is_empty() {
        let summary_id = state.next_id();
        state
            .summaries
            .lock()
            .expect("summaries mutex poisoned")
            .insert(
                summary_id.clone(),
                MemorySummaryRecord {
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
                },
            );
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
    let evidence = collect_l1_evidence(
        &state,
        &query.user_id,
        query.query.as_deref().unwrap_or_default(),
        query.limit.unwrap_or(10) as usize,
        3,
    );
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
    Json(request): Json<MemoryConsolidateRequest>,
) -> Result<Json<MemoryConsolidateResponse>, StatusCode> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)?;
    if state
        .consolidate_event_index
        .lock()
        .expect("consolidate_event_index mutex poisoned")
        .contains_key(&request.event_id)
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
        let artifacts = state.artifacts.lock().expect("artifacts mutex poisoned");
        let selected: Vec<_> = selected_ids
            .iter()
            .filter_map(|memory_id| artifacts.get(memory_id).cloned())
            .collect();
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
        drop(artifacts);
        state
            .summaries
            .lock()
            .expect("summaries mutex poisoned")
            .insert(
                summary_id.clone(),
                MemorySummaryRecord {
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
                    supersedes_previous: selected
                        .iter()
                        .any(|artifact| artifact.supersedes_previous),
                    preference_polarity,
                    updated_at: state.now_string(),
                },
            );
        state
            .consolidate_event_index
            .lock()
            .expect("consolidate_event_index mutex poisoned")
            .insert(request.event_id.clone(), summary_id);
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
    let artifact_count = state
        .artifacts
        .lock()
        .expect("artifacts mutex poisoned")
        .len();
    let summary_count = state
        .summaries
        .lock()
        .expect("summaries mutex poisoned")
        .len();
    let entity_count = state
        .entities
        .lock()
        .expect("entities mutex poisoned")
        .len();
    let link_count = state
        .entity_links
        .lock()
        .expect("entity_links mutex poisoned")
        .len();
    let routing = state
        .routing_metrics
        .lock()
        .expect("routing_metrics mutex poisoned")
        .clone();
    let failure_cases = state
        .failure_cases
        .lock()
        .expect("failure_cases mutex poisoned")
        .iter()
        .rev()
        .take(10)
        .cloned()
        .collect::<Vec<_>>();

    Ok(Json(AsmrLiteObservabilityResponse {
        artifact_count,
        summary_count,
        entity_count,
        link_count,
        checkpoint_count: state
            .checkpoints
            .lock()
            .expect("checkpoints mutex poisoned")
            .len(),
        total_failures: state
            .failure_cases
            .lock()
            .expect("failure_cases mutex poisoned")
            .len(),
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
                }),
        },
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

#[derive(Debug, Serialize)]
struct MemorySearchItem {
    memory_id: String,
    network_type: String,
    memory_level: String,
    content: String,
    confidence: f64,
    /// 证据上的偏好极性，便于下游与 benchmark 对齐
    preference_polarity: String,
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

#[derive(Debug, Default)]
struct L1Evidence {
    selected_summary_ids: Vec<String>,
    selected_memory_ids: Vec<String>,
    evidence_count: usize,
    summary_hits: usize,
    artifact_hits: usize,
    entity_hits: usize,
    conflict_count: usize,
    top_confidence: f64,
    estimated_tokens: usize,
    memory_context: String,
    items: Vec<MemorySearchItem>,
    /// 与 `top_confidence` 对应证据的 `preference_polarity`
    top_evidence_preference_polarity: String,
}

#[derive(Debug, Default)]
struct QueryIntent {
    wants_current: bool,
    wants_past: bool,
    wants_update: bool,
    wants_location: bool,
    wants_preference: bool,
    wants_connection: bool,
    wants_remote: bool,
}

fn collect_l1_evidence(
    state: &ContextAppState,
    user_id: &str,
    query: &str,
    memory_limit: usize,
    summary_limit: usize,
) -> L1Evidence {
    let normalized_query = query.trim().to_lowercase();
    let intent = parse_query_intent(query);
    let query_polarity_opt = l1_policy::query_preference_polarity(query);
    let summaries = state.summaries.lock().expect("summaries mutex poisoned");
    let artifacts = state.artifacts.lock().expect("artifacts mutex poisoned");
    let entities = state.entities.lock().expect("entities mutex poisoned");
    let entity_links = state
        .entity_links
        .lock()
        .expect("entity_links mutex poisoned");
    let mut matched_entity_ids: Vec<String> = entities
        .values()
        .filter(|entity| {
            entity.user_id == user_id
                && (normalized_query.contains(&entity.name.to_lowercase())
                    || entity.name.to_lowercase().contains(&normalized_query))
        })
        .map(|entity| entity.id.clone())
        .collect();

    // 「现在在哪座城市」类查询未点名城市时：把用户已有 location 实体纳入匹配，使 entity link 能抬高相关 evidence。
    if intent.wants_location && !l1_policy::query_names_any_city(query) {
        for entity in entities.values() {
            if entity.user_id == user_id && entity.entity_type == "location" {
                if !matched_entity_ids.iter().any(|id| id == &entity.id) {
                    matched_entity_ids.push(entity.id.clone());
                }
            }
        }
    }

    let mut ranked_summaries: Vec<(f64, MemorySummaryRecord)> = summaries
        .values()
        .filter(|summary| summary.user_id == user_id)
        .cloned()
        .map(|summary| {
            let summary_entity_hit = summary.memory_ids.iter().any(|mid| {
                entity_links.values().any(|link| {
                    link.user_id == user_id
                        && link.memory_id == *mid
                        && matched_entity_ids.iter().any(|eid| eid == &link.entity_id)
                })
            });
            (
                score_match(
                    &normalized_query,
                    &summary.summary_text,
                    &summary.keywords,
                    &summary.temporal_state,
                    summary.supersedes_previous,
                    "summary",
                    summary_entity_hit,
                    &intent,
                    &summary.preference_polarity,
                    query_polarity_opt,
                ),
                summary,
            )
        })
        .collect();
    ranked_summaries.sort_by(|left, right| {
        right
            .0
            .partial_cmp(&left.0)
            .unwrap_or(Ordering::Equal)
            .then_with(|| right.1.updated_at.cmp(&left.1.updated_at))
    });

    let mut ranked_artifacts: Vec<(f64, MemoryArtifactRecord)> = artifacts
        .values()
        .filter(|artifact| artifact.user_id == user_id)
        .cloned()
        .map(|artifact| {
            let linked_entity_hit = entity_links.values().any(|link| {
                link.user_id == user_id
                    && link.memory_id == artifact.memory_id
                    && matched_entity_ids
                        .iter()
                        .any(|entity_id| entity_id == &link.entity_id)
            });
            (
                score_match(
                    &normalized_query,
                    &artifact.content,
                    &artifact.keywords,
                    &artifact.temporal_state,
                    artifact.supersedes_previous,
                    &artifact.network_type,
                    linked_entity_hit,
                    &intent,
                    &artifact.preference_polarity,
                    query_polarity_opt,
                ) + artifact.confidence / l1_policy::SCORE_CONFIDENCE_DIVISOR,
                artifact,
            )
        })
        .collect();
    ranked_artifacts.sort_by(|left, right| {
        right
            .0
            .partial_cmp(&left.0)
            .unwrap_or(Ordering::Equal)
            .then_with(|| right.1.created_at.cmp(&left.1.created_at))
    });

    let selected_summaries: Vec<MemorySummaryRecord> = ranked_summaries
        .into_iter()
        .take(if summary_limit == 0 { 0 } else { summary_limit })
        .map(|(_, summary)| summary)
        .collect();
    let selected_artifacts: Vec<MemoryArtifactRecord> = ranked_artifacts
        .into_iter()
        .take(if memory_limit == 0 { 0 } else { memory_limit })
        .map(|(_, artifact)| artifact)
        .collect();

    let mut items = vec![];
    for summary in &selected_summaries {
        let summary_entity_hit = summary.memory_ids.iter().any(|mid| {
            entity_links.values().any(|link| {
                link.user_id == user_id
                    && link.memory_id == *mid
                    && matched_entity_ids.iter().any(|eid| eid == &link.entity_id)
            })
        });
        let confidence = score_match(
            &normalized_query,
            &summary.summary_text,
            &summary.keywords,
            &summary.temporal_state,
            summary.supersedes_previous,
            "summary",
            summary_entity_hit,
            &intent,
            &summary.preference_polarity,
            query_polarity_opt,
        );
        items.push(MemorySearchItem {
            memory_id: summary.summary_id.clone(),
            network_type: "summary".to_string(),
            memory_level: "working".to_string(),
            content: summary.summary_text.clone(),
            confidence,
            preference_polarity: normalized_stored_polarity(&summary.preference_polarity),
        });
    }
    for artifact in &selected_artifacts {
        let linked_entity_hit = entity_links.values().any(|link| {
            link.user_id == user_id
                && link.memory_id == artifact.memory_id
                && matched_entity_ids
                    .iter()
                    .any(|entity_id| entity_id == &link.entity_id)
        });
        let confidence = score_match(
            &normalized_query,
            &artifact.content,
            &artifact.keywords,
            &artifact.temporal_state,
            artifact.supersedes_previous,
            &artifact.network_type,
            linked_entity_hit,
            &intent,
            &artifact.preference_polarity,
            query_polarity_opt,
        );
        items.push(MemorySearchItem {
            memory_id: artifact.memory_id.clone(),
            network_type: artifact.network_type.clone(),
            memory_level: artifact.memory_level.clone(),
            content: artifact.content.clone(),
            confidence,
            preference_polarity: normalized_stored_polarity(&artifact.preference_polarity),
        });
    }

    let conflict_count = selected_summaries
        .iter()
        .map(|summary| l1_policy::detect_conflict_count(&summary.summary_text))
        .sum::<usize>()
        + selected_artifacts
            .iter()
            .map(|artifact| l1_policy::detect_conflict_count(&artifact.content))
            .sum::<usize>();
    let summary_preview = selected_summaries
        .iter()
        .take(2)
        .map(|summary| summary.summary_text.clone())
        .collect::<Vec<_>>()
        .join(" | ");
    let artifact_preview = selected_artifacts
        .iter()
        .take(3)
        .map(|artifact| format!("[{}] {}", artifact.network_type, artifact.content))
        .collect::<Vec<_>>()
        .join(" | ");
    let top_confidence = items
        .iter()
        .map(|item| item.confidence)
        .fold(0.0_f64, f64::max);
    let top_evidence_preference_polarity = top_polarity_for_selection(
        &normalized_query,
        query_polarity_opt,
        &intent,
        user_id,
        &selected_summaries,
        &selected_artifacts,
        &matched_entity_ids,
        &entity_links,
    );
    let query_polarity_hint = query_polarity_opt.unwrap_or("none");
    let memory_context = format!(
        "summary_hits={}; artifact_hits={}; entity_hits={}; top_confidence={:.2}; query_polarity_hint={}; pref_top={}; summaries={}; artifacts={}",
        selected_summaries.len(),
        selected_artifacts.len(),
        matched_entity_ids.len(),
        top_confidence,
        query_polarity_hint,
        top_evidence_preference_polarity,
        summary_preview,
        artifact_preview
    );
    let estimated_tokens = estimate_token_count(
        selected_summaries
            .iter()
            .map(|summary| summary.summary_text.as_str())
            .chain(
                selected_artifacts
                    .iter()
                    .map(|artifact| artifact.content.as_str()),
            ),
    );

    L1Evidence {
        selected_summary_ids: selected_summaries
            .iter()
            .map(|summary| summary.summary_id.clone())
            .collect(),
        selected_memory_ids: selected_artifacts
            .iter()
            .map(|artifact| artifact.memory_id.clone())
            .collect(),
        evidence_count: selected_summaries.len() + selected_artifacts.len(),
        summary_hits: selected_summaries.len(),
        artifact_hits: selected_artifacts.len(),
        entity_hits: matched_entity_ids.len(),
        conflict_count,
        top_confidence,
        estimated_tokens,
        memory_context,
        items,
        top_evidence_preference_polarity,
    }
}

#[inline]
fn normalized_stored_polarity(raw: &str) -> String {
    if raw.is_empty() {
        "neutral".to_string()
    } else {
        raw.to_string()
    }
}

fn top_polarity_for_selection(
    normalized_query: &str,
    query_polarity_opt: Option<&str>,
    intent: &QueryIntent,
    user_id: &str,
    selected_summaries: &[MemorySummaryRecord],
    selected_artifacts: &[MemoryArtifactRecord],
    matched_entity_ids: &[String],
    entity_links: &std::collections::HashMap<String, MemoryEntityLinkRecord>,
) -> String {
    let mut best_score = -1.0_f64;
    let mut best_pol = "neutral".to_string();
    for summary in selected_summaries {
        let summary_entity_hit = summary.memory_ids.iter().any(|mid| {
            entity_links.values().any(|link| {
                link.user_id == user_id
                    && link.memory_id == *mid
                    && matched_entity_ids.iter().any(|eid| eid == &link.entity_id)
            })
        });
        let s = score_match(
            normalized_query,
            &summary.summary_text,
            &summary.keywords,
            &summary.temporal_state,
            summary.supersedes_previous,
            "summary",
            summary_entity_hit,
            intent,
            &summary.preference_polarity,
            query_polarity_opt,
        );
        if s > best_score {
            best_score = s;
            best_pol = normalized_stored_polarity(&summary.preference_polarity);
        }
    }
    for artifact in selected_artifacts {
        let linked_entity_hit = entity_links.values().any(|link| {
            link.user_id == user_id
                && link.memory_id == artifact.memory_id
                && matched_entity_ids
                    .iter()
                    .any(|entity_id| entity_id == &link.entity_id)
        });
        let s = score_match(
            normalized_query,
            &artifact.content,
            &artifact.keywords,
            &artifact.temporal_state,
            artifact.supersedes_previous,
            &artifact.network_type,
            linked_entity_hit,
            intent,
            &artifact.preference_polarity,
            query_polarity_opt,
        ) + artifact.confidence / l1_policy::SCORE_CONFIDENCE_DIVISOR;
        if s > best_score {
            best_score = s;
            best_pol = normalized_stored_polarity(&artifact.preference_polarity);
        }
    }
    best_pol
}

fn score_match(
    query: &str,
    content: &str,
    keywords: &[String],
    temporal_state: &str,
    supersedes_previous: bool,
    network_type: &str,
    linked_entity_hit: bool,
    intent: &QueryIntent,
    preference_polarity: &str,
    query_polarity_opt: Option<&str>,
) -> f64 {
    if query.is_empty() {
        return l1_policy::SCORE_EMPTY_QUERY;
    }
    let content_lower = content.to_lowercase();
    let mut score: f64 = if content_lower.contains(query) {
        l1_policy::SCORE_CONTENT_CONTAINS_QUERY
    } else {
        l1_policy::SCORE_CONTENT_BASE
    };
    if token_overlap_count(query, &content_lower) > 0 {
        score += l1_policy::SCORE_TOKEN_OVERLAP;
    }
    for keyword in keywords {
        if query.contains(&keyword.to_lowercase())
            || content_lower.contains(&keyword.to_lowercase())
        {
            score += l1_policy::SCORE_KEYWORD_HIT;
        }
    }
    if intent.wants_current && matches!(temporal_state, "current" | "updated") {
        score += l1_policy::SCORE_TEMPORAL_CURRENT_MATCH;
    } else if intent.wants_current && temporal_state == "past" {
        score += l1_policy::SCORE_TEMPORAL_CURRENT_PAST_PENALTY;
    }
    if intent.wants_past && temporal_state == "past" {
        score += l1_policy::SCORE_TEMPORAL_PAST_MATCH;
    } else if intent.wants_past && matches!(temporal_state, "current" | "updated") {
        score += l1_policy::SCORE_TEMPORAL_PAST_CURRENT_PENALTY;
    }
    if intent.wants_update && supersedes_previous {
        score += l1_policy::SCORE_UPDATE_SUPERSEDES;
    }
    if intent.wants_location && l1_policy::contains_known_city(&content_lower) {
        score += l1_policy::SCORE_LOCATION_CITY;
    }
    if intent.wants_preference && network_type == "opinion" {
        score += l1_policy::SCORE_PREFERENCE_OPINION;
    }
    if intent.wants_connection && (content.contains("投资人") || content.contains("合伙人")) {
        score += l1_policy::SCORE_CONNECTION;
    }
    if intent.wants_remote && content.contains("远程") {
        score += l1_policy::SCORE_REMOTE;
    }
    if linked_entity_hit {
        score += l1_policy::SCORE_ENTITY_LINK;
    }
    if supersedes_previous && !intent.wants_past {
        score += l1_policy::SCORE_SUPERSEDES_GENERAL;
    }

    let stored = if preference_polarity.is_empty() {
        "neutral"
    } else {
        preference_polarity
    };
    if let Some(qp) = query_polarity_opt {
        if qp != "neutral" && stored == qp {
            score += l1_policy::SCORE_PREFERENCE_ALIGN;
        } else if qp != "neutral" && stored != "neutral" && stored != qp {
            score += l1_policy::SCORE_PREFERENCE_MISMATCH;
        }
    }

    score.min(l1_policy::SCORE_MAX)
}

fn parse_query_intent(query: &str) -> QueryIntent {
    QueryIntent {
        wants_current: l1_policy::INTENT_WANTS_CURRENT
            .iter()
            .any(|marker| query.contains(*marker)),
        wants_past: l1_policy::INTENT_WANTS_PAST
            .iter()
            .any(|marker| query.contains(*marker)),
        wants_update: l1_policy::INTENT_WANTS_UPDATE
            .iter()
            .any(|marker| query.contains(*marker)),
        wants_location: l1_policy::INTENT_WANTS_LOCATION
            .iter()
            .any(|marker| query.contains(*marker)),
        wants_preference: l1_policy::INTENT_WANTS_PREFERENCE
            .iter()
            .any(|marker| query.contains(*marker)),
        wants_connection: l1_policy::INTENT_WANTS_CONNECTION
            .iter()
            .any(|marker| query.contains(*marker)),
        wants_remote: l1_policy::INTENT_WANTS_REMOTE
            .iter()
            .any(|marker| query.contains(*marker)),
    }
}

fn token_overlap_count(query: &str, content: &str) -> usize {
    query_tokens(query)
        .into_iter()
        .filter(|token| content.contains(token))
        .count()
}

fn query_tokens(query: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    for token in query
        .split(|ch: char| !ch.is_alphanumeric() && !matches!(ch, '_' | '-'))
        .filter(|token| token.len() >= 2)
    {
        if !tokens.iter().any(|existing| existing == token) {
            tokens.push(token.to_string());
        }
    }
    for marker in l1_policy::QUERY_TOKEN_EXTRA {
        if query.contains(*marker) && !tokens.iter().any(|existing| existing == *marker) {
            tokens.push((*marker).to_string());
        }
    }
    tokens
}

fn estimate_token_count<'a>(values: impl Iterator<Item = &'a str>) -> usize {
    let total_chars = values.map(|value| value.chars().count()).sum::<usize>();
    (total_chars / 4).max(1)
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
