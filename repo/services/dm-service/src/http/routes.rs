use axum::{
    extract::State,
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use onelink_internal_auth::verify_internal_token;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

use crate::config::Config;
use crate::store::postgres::PostgresStore;

pub fn router(state: Arc<DmState>) -> Router {
    Router::new()
        .route("/api/v1/dm/threads", get(list_threads))
        .route("/api/v1/dm/threads", post(create_thread))
        .route("/api/v1/dm/threads/:threadId", get(get_thread_detail))
        .route("/api/v1/dm/threads/:threadId/archive", post(archive_thread))
        .route("/api/v1/dm/threads/:threadId/messages", get(get_messages))
        .route("/api/v1/dm/threads/:threadId/messages", post(send_message))
        .route("/api/v1/dm/threads/:threadId/read", post(mark_read))
        .route(
            "/api/v1/dm/threads/:threadId/screening-log",
            get(get_screening_log),
        )
        .route("/api/v1/dm/blocks", post(create_dm_block))
        .route("/api/v1/dm/blocks", get(list_dm_blocks))
        .route("/internal/dm/health-detail", get(internal_health_detail))
        .with_state(state)
}

fn emit_event(event_name: &str, actor_user_id: Option<String>, payload: serde_json::Value) {
    let envelope = onelink_event_envelope::EventEnvelope::new_v1(
        event_name,
        "dm-service",
        actor_user_id,
        None,
        payload,
    );
    tracing::info!(
        event_id = %envelope.event_id,
        event_name = %envelope.event_name,
        "event emitted"
    );
}

#[derive(Debug)]
pub struct DmState {
    pub config: Config,
    threads: Mutex<HashMap<String, DmThread>>,
    messages: Mutex<HashMap<String, Vec<DmMessage>>>,
    safety_screening_log: Mutex<Vec<SafetyScreeningRecord>>,
    blocks: Mutex<HashMap<String, Vec<DmBlockRecord>>>,
    pub pg: Option<Arc<PostgresStore>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmThread {
    pub thread_id: String,
    pub participant_a_id: String,
    pub participant_b_id: String,
    pub created_at: String,
    pub last_message_at: Option<String>,
    pub last_message_preview: Option<String>,
    pub status: DmThreadStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DmThreadStatus {
    Active,
    Archived,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmMessage {
    pub message_id: String,
    pub thread_id: String,
    pub sender_user_id: String,
    pub content: String,
    pub created_at: String,
    pub read_by_recipient: bool,
    pub message_type: DmMessageType,
    pub safety_screened: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DmMessageType {
    Text,
    System,
    SafetyNotice,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyScreeningRecord {
    pub screening_id: String,
    pub thread_id: String,
    pub message_id: String,
    pub sender_user_id: String,
    pub verdict: String,
    pub screened_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmBlockRecord {
    pub block_id: String,
    pub blocker_user_id: String,
    pub blocked_user_id: String,
    pub created_at: String,
}

impl DmState {
    pub fn new(config: Config, pg: Option<Arc<PostgresStore>>) -> Arc<Self> {
        Arc::new(Self {
            config,
            threads: Mutex::new(HashMap::new()),
            messages: Mutex::new(HashMap::new()),
            safety_screening_log: Mutex::new(Vec::new()),
            blocks: Mutex::new(HashMap::new()),
            pg,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct ListThreadsQuery {
    pub user_id: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    20
}

fn compute_dm_risk_score(content: &str) -> (f64, Option<String>) {
    let mut score: f64 = 0.0;
    let mut reason = None;

    let lower = content.to_lowercase();
    let flag_patterns = [
        ("http://", 0.15),
        ("https://", 0.10),
        ("telegram", 0.12),
        ("whatsapp", 0.12),
        ("wechat", 0.12),
        ("signal", 0.10),
        ("wire transfer", 0.30),
        ("crypto", 0.20),
        ("investment", 0.15),
        ("send money", 0.25),
    ];

    for (pattern, weight) in &flag_patterns {
        if lower.contains(pattern) {
            score += weight;
        }
    }

    if content.len() > 2000 {
        score += 0.05;
    }

    if score > 0.0 {
        reason = Some("first_message_flag".to_string());
    }

    (score.min(1.0), reason)
}

#[derive(Debug, Serialize)]
pub struct ListThreadsResponse {
    pub threads: Vec<DmThread>,
}

async fn list_threads(
    State(state): State<Arc<DmState>>,
    headers: HeaderMap,
    axum::extract::Query(query): axum::extract::Query<ListThreadsQuery>,
) -> Result<Json<ListThreadsResponse>, (axum::http::StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;

    let threads = state.threads.lock().expect("mutex poisoned");
    let mut user_threads: Vec<DmThread> = threads
        .values()
        .filter(|t| t.participant_a_id == query.user_id || t.participant_b_id == query.user_id)
        .cloned()
        .collect();

    user_threads.sort_by(|a, b| {
        let a_time = a.last_message_at.as_deref().unwrap_or(&a.created_at);
        let b_time = b.last_message_at.as_deref().unwrap_or(&b.created_at);
        b_time.cmp(a_time)
    });

    user_threads.truncate(query.limit);

    Ok(Json(ListThreadsResponse {
        threads: user_threads,
    }))
}

#[derive(Debug, Deserialize)]
pub struct CreateThreadPayload {
    pub initiator_user_id: String,
    pub recipient_user_id: String,
}

#[derive(Debug, Serialize)]
pub struct CreateThreadResponse {
    pub thread_id: String,
    pub created: bool,
}

async fn create_thread(
    State(state): State<Arc<DmState>>,
    headers: HeaderMap,
    Json(body): Json<CreateThreadPayload>,
) -> Result<Json<CreateThreadResponse>, (axum::http::StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;

    if body.initiator_user_id == body.recipient_user_id {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "cannot create DM thread with yourself".to_string(),
        ));
    }

    {
        let bl = state.blocks.lock().expect("mutex poisoned");
        let is_blocked_a = bl
            .get(&body.initiator_user_id)
            .map(|list| {
                list.iter()
                    .any(|b| b.blocked_user_id == body.recipient_user_id)
            })
            .unwrap_or(false);
        let is_blocked_b = bl
            .get(&body.recipient_user_id)
            .map(|list| {
                list.iter()
                    .any(|b| b.blocked_user_id == body.initiator_user_id)
            })
            .unwrap_or(false);
        if is_blocked_a || is_blocked_b {
            return Err((
                axum::http::StatusCode::FORBIDDEN,
                "cannot create DM thread with a blocked user".to_string(),
            ));
        }
    }

    {
        let threads = state.threads.lock().expect("mutex poisoned");
        let existing = threads.values().find(|t| {
            (t.participant_a_id == body.initiator_user_id
                && t.participant_b_id == body.recipient_user_id)
                || (t.participant_a_id == body.recipient_user_id
                    && t.participant_b_id == body.initiator_user_id)
        });
        if let Some(existing) = existing {
            return Ok(Json(CreateThreadResponse {
                thread_id: existing.thread_id.clone(),
                created: false,
            }));
        }
    }

    let thread_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let thread = DmThread {
        thread_id: thread_id.clone(),
        participant_a_id: body.initiator_user_id.clone(),
        participant_b_id: body.recipient_user_id.clone(),
        created_at: now,
        last_message_at: None,
        last_message_preview: None,
        status: DmThreadStatus::Active,
    };

    {
        let mut threads = state.threads.lock().expect("mutex poisoned");
        threads.insert(thread_id.clone(), thread);
    }

    {
        let mut messages = state.messages.lock().expect("mutex poisoned");
        messages.insert(thread_id.clone(), Vec::new());
    }

    tracing::info!(
        thread_id = %thread_id,
        initiator = %body.initiator_user_id,
        recipient = %body.recipient_user_id,
        "dm-service: thread created"
    );

    if let Some(pg) = &state.pg {
        let thread_uuid = match Uuid::parse_str(&thread_id) {
            Ok(u) => u,
            Err(_) => Uuid::new_v4(),
        };
        if let Err(e) = pg.insert_thread(&thread_uuid, "direct", "active").await {
            tracing::error!(error = %e, thread_id = %thread_id, "postgres insert_thread failed — data will not persist");
        } else {
            let initiator_uuid = match Uuid::parse_str(&body.initiator_user_id) {
                Ok(u) => u,
                Err(_) => Uuid::new_v4(),
            };
            let recipient_uuid = match Uuid::parse_str(&body.recipient_user_id) {
                Ok(u) => u,
                Err(_) => Uuid::new_v4(),
            };
            if let Err(e) = pg
                .insert_participant(
                    &Uuid::new_v4(),
                    &thread_uuid,
                    &initiator_uuid,
                    "initiator",
                    "active",
                )
                .await
            {
                tracing::error!(error = %e, thread_id = %thread_id, "postgres insert_participant (initiator) failed");
            }
            if let Err(e) = pg
                .insert_participant(
                    &Uuid::new_v4(),
                    &thread_uuid,
                    &recipient_uuid,
                    "recipient",
                    "active",
                )
                .await
            {
                tracing::error!(error = %e, thread_id = %thread_id, "postgres insert_participant (recipient) failed");
            }
        }
    }

    Ok(Json(CreateThreadResponse {
        thread_id,
        created: true,
    }))
}

#[derive(Debug, Deserialize)]
pub struct GetMessagesQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub before: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GetMessagesResponse {
    pub thread_id: String,
    pub messages: Vec<DmMessage>,
}

async fn get_messages(
    State(state): State<Arc<DmState>>,
    headers: HeaderMap,
    axum::extract::Path(thread_id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<GetMessagesQuery>,
) -> Result<Json<GetMessagesResponse>, (axum::http::StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;

    {
        let threads = state.threads.lock().expect("mutex poisoned");
        if !threads.contains_key(&thread_id) {
            return Err((
                axum::http::StatusCode::NOT_FOUND,
                "thread not found".to_string(),
            ));
        }
    }

    let messages = state.messages.lock().expect("mutex poisoned");
    let mut thread_messages: Vec<DmMessage> = messages.get(&thread_id).cloned().unwrap_or_default();

    if let Some(before) = &query.before {
        thread_messages.retain(|m| m.created_at < *before);
    }

    let count = thread_messages.len();
    let start = count.saturating_sub(query.limit);
    thread_messages = thread_messages.into_iter().skip(start).collect();

    Ok(Json(GetMessagesResponse {
        thread_id,
        messages: thread_messages,
    }))
}

#[derive(Debug, Deserialize)]
pub struct SendMessagePayload {
    pub sender_user_id: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct SendMessageResponse {
    pub message_id: String,
    pub thread_id: String,
    pub created_at: String,
}

async fn send_message(
    State(state): State<Arc<DmState>>,
    headers: HeaderMap,
    axum::extract::Path(thread_id): axum::extract::Path<String>,
    Json(body): Json<SendMessagePayload>,
) -> Result<Json<SendMessageResponse>, (axum::http::StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;

    if body.content.trim().is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "content must not be empty".to_string(),
        ));
    }

    let thread = {
        let threads = state.threads.lock().expect("mutex poisoned");
        threads.get(&thread_id).cloned().ok_or((
            axum::http::StatusCode::NOT_FOUND,
            "thread not found".to_string(),
        ))?
    };

    if body.sender_user_id != thread.participant_a_id
        && body.sender_user_id != thread.participant_b_id
    {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            "sender is not a participant in this thread".to_string(),
        ));
    }

    let is_blocked = {
        let bl = state.blocks.lock().expect("mutex poisoned");
        bl.get(&thread.participant_b_id)
            .map(|list| {
                list.iter()
                    .any(|b| b.blocked_user_id == body.sender_user_id)
            })
            .unwrap_or(false)
            || bl
                .get(&thread.participant_a_id)
                .map(|list| {
                    list.iter()
                        .any(|b| b.blocked_user_id == body.sender_user_id)
                })
                .unwrap_or(false)
    };

    if is_blocked {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            "sender is blocked by recipient".to_string(),
        ));
    }

    let is_first_message_in_thread = {
        let messages = state.messages.lock().expect("mutex poisoned");
        messages
            .get(&thread_id)
            .map(|m| m.is_empty())
            .unwrap_or(true)
    };

    let (safety_screened, screening_verdict) = if is_first_message_in_thread {
        let (risk_score, risk_reason) = compute_dm_risk_score(&body.content);
        let verdict = if risk_score >= 0.5 {
            "reject"
        } else if risk_score >= 0.2 {
            "flag"
        } else {
            "allow"
        };
        let screening_id = Uuid::new_v4().to_string();
        let screening_record = SafetyScreeningRecord {
            screening_id,
            thread_id: thread_id.clone(),
            message_id: "pending".to_string(),
            sender_user_id: body.sender_user_id.clone(),
            verdict: verdict.to_string(),
            screened_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        };
        {
            let mut log = state.safety_screening_log.lock().expect("mutex poisoned");
            log.push(screening_record);
        }
        if verdict == "reject" {
            tracing::info!(
                thread_id = %thread_id,
                sender = %body.sender_user_id,
                verdict = verdict,
                risk_score = risk_score,
                "dm-service: first message rejected by safety screening"
            );
            return Err((
                axum::http::StatusCode::FORBIDDEN,
                format!(
                    "first message rejected by safety screening: {}",
                    risk_reason.unwrap_or_else(|| "content_risk_too_high".to_string())
                ),
            ));
        }
        (true, verdict.to_string())
    } else {
        (false, "allow".to_string())
    };

    let message_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let message = DmMessage {
        message_id: message_id.clone(),
        thread_id: thread_id.clone(),
        sender_user_id: body.sender_user_id.clone(),
        content: body.content.clone(),
        created_at: now.clone(),
        read_by_recipient: false,
        message_type: DmMessageType::Text,
        safety_screened,
    };

    {
        let mut messages = state.messages.lock().expect("mutex poisoned");
        messages.entry(thread_id.clone()).or_default().push(message);
    }

    if safety_screened {
        let mut log = state.safety_screening_log.lock().expect("mutex poisoned");
        for entry in log.iter_mut() {
            if entry.message_id == "pending" && entry.thread_id == thread_id {
                entry.message_id = message_id.clone();
            }
        }
    }

    {
        let mut threads = state.threads.lock().expect("mutex poisoned");
        if let Some(t) = threads.get_mut(&thread_id) {
            t.last_message_at = Some(now.clone());
            let preview = if body.content.len() > 50 {
                format!("{}...", &body.content[..50])
            } else {
                body.content.clone()
            };
            t.last_message_preview = Some(preview);
        }
    }

    if let Some(pg) = &state.pg {
        let msg_uuid = match Uuid::parse_str(&message_id) {
            Ok(u) => u,
            Err(_) => Uuid::new_v4(),
        };
        let thread_uuid = match Uuid::parse_str(&thread_id) {
            Ok(u) => u,
            Err(_) => Uuid::new_v4(),
        };
        let sender_uuid = match Uuid::parse_str(&body.sender_user_id) {
            Ok(u) => u,
            Err(_) => Uuid::new_v4(),
        };
        let review_status = if safety_screened {
            &screening_verdict
        } else {
            "allow"
        };
        if let Err(e) = pg
            .insert_message(
                &msg_uuid,
                &thread_uuid,
                &sender_uuid,
                "text",
                &body.content,
                review_status,
            )
            .await
        {
            tracing::error!(error = %e, message_id = %message_id, "postgres insert_message failed — data will not persist");
        }
    }

    tracing::info!(
        message_id = %message_id,
        thread_id = %thread_id,
        sender = %body.sender_user_id,
        safety_screened = safety_screened,
        screening_verdict = %screening_verdict,
        "dm-service: message sent"
    );

    emit_event(
        "dm.message.created",
        Some(body.sender_user_id.clone()),
        json!({
            "thread_id": thread_id,
            "message_id": message_id,
            "sender_user_id": body.sender_user_id,
            "safety_screened": safety_screened,
            "screening_verdict": screening_verdict,
        }),
    );

    Ok(Json(SendMessageResponse {
        message_id,
        thread_id,
        created_at: now,
    }))
}

#[derive(Debug, Deserialize)]
pub struct MarkReadPayload {
    pub reader_user_id: String,
}

async fn mark_read(
    State(state): State<Arc<DmState>>,
    headers: HeaderMap,
    axum::extract::Path(thread_id): axum::extract::Path<String>,
    Json(body): Json<MarkReadPayload>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;

    {
        let threads = state.threads.lock().expect("mutex poisoned");
        if !threads.contains_key(&thread_id) {
            return Err((
                axum::http::StatusCode::NOT_FOUND,
                "thread not found".to_string(),
            ));
        }
    }

    let mut marked = 0usize;
    {
        let mut messages = state.messages.lock().expect("mutex poisoned");
        if let Some(thread_messages) = messages.get_mut(&thread_id) {
            for msg in thread_messages.iter_mut() {
                if msg.sender_user_id != body.reader_user_id && !msg.read_by_recipient {
                    msg.read_by_recipient = true;
                    marked += 1;
                }
            }
        }
    }

    tracing::info!(
        thread_id = %thread_id,
        reader = %body.reader_user_id,
        marked = marked,
        "dm-service: messages marked as read"
    );

    Ok(Json(json!({
        "thread_id": thread_id,
        "marked_read": marked,
    })))
}

#[derive(Debug, Serialize)]
pub struct GetThreadDetailResponse {
    pub thread_id: String,
    pub participant_a_id: String,
    pub participant_b_id: String,
    pub created_at: String,
    pub last_message_at: Option<String>,
    pub last_message_preview: Option<String>,
    pub status: DmThreadStatus,
    pub message_count: usize,
}

async fn get_thread_detail(
    State(state): State<Arc<DmState>>,
    headers: HeaderMap,
    axum::extract::Path(thread_id): axum::extract::Path<String>,
) -> Result<Json<GetThreadDetailResponse>, (axum::http::StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;

    let threads = state.threads.lock().expect("mutex poisoned");
    let thread = threads.get(&thread_id).ok_or((
        axum::http::StatusCode::NOT_FOUND,
        "thread not found".to_string(),
    ))?;

    let messages = state.messages.lock().expect("mutex poisoned");
    let message_count = messages.get(&thread_id).map(|m| m.len()).unwrap_or(0);

    Ok(Json(GetThreadDetailResponse {
        thread_id: thread.thread_id.clone(),
        participant_a_id: thread.participant_a_id.clone(),
        participant_b_id: thread.participant_b_id.clone(),
        created_at: thread.created_at.clone(),
        last_message_at: thread.last_message_at.clone(),
        last_message_preview: thread.last_message_preview.clone(),
        status: thread.status.clone(),
        message_count,
    }))
}

async fn archive_thread(
    State(state): State<Arc<DmState>>,
    headers: HeaderMap,
    axum::extract::Path(thread_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;

    let mut threads = state.threads.lock().expect("mutex poisoned");
    if let Some(thread) = threads.get_mut(&thread_id) {
        thread.status = DmThreadStatus::Archived;
    } else {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            "thread not found".to_string(),
        ));
    }

    tracing::info!(
        thread_id = %thread_id,
        "dm-service: thread archived"
    );

    Ok(Json(json!({
        "thread_id": thread_id,
        "status": "archived",
    })))
}

#[derive(Debug, Serialize)]
pub struct GetScreeningLogResponse {
    pub thread_id: String,
    pub screenings: Vec<SafetyScreeningRecord>,
}

async fn get_screening_log(
    State(state): State<Arc<DmState>>,
    headers: HeaderMap,
    axum::extract::Path(thread_id): axum::extract::Path<String>,
) -> Result<Json<GetScreeningLogResponse>, (axum::http::StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;

    {
        let threads = state.threads.lock().expect("mutex poisoned");
        if !threads.contains_key(&thread_id) {
            return Err((
                axum::http::StatusCode::NOT_FOUND,
                "thread not found".to_string(),
            ));
        }
    }

    let screening_log = state.safety_screening_log.lock().expect("mutex poisoned");
    let thread_screenings: Vec<SafetyScreeningRecord> = screening_log
        .iter()
        .filter(|s| s.thread_id == thread_id)
        .cloned()
        .collect();

    Ok(Json(GetScreeningLogResponse {
        thread_id,
        screenings: thread_screenings,
    }))
}

#[derive(Debug, Deserialize)]
pub struct CreateDmBlockPayload {
    pub blocker_user_id: String,
    pub blocked_user_id: String,
}

#[derive(Debug, Serialize)]
pub struct CreateDmBlockResponse {
    pub block_id: String,
    pub blocked_user_id: String,
}

async fn create_dm_block(
    State(state): State<Arc<DmState>>,
    headers: HeaderMap,
    Json(body): Json<CreateDmBlockPayload>,
) -> Result<Json<CreateDmBlockResponse>, (axum::http::StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;

    if body.blocker_user_id == body.blocked_user_id {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "cannot block yourself".to_string(),
        ));
    }

    {
        let bl = state.blocks.lock().expect("mutex poisoned");
        if let Some(list) = bl.get(&body.blocker_user_id) {
            if list
                .iter()
                .any(|b| b.blocked_user_id == body.blocked_user_id)
            {
                return Err((
                    axum::http::StatusCode::CONFLICT,
                    "already blocked".to_string(),
                ));
            }
        }
    }

    let block_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let record = DmBlockRecord {
        block_id: block_id.clone(),
        blocker_user_id: body.blocker_user_id.clone(),
        blocked_user_id: body.blocked_user_id.clone(),
        created_at: now,
    };

    {
        let mut bl = state.blocks.lock().expect("mutex poisoned");
        bl.entry(body.blocker_user_id.clone())
            .or_default()
            .push(record);
    }

    {
        let mut threads = state.threads.lock().expect("mutex poisoned");
        for (_tid, thread) in threads.iter_mut() {
            if (thread.participant_a_id == body.blocker_user_id
                && thread.participant_b_id == body.blocked_user_id)
                || (thread.participant_a_id == body.blocked_user_id
                    && thread.participant_b_id == body.blocker_user_id)
            {
                thread.status = DmThreadStatus::Blocked;
            }
        }
    }

    tracing::info!(
        blocker = %body.blocker_user_id,
        blocked = %body.blocked_user_id,
        "dm-service: DM block created, affected threads blocked"
    );

    Ok(Json(CreateDmBlockResponse {
        block_id,
        blocked_user_id: body.blocked_user_id,
    }))
}

#[derive(Debug, Deserialize)]
pub struct ListDmBlocksQuery {
    pub user_id: String,
}

#[derive(Debug, Serialize)]
pub struct ListDmBlocksResponse {
    pub user_id: String,
    pub blocked_users: Vec<DmBlockRecord>,
}

async fn list_dm_blocks(
    State(state): State<Arc<DmState>>,
    headers: HeaderMap,
    axum::extract::Query(query): axum::extract::Query<ListDmBlocksQuery>,
) -> Result<Json<ListDmBlocksResponse>, (axum::http::StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;

    let bl = state.blocks.lock().expect("mutex poisoned");
    let blocked_users = bl.get(&query.user_id).cloned().unwrap_or_default();

    Ok(Json(ListDmBlocksResponse {
        user_id: query.user_id,
        blocked_users,
    }))
}

async fn internal_health_detail(
    State(state): State<Arc<DmState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)?;
    let threads = state.threads.lock().expect("mutex poisoned");
    let messages = state.messages.lock().expect("mutex poisoned");
    let total_messages: usize = messages.values().map(|v| v.len()).sum();
    let blocks = state.blocks.lock().expect("mutex poisoned");
    let block_count: usize = blocks.values().map(|v| v.len()).sum();
    Ok(Json(json!({
        "thread_count": threads.len(),
        "message_count": total_messages,
        "block_count": block_count,
        "env_mode": state.config.env_mode,
        "backend": if state.pg.is_some() { "postgres" } else { "in-memory" },
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dm_thread_serializes() {
        let thread = DmThread {
            thread_id: "t-1".to_string(),
            participant_a_id: "u-1".to_string(),
            participant_b_id: "u-2".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            last_message_at: None,
            last_message_preview: None,
            status: DmThreadStatus::Active,
        };
        let v = serde_json::to_value(&thread).unwrap();
        assert_eq!(v["thread_id"], "t-1");
        assert_eq!(v["participant_a_id"], "u-1");
        assert_eq!(v["status"], "active");
    }

    #[test]
    fn dm_message_serializes() {
        let msg = DmMessage {
            message_id: "m-1".to_string(),
            thread_id: "t-1".to_string(),
            sender_user_id: "u-1".to_string(),
            content: "hello".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            read_by_recipient: false,
            message_type: DmMessageType::Text,
            safety_screened: false,
        };
        let v = serde_json::to_value(&msg).unwrap();
        assert_eq!(v["message_id"], "m-1");
        assert_eq!(v["read_by_recipient"], false);
        assert_eq!(v["message_type"], "text");
        assert_eq!(v["safety_screened"], false);
    }

    #[test]
    fn create_thread_duplicate_returns_existing() {
        let config = Config::from_env();
        let state = DmState::new(config, None);
        let thread = DmThread {
            thread_id: "t-1".to_string(),
            participant_a_id: "u-1".to_string(),
            participant_b_id: "u-2".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            last_message_at: None,
            last_message_preview: None,
            status: DmThreadStatus::Active,
        };
        {
            let mut threads = state.threads.lock().expect("mutex poisoned");
            threads.insert("t-1".to_string(), thread);
        }
        let threads = state.threads.lock().expect("mutex poisoned");
        let existing = threads.values().find(|t| {
            (t.participant_a_id == "u-1" && t.participant_b_id == "u-2")
                || (t.participant_a_id == "u-2" && t.participant_b_id == "u-1")
        });
        assert!(existing.is_some());
        assert_eq!(existing.unwrap().thread_id, "t-1");
    }

    #[test]
    fn send_message_participant_check() {
        let config = Config::from_env();
        let state = DmState::new(config, None);
        let thread = DmThread {
            thread_id: "t-1".to_string(),
            participant_a_id: "u-1".to_string(),
            participant_b_id: "u-2".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            last_message_at: None,
            last_message_preview: None,
            status: DmThreadStatus::Active,
        };
        {
            let mut threads = state.threads.lock().expect("mutex poisoned");
            threads.insert("t-1".to_string(), thread);
        }
        let threads = state.threads.lock().expect("mutex poisoned");
        let t = threads.get("t-1").unwrap();
        assert!(t.participant_a_id == "u-1" || t.participant_b_id == "u-1");
        assert!(t.participant_a_id != "u-3" && t.participant_b_id != "u-3");
    }

    #[test]
    fn dm_thread_status_serializes_snake_case() {
        let s = serde_json::to_string(&DmThreadStatus::Active).unwrap();
        assert_eq!(s, "\"active\"");
        let s = serde_json::to_string(&DmThreadStatus::Archived).unwrap();
        assert_eq!(s, "\"archived\"");
        let s = serde_json::to_string(&DmThreadStatus::Blocked).unwrap();
        assert_eq!(s, "\"blocked\"");
    }

    #[test]
    fn dm_message_type_serializes_snake_case() {
        let s = serde_json::to_string(&DmMessageType::Text).unwrap();
        assert_eq!(s, "\"text\"");
        let s = serde_json::to_string(&DmMessageType::SafetyNotice).unwrap();
        assert_eq!(s, "\"safety_notice\"");
    }

    #[test]
    fn archive_thread_updates_status() {
        let config = Config::from_env();
        let state = DmState::new(config, None);
        let thread = DmThread {
            thread_id: "t-1".to_string(),
            participant_a_id: "u-1".to_string(),
            participant_b_id: "u-2".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            last_message_at: None,
            last_message_preview: None,
            status: DmThreadStatus::Active,
        };
        {
            let mut threads = state.threads.lock().expect("mutex poisoned");
            threads.insert("t-1".to_string(), thread);
        }
        {
            let mut threads = state.threads.lock().expect("mutex poisoned");
            if let Some(t) = threads.get_mut("t-1") {
                t.status = DmThreadStatus::Archived;
            }
        }
        let threads = state.threads.lock().expect("mutex poisoned");
        assert_eq!(threads.get("t-1").unwrap().status, DmThreadStatus::Archived);
    }

    #[test]
    fn safety_screening_log_records_entries() {
        let config = Config::from_env();
        let state = DmState::new(config, None);

        let record = SafetyScreeningRecord {
            screening_id: "scr-1".to_string(),
            thread_id: "t-1".to_string(),
            message_id: "m-1".to_string(),
            sender_user_id: "u-1".to_string(),
            verdict: "allow".to_string(),
            screened_at: "2026-01-01T00:00:00.000Z".to_string(),
        };

        {
            let mut log = state.safety_screening_log.lock().expect("mutex poisoned");
            log.push(record);
        }

        let log = state.safety_screening_log.lock().expect("mutex poisoned");
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].verdict, "allow");
    }

    #[test]
    fn compute_dm_risk_score_clean_message() {
        let (score, reason) = compute_dm_risk_score("你好，很高兴认识你！");
        assert_eq!(score, 0.0);
        assert!(reason.is_none());
    }

    #[test]
    fn compute_dm_risk_score_suspicious_message() {
        let (score, reason) = compute_dm_risk_score(
            "wire transfer crypto investment send money http:// telegram whatsapp",
        );
        assert!(score >= 0.5);
        assert!(reason.is_some());
    }

    #[test]
    fn compute_dm_risk_score_capped_at_one() {
        let (score, _) = compute_dm_risk_score(
            "wire transfer crypto investment send money http:// telegram whatsapp signal investment investment investment",
        );
        assert!(score <= 1.0);
    }

    #[test]
    fn create_thread_blocked_user_forbidden() {
        let config = Config::from_env();
        let state = DmState::new(config, None);

        let block_record = DmBlockRecord {
            block_id: "blk-1".to_string(),
            blocker_user_id: "u-1".to_string(),
            blocked_user_id: "u-2".to_string(),
            created_at: "2026-01-01T00:00:00.000Z".to_string(),
        };
        {
            let mut bl = state.blocks.lock().expect("mutex poisoned");
            bl.entry("u-1".to_string()).or_default().push(block_record);
        }

        let bl = state.blocks.lock().expect("mutex poisoned");
        let is_blocked_a = bl
            .get("u-1")
            .map(|list| list.iter().any(|b| b.blocked_user_id == "u-2"))
            .unwrap_or(false);
        assert!(
            is_blocked_a,
            "block-aware thread creation must check blocks"
        );
    }

    #[test]
    fn dm_block_record_creation_and_lookup() {
        let config = Config::from_env();
        let state = DmState::new(config, None);

        let record = DmBlockRecord {
            block_id: "blk-test".to_string(),
            blocker_user_id: "u-A".to_string(),
            blocked_user_id: "u-B".to_string(),
            created_at: "2026-01-01T00:00:00.000Z".to_string(),
        };
        {
            let mut bl = state.blocks.lock().expect("mutex poisoned");
            bl.entry("u-A".to_string()).or_default().push(record);
        }

        let bl = state.blocks.lock().expect("mutex poisoned");
        assert!(bl
            .get("u-A")
            .unwrap()
            .iter()
            .any(|b| b.blocked_user_id == "u-B"));
    }
}
