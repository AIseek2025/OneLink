//! ai-chat-service — vertical slice + dev-only event relay.

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    extract::{Path, State},
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use onelink_event_envelope::EventEnvelope;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::time::{sleep, Duration};

use crate::config::Config;

const DEFAULT_AGENT_ID: &str = "00000000-0000-0000-0000-0000000000a1";
const DEFAULT_TIMESTAMP: &str = "2026-03-20T00:00:00Z";
/// Lowercase header name (HTTP compares case-insensitively).
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

fn assert_conversation_owner(
    state: &AppState,
    conversation_id: &str,
    user_id: &str,
) -> Result<(), (StatusCode, String)> {
    let owners = state
        .conversation_owner
        .lock()
        .expect("owner mutex poisoned");
    let owner = owners
        .get(conversation_id)
        .ok_or((StatusCode::NOT_FOUND, "conversation not found".to_string()))?;
    if owner != user_id {
        return Err((StatusCode::FORBIDDEN, "not your conversation".to_string()));
    }
    Ok(())
}

pub fn router(config: Config) -> Router {
    let state = Arc::new(AppState::new(config));

    Router::new()
        .route(
            "/api/v1/chat/conversations",
            post(create_or_get_conversation),
        )
        .route(
            "/api/v1/chat/conversations/:conversationId/messages",
            post(send_message).get(list_messages),
        )
        .route(
            "/api/v1/chat/conversations/:conversationId/context",
            get(get_context),
        )
        .route(
            "/internal/chat/conversations/:conversationId/messages/:messageId",
            get(get_internal_message),
        )
        .route(
            "/internal/observability/chat-relay",
            get(get_chat_relay_observability),
        )
        .with_state(state)
}

async fn resolve_user_id(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<String, (StatusCode, String)> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "missing Authorization".to_string(),
        ))?;
    let token = auth
        .strip_prefix("Bearer ")
        .or_else(|| auth.strip_prefix("bearer "))
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "invalid Authorization scheme".to_string(),
        ))?
        .trim();
    if token.is_empty() {
        return Err((StatusCode::UNAUTHORIZED, "empty token".to_string()));
    }

    let url = format!(
        "{}/api/v1/identity/me",
        state.config.identity_service_base_url
    );
    let response = state
        .client
        .get(url)
        .header(AUTHORIZATION, auth)
        .send()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("identity-service: {e}")))?;
    if response.status().is_server_error() {
        return Err((
            StatusCode::BAD_GATEWAY,
            format!("identity-service status {}", response.status()),
        ));
    }
    if !response.status().is_success() {
        return Err((
            StatusCode::UNAUTHORIZED,
            "invalid or expired token".to_string(),
        ));
    }
    let me: IdentityMeResponse = response
        .json()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, format!("identity decode: {e}")))?;
    Ok(me.user_id)
}

#[derive(Debug, Deserialize)]
struct IdentityMeResponse {
    user_id: String,
}

async fn create_or_get_conversation(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateConversationRequest>,
) -> Result<Json<CreateConversationResponse>, (StatusCode, String)> {
    // MVP accepts the field for forward compatibility but does not dedupe on it yet.
    let _ = request.idempotency_key;
    let user_id = resolve_user_id(&state, &headers).await?;

    let existing = {
        let map = state
            .user_primary_conversation
            .lock()
            .expect("user_primary mutex poisoned");
        map.get(&user_id).cloned()
    };

    if let Some(cid) = existing {
        let conversations = state
            .conversations
            .lock()
            .expect("conversations mutex poisoned");
        if let Some(conversation) = conversations.get(&cid) {
            return Ok(Json(CreateConversationResponse {
                conversation_id: conversation.conversation_id.clone(),
                status: conversation.status.clone(),
                created_at: conversation.created_at.clone(),
            }));
        }
    }

    let conversation_id = {
        let conversation_id = state.next_uuid_like();
        let conversation = ConversationRecord {
            conversation_id: conversation_id.clone(),
            status: "active".to_string(),
            created_at: DEFAULT_TIMESTAMP.to_string(),
            last_message_at: None,
            context_version: 0,
        };
        let mut conversations = state
            .conversations
            .lock()
            .expect("conversations mutex poisoned");
        conversations.insert(conversation_id.clone(), conversation);
        let mut owners = state
            .conversation_owner
            .lock()
            .expect("owner mutex poisoned");
        owners.insert(conversation_id.clone(), user_id.clone());
        let mut prim = state
            .user_primary_conversation
            .lock()
            .expect("user_primary mutex poisoned");
        prim.insert(user_id, conversation_id.clone());
        conversation_id
    };

    Ok(Json(CreateConversationResponse {
        conversation_id,
        status: "active".to_string(),
        created_at: DEFAULT_TIMESTAMP.to_string(),
    }))
}

async fn send_message(
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<SendMessageRequest>,
) -> Result<Json<SendMessageResponse>, (StatusCode, String)> {
    // MVP keeps the request field visible but does not execute request dedupe on it yet.
    let _ = request.idempotency_key.as_deref();
    let user_id = resolve_user_id(&state, &headers).await?;
    {
        let owners = state
            .conversation_owner
            .lock()
            .expect("owner mutex poisoned");
        let owner = owners
            .get(&conversation_id)
            .ok_or((StatusCode::NOT_FOUND, "conversation not found".to_string()))?;
        if owner != &user_id {
            return Err((StatusCode::FORBIDDEN, "not your conversation".to_string()));
        }
    }

    state.ensure_conversation_exists(&conversation_id, &user_id);

    let user_message_id = state.next_uuid_like();
    let ai_message_id = state.next_uuid_like();
    let trace_id = state.next_uuid_like();

    let context_response = state
        .build_context(&user_id, &conversation_id, &request.content_text, &trace_id)
        .await
        .unwrap_or_else(|error| ContextBuildResponse {
            system_prompt: "fallback system prompt".to_string(),
            user_context: format!("fallback user context; error={error}"),
            memory_context: "fallback memory context".to_string(),
            task_context: "fallback task context".to_string(),
            selected_summary_ids: vec![],
            selected_memory_ids: vec![],
            retrieval_used: vec!["structured".to_string()],
            degraded: true,
            token_budget: TokenBudget {
                max_tokens: 8000,
                memory_limit: 20,
                summary_limit: 3,
            },
        });

    let ai_content_text = state
        .invoke_model_gateway(&request.content_text, &context_response, &trace_id)
        .await
        .unwrap_or_else(|error| format!("Lumi fallback reply (skeleton): {error}"));

    {
        let mut messages = state.messages.lock().expect("messages mutex poisoned");
        let entry = messages.entry(conversation_id.clone()).or_default();
        entry.push(MessageItem {
            message_id: user_message_id.clone(),
            sender_type: "user".to_string(),
            content_text: request.content_text.clone(),
            content_type: request.content_type.clone(),
            created_at: DEFAULT_TIMESTAMP.to_string(),
        });
        entry.push(MessageItem {
            message_id: ai_message_id.clone(),
            sender_type: "assistant".to_string(),
            content_text: ai_content_text.clone(),
            content_type: "text".to_string(),
            created_at: DEFAULT_TIMESTAMP.to_string(),
        });
    }

    {
        let mut conversations = state
            .conversations
            .lock()
            .expect("conversations mutex poisoned");
        if let Some(conversation) = conversations.get_mut(&conversation_id) {
            conversation.last_message_at = Some(DEFAULT_TIMESTAMP.to_string());
            conversation.context_version += 1;
        }
    }

    {
        let mut contexts = state.contexts.lock().expect("contexts mutex poisoned");
        contexts.insert(
            conversation_id.clone(),
            ContextView {
                context_version: state.current_context_version(&conversation_id),
                summary: format!(
                    "retrieval_used={}, degraded={}, memory_context={}",
                    context_response.retrieval_used.join(","),
                    context_response.degraded,
                    context_response.memory_context
                ),
                last_message_at: Some(DEFAULT_TIMESTAMP.to_string()),
            },
        );
    }

    let relay = EventEnvelope::new_v1(
        "chat.user_message.created.v1",
        "ai-chat-service",
        Some(user_id.clone()),
        Some(trace_id.clone()),
        json!({
            "conversation_id": conversation_id.clone(),
            "message_id": user_message_id.clone(),
            "user_id": user_id,
            "content_type": request.content_type,
        }),
    );
    let url = format!(
        "{}/internal/events/receive",
        state.config.context_service_base_url
    );
    let client = state.client.clone();
    let internal_secret = state.config.internal_shared_secret.clone();
    let relay_state = Arc::clone(&state);
    let checkpoint_state = Arc::clone(&state);
    let checkpoint_secret = state.config.internal_shared_secret.clone();
    let relay_trace_id = trace_id.clone();
    tracing::info!(
        envelope = %serde_json::to_string(&relay).unwrap_or_default(),
        "relay chat.user_message.created.v1 -> context-service (async)"
    );
    tokio::spawn(async move {
        for attempt in 1..=3_u32 {
            match client
                .post(url.clone())
                .header(INTERNAL_TOKEN_HEADER, internal_secret.as_str())
                .json(&relay)
                .send()
                .await
            {
                Ok(r) if r.status().is_success() => return,
                Ok(r) => {
                    relay_state.record_relay_failure(
                        "context_event_relay_non_success",
                        format!("status={} attempt={attempt}", r.status()),
                        Some(relay_trace_id.clone()),
                        attempt,
                        true,
                    );
                    tracing::warn!(status = %r.status(), attempt, "context relay non-success");
                }
                Err(e) => {
                    relay_state.record_relay_failure(
                        "context_event_relay_request_failed",
                        format!("error={e} attempt={attempt}"),
                        Some(relay_trace_id.clone()),
                        attempt,
                        true,
                    );
                    tracing::warn!(error = %e, attempt, "context relay failed");
                }
            }
            if attempt < 3 {
                sleep(Duration::from_millis(120 * u64::from(attempt))).await;
            }
        }
    });
    let checkpoint_trace_id = trace_id.clone();
    let checkpoint_summary_ref = context_response.selected_summary_ids.first().cloned();
    let checkpoint_context_version = state.current_context_version(&conversation_id);
    let checkpoint_user_id = state
        .conversation_owner
        .lock()
        .expect("owner mutex poisoned")
        .get(&conversation_id)
        .cloned()
        .unwrap_or_default();
    let checkpoint_conversation_id = conversation_id.clone();
    tokio::spawn(async move {
        for attempt in 1..=3_u32 {
            match checkpoint_state
                .client
                .post(format!(
                    "{}/internal/session/checkpoint",
                    checkpoint_state.config.context_service_base_url
                ))
                .header(INTERNAL_TOKEN_HEADER, checkpoint_secret.as_str())
                .json(&json!({
                    "agent_id": DEFAULT_AGENT_ID,
                    "user_id": checkpoint_user_id,
                    "conversation_id": checkpoint_conversation_id,
                    "schema_version": 1,
                    "working_summary_ref": checkpoint_summary_ref,
                    "runtime_state_blob": {
                        "active_task": "chat",
                        "context_version": checkpoint_context_version,
                        "trace_id": checkpoint_trace_id.clone()
                    },
                    "policy_versions": {
                        "memory_policy": "v1",
                        "session_policy": "v1",
                        "retrieval_policy": "v1"
                    }
                }))
                .send()
                .await
            {
                Ok(r) if r.status().is_success() => return,
                Ok(r) => {
                    checkpoint_state.record_relay_failure(
                        "context_checkpoint_non_success",
                        format!("status={} attempt={attempt}", r.status()),
                        Some(checkpoint_trace_id.clone()),
                        attempt,
                        true,
                    );
                }
                Err(e) => {
                    checkpoint_state.record_relay_failure(
                        "context_checkpoint_request_failed",
                        format!("error={e} attempt={attempt}"),
                        Some(checkpoint_trace_id.clone()),
                        attempt,
                        true,
                    );
                }
            }
            if attempt < 3 {
                sleep(Duration::from_millis(120 * u64::from(attempt))).await;
            }
        }
    });

    Ok(Json(SendMessageResponse {
        user_message_id,
        ai_message_id,
        ai_content_text,
        created_at: DEFAULT_TIMESTAMP.to_string(),
    }))
}

async fn get_internal_message(
    State(state): State<Arc<AppState>>,
    Path((conversation_id, message_id)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Json<InternalMessageResponse>, StatusCode> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)?;
    let messages = state.messages.lock().expect("messages mutex poisoned");
    let items = messages
        .get(&conversation_id)
        .ok_or(StatusCode::NOT_FOUND)?;
    let found = items
        .iter()
        .find(|m| m.message_id == message_id && m.sender_type == "user")
        .ok_or(StatusCode::NOT_FOUND)?;
    let owner_user_id = state
        .conversation_owner
        .lock()
        .expect("owner mutex poisoned")
        .get(&conversation_id)
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(InternalMessageResponse {
        conversation_id,
        owner_user_id,
        content_text: found.content_text.clone(),
        content_type: found.content_type.clone(),
    }))
}

async fn get_chat_relay_observability(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<ChatRelayObservabilityResponse>, StatusCode> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)?;
    let failures = state
        .relay_failures
        .lock()
        .expect("relay_failures mutex poisoned");
    Ok(Json(ChatRelayObservabilityResponse {
        total_failures: failures.len(),
        recent_failures: failures.iter().rev().take(10).cloned().collect(),
    }))
}

async fn list_messages(
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<ListMessagesResponse>, (StatusCode, String)> {
    let user_id = resolve_user_id(&state, &headers).await?;
    assert_conversation_owner(&state, &conversation_id, &user_id)?;
    let items = state
        .messages
        .lock()
        .expect("messages mutex poisoned")
        .get(&conversation_id)
        .cloned()
        .unwrap_or_default();

    Ok(Json(ListMessagesResponse {
        items,
        next_cursor: None,
    }))
}

async fn get_context(
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<GetContextResponse>, (StatusCode, String)> {
    let user_id = resolve_user_id(&state, &headers).await?;
    assert_conversation_owner(&state, &conversation_id, &user_id)?;
    let context = state
        .contexts
        .lock()
        .expect("contexts mutex poisoned")
        .get(&conversation_id)
        .cloned()
        .unwrap_or(ContextView {
            context_version: state.current_context_version(&conversation_id),
            summary: "no context built yet".to_string(),
            last_message_at: None,
        });

    Ok(Json(GetContextResponse {
        context_version: context.context_version,
        summary: context.summary,
        last_message_at: context
            .last_message_at
            .unwrap_or_else(|| DEFAULT_TIMESTAMP.to_string()),
    }))
}

#[derive(Debug)]
struct AppState {
    config: Config,
    client: reqwest::Client,
    id_counter: AtomicU64,
    conversations: Mutex<HashMap<String, ConversationRecord>>,
    conversation_owner: Mutex<HashMap<String, String>>,
    user_primary_conversation: Mutex<HashMap<String, String>>,
    messages: Mutex<HashMap<String, Vec<MessageItem>>>,
    contexts: Mutex<HashMap<String, ContextView>>,
    relay_failures: Mutex<Vec<RelayFailureRecord>>,
}

impl AppState {
    fn new(config: Config) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
            id_counter: AtomicU64::new(1),
            conversations: Mutex::new(HashMap::new()),
            conversation_owner: Mutex::new(HashMap::new()),
            user_primary_conversation: Mutex::new(HashMap::new()),
            messages: Mutex::new(HashMap::new()),
            contexts: Mutex::new(HashMap::new()),
            relay_failures: Mutex::new(vec![]),
        }
    }

    fn next_uuid_like(&self) -> String {
        let next = self.id_counter.fetch_add(1, Ordering::Relaxed);
        format!("00000000-0000-0000-0000-{next:012}")
    }

    fn ensure_conversation_exists(&self, conversation_id: &str, user_id: &str) {
        let mut conversations = self
            .conversations
            .lock()
            .expect("conversations mutex poisoned");
        conversations
            .entry(conversation_id.to_string())
            .or_insert_with(|| ConversationRecord {
                conversation_id: conversation_id.to_string(),
                status: "active".to_string(),
                created_at: DEFAULT_TIMESTAMP.to_string(),
                last_message_at: None,
                context_version: 0,
            });
        let mut owners = self
            .conversation_owner
            .lock()
            .expect("owner mutex poisoned");
        owners
            .entry(conversation_id.to_string())
            .or_insert_with(|| user_id.to_string());
    }

    fn current_context_version(&self, conversation_id: &str) -> u64 {
        self.conversations
            .lock()
            .expect("conversations mutex poisoned")
            .get(conversation_id)
            .map(|conversation| conversation.context_version)
            .unwrap_or(0)
    }

    fn record_relay_failure(
        &self,
        stage: &str,
        detail: String,
        trace_id: Option<String>,
        attempt_count: u32,
        retryable: bool,
    ) {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .to_string();
        self.relay_failures
            .lock()
            .expect("relay_failures mutex poisoned")
            .push(RelayFailureRecord {
                id: self.next_uuid_like(),
                stage: stage.to_string(),
                detail,
                trace_id,
                attempt_count,
                retryable,
                created_at,
            });
    }

    async fn build_context(
        &self,
        user_id: &str,
        conversation_id: &str,
        input: &str,
        trace_id: &str,
    ) -> Result<ContextBuildResponse, String> {
        let response = self
            .client
            .post(format!(
                "{}/internal/context/build",
                self.config.context_service_base_url
            ))
            .header(
                INTERNAL_TOKEN_HEADER,
                self.config.internal_shared_secret.as_str(),
            )
            .json(&serde_json::json!({
                "user_id": user_id,
                "agent_id": DEFAULT_AGENT_ID,
                "conversation_id": conversation_id,
                "input": input,
                "task_type": "chat",
                "max_tokens": 8000,
                "memory_limit": 20,
                "summary_limit": 3,
                "reply_style": "brief",
                "trace_id": trace_id,
                "retrieval_modes": ["structured", "semantic", "temporal"]
            }))
            .send()
            .await
            .map_err(|error| format!("context-service request failed: {error}"))?;
        if !response.status().is_success() {
            return Err(format!("context-service status {}", response.status()));
        }

        response
            .json::<ContextBuildResponse>()
            .await
            .map_err(|error| format!("context-service decode failed: {error}"))
    }

    async fn invoke_model_gateway(
        &self,
        input: &str,
        context: &ContextBuildResponse,
        trace_id: &str,
    ) -> Result<String, String> {
        let response = self
            .client
            .post(format!(
                "{}/internal/v1/invoke",
                self.config.model_gateway_base_url
            ))
            .json(&serde_json::json!({
                "capability_name": "chat.respond",
                "trace_id": trace_id,
                "payload": {
                    "messages": [
                        {
                            "role": "user",
                            "content": input
                        }
                    ],
                    "context": context
                }
            }))
            .send()
            .await
            .map_err(|error| format!("model-gateway request failed: {error}"))?;
        if !response.status().is_success() {
            return Err(format!("model-gateway status {}", response.status()));
        }

        let decoded = response
            .json::<ModelInvokeResponse>()
            .await
            .map_err(|error| format!("model-gateway decode failed: {error}"))?;

        Ok(format!(
            "[{}:{}] {}",
            decoded.capability_name, decoded.model_id, decoded.output_text
        ))
    }
}

#[derive(Debug, Deserialize)]
struct CreateConversationRequest {
    idempotency_key: Option<String>,
}

#[derive(Debug, Serialize)]
struct CreateConversationResponse {
    conversation_id: String,
    status: String,
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct SendMessageRequest {
    content_type: String,
    content_text: String,
    idempotency_key: Option<String>,
}

#[derive(Debug, Serialize)]
struct SendMessageResponse {
    user_message_id: String,
    ai_message_id: String,
    ai_content_text: String,
    created_at: String,
}

#[derive(Debug, Serialize, Clone)]
struct MessageItem {
    message_id: String,
    sender_type: String,
    content_text: String,
    content_type: String,
    created_at: String,
}

#[derive(Debug, Serialize)]
struct ListMessagesResponse {
    items: Vec<MessageItem>,
    next_cursor: Option<String>,
}

#[derive(Debug, Serialize)]
struct GetContextResponse {
    context_version: u64,
    summary: String,
    last_message_at: String,
}

#[derive(Debug, Serialize)]
struct InternalMessageResponse {
    conversation_id: String,
    owner_user_id: String,
    content_text: String,
    content_type: String,
}

#[derive(Debug, Serialize, Clone)]
struct RelayFailureRecord {
    id: String,
    stage: String,
    detail: String,
    trace_id: Option<String>,
    attempt_count: u32,
    retryable: bool,
    created_at: String,
}

#[derive(Debug, Serialize)]
struct ChatRelayObservabilityResponse {
    total_failures: usize,
    recent_failures: Vec<RelayFailureRecord>,
}

#[derive(Debug)]
struct ConversationRecord {
    conversation_id: String,
    status: String,
    created_at: String,
    last_message_at: Option<String>,
    context_version: u64,
}

#[derive(Debug, Clone)]
struct ContextView {
    context_version: u64,
    summary: String,
    last_message_at: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
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

#[derive(Debug, Deserialize, Serialize)]
struct TokenBudget {
    max_tokens: i32,
    memory_limit: i32,
    summary_limit: i32,
}

#[derive(Debug, Deserialize)]
struct ModelInvokeResponse {
    capability_name: String,
    model_id: String,
    output_text: String,
}
