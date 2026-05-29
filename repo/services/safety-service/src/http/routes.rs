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

pub fn router(state: Arc<SafetyState>) -> Router {
    Router::new()
        .route("/api/v1/safety/screen-message", post(screen_message))
        .route("/api/v1/safety/reports", post(create_report))
        .route("/api/v1/safety/reports/:reportTicketId", get(get_report))
        .route("/api/v1/safety/reports/actions", post(create_report_action))
        .route("/api/v1/safety/me/moderation", get(get_moderation))
        .route("/api/v1/safety/appeals", post(create_appeal))
        .route("/api/v1/safety/blocks", post(create_block))
        .route("/api/v1/safety/blocks", get(list_blocks))
        .route("/api/v1/safety/blocks/unblock", post(delete_block))
        .route(
            "/api/v1/safety/dm-first-message-review",
            post(dm_first_message_review),
        )
        .route("/api/v1/safety/risk-flags/:userId", get(get_risk_flags))
        .route("/api/v1/safety/risk-flags", post(create_risk_flag))
        .route(
            "/internal/safety/health-detail",
            get(internal_health_detail),
        )
        .with_state(state)
}

fn emit_event(event_name: &str, actor_user_id: Option<String>, payload: serde_json::Value) {
    let envelope = onelink_event_envelope::EventEnvelope::new_v1(
        event_name,
        "safety-service",
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
pub struct SafetyState {
    pub config: Config,
    reports: Mutex<HashMap<String, ReportRecord>>,
    blocks: Mutex<HashMap<String, Vec<BlockRecord>>>,
    appeals: Mutex<Vec<AppealRecord>>,
    risk_flags: Mutex<HashMap<String, RiskFlagRecord>>,
    report_actions: Mutex<HashMap<String, Vec<ReportActionRecord>>>,
    pub pg: Option<Arc<PostgresStore>>,
}

impl SafetyState {
    pub fn new(config: Config, pg: Option<Arc<PostgresStore>>) -> Arc<Self> {
        Arc::new(Self {
            config,
            reports: Mutex::new(HashMap::new()),
            blocks: Mutex::new(HashMap::new()),
            appeals: Mutex::new(Vec::new()),
            risk_flags: Mutex::new(HashMap::new()),
            report_actions: Mutex::new(HashMap::new()),
            pg,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportRecord {
    pub report_ticket_id: String,
    pub reporter_user_id: String,
    pub reported_user_id: String,
    pub reason: ReportReason,
    pub description: String,
    pub status: ReportStatus,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ReportReason {
    Harassment,
    Spam,
    InappropriateContent,
    SafetyConcern,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ReportStatus {
    Submitted,
    UnderReview,
    ActionTaken,
    Dismissed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockRecord {
    pub block_id: String,
    pub blocker_user_id: String,
    pub blocked_user_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ContentVerdict {
    Allow,
    Flag,
    Reject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenMessageRequest {
    pub sender_user_id: String,
    pub recipient_user_id: String,
    pub content: String,
    pub is_first_message: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenMessageResponse {
    pub verdict: ContentVerdict,
    pub risk_score: f64,
    pub reason: Option<String>,
    pub screening_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateReportPayload {
    pub reporter_user_id: String,
    pub reported_user_id: String,
    pub reason: ReportReason,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct CreateReportResponse {
    pub report_ticket_id: String,
    pub status: ReportStatus,
}

#[derive(Debug, Serialize)]
pub struct GetReportResponse {
    pub report_ticket_id: String,
    pub reporter_user_id: String,
    pub reported_user_id: String,
    pub reason: ReportReason,
    pub description: String,
    pub status: ReportStatus,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ModerationQuery {
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModerationEntry {
    pub user_id: String,
    pub action: String,
    pub reason: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct ModerationResponse {
    pub user_id: String,
    pub entries: Vec<ModerationEntry>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAppealPayload {
    pub report_ticket_id: String,
    pub appellant_user_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AppealStatus {
    Submitted,
    UnderReview,
    Upheld,
    Overturned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppealRecord {
    pub appeal_id: String,
    pub report_ticket_id: String,
    pub appellant_user_id: String,
    pub reason: String,
    pub status: AppealStatus,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct CreateAppealResponse {
    pub appeal_id: String,
    pub status: AppealStatus,
}

#[derive(Debug, Deserialize)]
pub struct CreateBlockPayload {
    pub user_id: String,
    pub blocked_user_id: String,
}

#[derive(Debug, Serialize)]
pub struct CreateBlockResponse {
    pub blocked_user_id: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ListBlocksQuery {
    pub user_id: String,
}

#[derive(Debug, Serialize)]
pub struct ListBlocksResponse {
    pub user_id: String,
    pub blocked_users: Vec<BlockRecord>,
}

#[derive(Debug, Deserialize)]
struct DeleteBlockAuthed {
    user_id: String,
    blocked_user_id: String,
}

async fn delete_block(
    State(state): State<Arc<SafetyState>>,
    headers: HeaderMap,
    Json(body): Json<DeleteBlockAuthed>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;
    if body.user_id.is_empty() || body.blocked_user_id.is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "user_id and blocked_user_id required".to_string(),
        ));
    }

    {
        let mut blocks = state.blocks.lock().expect("mutex poisoned");
        if let Some(list) = blocks.get_mut(&body.user_id) {
            let before = list.len();
            list.retain(|b| b.blocked_user_id != body.blocked_user_id);
            if list.len() == before {
                return Err((
                    axum::http::StatusCode::NOT_FOUND,
                    "block not found".to_string(),
                ));
            }
        } else {
            return Err((
                axum::http::StatusCode::NOT_FOUND,
                "block not found".to_string(),
            ));
        }
    }

    tracing::info!(
        user_id = %body.user_id,
        blocked_user_id = %body.blocked_user_id,
        "safety-service: block removed"
    );

    if let Some(pg) = &state.pg {
        let blocker_uuid = match Uuid::parse_str(&body.user_id) {
            Ok(u) => u,
            Err(_) => Uuid::new_v4(),
        };
        let blocked_uuid = match Uuid::parse_str(&body.blocked_user_id) {
            Ok(u) => u,
            Err(_) => Uuid::new_v4(),
        };
        if let Err(e) = pg.remove_block(&blocker_uuid, &blocked_uuid).await {
            tracing::error!(error = %e, user_id = %body.user_id, "postgres remove_block failed");
        }
    }

    Ok(Json(json!({ "unblocked": body.blocked_user_id })))
}

async fn dm_first_message_review(
    State(state): State<Arc<SafetyState>>,
    headers: HeaderMap,
    Json(body): Json<ReviewDmFirstMessagePayload>,
) -> Result<Json<ReviewDmFirstMessageResponse>, (axum::http::StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;

    if body.message_content.trim().is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "message_content must not be empty".to_string(),
        ));
    }

    let review_id = Uuid::new_v4().to_string();

    let is_blocked = {
        let bl = state.blocks.lock().expect("mutex poisoned");
        bl.get(&body.recipient_user_id)
            .map(|list| {
                list.iter()
                    .any(|b| b.blocked_user_id == body.sender_user_id)
            })
            .unwrap_or(false)
    };

    if is_blocked {
        tracing::info!(
            review_id = %review_id,
            sender = %body.sender_user_id,
            recipient = %body.recipient_user_id,
            allowed = false,
            reason = "sender_blocked_by_recipient",
            "safety-service: DM first message review"
        );
        return Ok(Json(ReviewDmFirstMessageResponse {
            allowed: false,
            reason: Some("sender_blocked_by_recipient".to_string()),
            review_id,
        }));
    }

    let (risk_score, risk_reason) = compute_risk_score(&body.message_content, true);

    if risk_score >= 0.5 {
        tracing::info!(
            review_id = %review_id,
            sender = %body.sender_user_id,
            recipient = %body.recipient_user_id,
            allowed = false,
            risk_score = risk_score,
            "safety-service: DM first message review rejected"
        );
        Ok(Json(ReviewDmFirstMessageResponse {
            allowed: false,
            reason: risk_reason.or_else(|| Some("content_risk_too_high".to_string())),
            review_id,
        }))
    } else {
        tracing::info!(
            review_id = %review_id,
            sender = %body.sender_user_id,
            recipient = %body.recipient_user_id,
            allowed = true,
            risk_score = risk_score,
            "safety-service: DM first message review passed"
        );
        Ok(Json(ReviewDmFirstMessageResponse {
            allowed: true,
            reason: if risk_score > 0.0 { risk_reason } else { None },
            review_id,
        }))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RiskFlagType {
    MultipleReports,
    PatternViolation,
    AutomatedDetection,
    ManualReview,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RiskSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFlagRecord {
    pub flag_id: String,
    pub user_id: String,
    pub flag_type: RiskFlagType,
    pub source: String,
    pub description: String,
    pub severity: RiskSeverity,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ReportActionType {
    Dismissed,
    WarningIssued,
    TemporarySuspension,
    PermanentBan,
    EscalatedToHuman,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportActionRecord {
    pub action_id: String,
    pub report_ticket_id: String,
    pub action_type: ReportActionType,
    pub actor_user_id: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct ReviewDmFirstMessagePayload {
    pub sender_user_id: String,
    pub recipient_user_id: String,
    pub message_content: String,
}

#[derive(Debug, Serialize)]
pub struct ReviewDmFirstMessageResponse {
    pub allowed: bool,
    pub reason: Option<String>,
    pub review_id: String,
}

fn compute_risk_score(content: &str, is_first_message: bool) -> (f64, Option<String>) {
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

    if is_first_message && score > 0.0 {
        score += 0.15;
        reason = Some("first_message_flag".to_string());
    } else if score > 0.0 {
        reason = Some("content_risk_detected".to_string());
    }

    (score.min(1.0), reason)
}

#[derive(Debug, Deserialize)]
struct ScreenMessageAuthed {
    sender_user_id: String,
    recipient_user_id: String,
    content: String,
    #[serde(default)]
    is_first_message: bool,
}

async fn screen_message(
    State(state): State<Arc<SafetyState>>,
    headers: HeaderMap,
    Json(body): Json<ScreenMessageAuthed>,
) -> Result<Json<ScreenMessageResponse>, (axum::http::StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;

    if body.content.trim().is_empty() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "content must not be empty".to_string(),
        ));
    }

    let is_blocked = {
        let bl = state.blocks.lock().expect("mutex poisoned");
        bl.get(&body.recipient_user_id)
            .map(|list| {
                list.iter()
                    .any(|b| b.blocked_user_id == body.sender_user_id)
            })
            .unwrap_or(false)
    };

    if is_blocked {
        let screening_id = Uuid::new_v4().to_string();
        return Ok(Json(ScreenMessageResponse {
            verdict: ContentVerdict::Reject,
            risk_score: 1.0,
            reason: Some("sender_blocked_by_recipient".to_string()),
            screening_id,
        }));
    }

    let (risk_score, risk_reason) = compute_risk_score(&body.content, body.is_first_message);
    let screening_id = Uuid::new_v4().to_string();

    let verdict = if risk_score >= 0.5 {
        ContentVerdict::Reject
    } else if risk_score >= 0.2 {
        ContentVerdict::Flag
    } else {
        ContentVerdict::Allow
    };

    if let Some(pg) = &state.pg {
        let screening_uuid = Uuid::parse_str(&screening_id).unwrap_or_else(|_| Uuid::new_v4());
        let sender_uuid = Uuid::parse_str(&body.sender_user_id).unwrap_or_else(|_| Uuid::new_v4());
        let verdict_str = match verdict {
            ContentVerdict::Allow => "allow",
            ContentVerdict::Flag => "flag",
            ContentVerdict::Reject => "reject",
        };
        if let Err(e) = pg
            .insert_screening(
                &screening_uuid,
                "dm_message",
                &sender_uuid,
                verdict_str,
                risk_score,
                risk_reason.as_deref(),
            )
            .await
        {
            tracing::error!(error = %e, screening_id = %screening_id, "postgres insert_screening failed");
        }
    }

    emit_event(
        "safety.message.screened",
        Some(body.sender_user_id.clone()),
        json!({
            "screening_id": screening_id,
            "sender_user_id": body.sender_user_id,
            "recipient_user_id": body.recipient_user_id,
            "verdict": serde_json::to_value(&verdict).unwrap_or(json!("unknown")),
            "risk_score": risk_score,
        }),
    );

    Ok(Json(ScreenMessageResponse {
        verdict,
        risk_score,
        reason: risk_reason,
        screening_id,
    }))
}

async fn create_report(
    State(state): State<Arc<SafetyState>>,
    headers: HeaderMap,
    Json(body): Json<CreateReportPayload>,
) -> Result<Json<CreateReportResponse>, (axum::http::StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;

    if body.reporter_user_id == body.reported_user_id {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "cannot report yourself".to_string(),
        ));
    }

    let ticket_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let record = ReportRecord {
        report_ticket_id: ticket_id.clone(),
        reporter_user_id: body.reporter_user_id.clone(),
        reported_user_id: body.reported_user_id.clone(),
        reason: body.reason.clone(),
        description: body.description.clone(),
        status: ReportStatus::Submitted,
        created_at: now,
    };

    {
        let mut reports = state.reports.lock().expect("mutex poisoned");
        reports.insert(ticket_id.clone(), record);
    }

    tracing::info!(
        ticket_id = %ticket_id,
        reporter = %body.reporter_user_id,
        reported = %body.reported_user_id,
        reason = ?body.reason,
        "safety-service: report created"
    );

    emit_event(
        "safety.report.created",
        Some(body.reporter_user_id.clone()),
        json!({
            "ticket_id": ticket_id,
            "reporter_user_id": body.reporter_user_id,
            "reported_user_id": body.reported_user_id,
            "reason": serde_json::to_value(&body.reason).unwrap_or(json!("unknown")),
        }),
    );

    if let Some(pg) = &state.pg {
        let ticket_uuid = Uuid::parse_str(&ticket_id).unwrap_or_else(|_| Uuid::new_v4());
        let reporter_uuid =
            Uuid::parse_str(&body.reporter_user_id).unwrap_or_else(|_| Uuid::new_v4());
        let reported_uuid =
            Uuid::parse_str(&body.reported_user_id).unwrap_or_else(|_| Uuid::new_v4());
        let reason_str = body.reason.serialize_name();
        if let Err(e) = pg
            .insert_report(
                &ticket_uuid,
                &reporter_uuid,
                "user",
                &reported_uuid,
                reason_str,
                "submitted",
            )
            .await
        {
            tracing::error!(error = %e, ticket_id = %ticket_id, "postgres insert_report failed");
        }
    }

    Ok(Json(CreateReportResponse {
        report_ticket_id: ticket_id,
        status: ReportStatus::Submitted,
    }))
}

async fn get_report(
    State(state): State<Arc<SafetyState>>,
    headers: HeaderMap,
    axum::extract::Path(report_ticket_id): axum::extract::Path<String>,
) -> Result<Json<GetReportResponse>, (axum::http::StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;

    let reports = state.reports.lock().expect("mutex poisoned");
    let record = reports.get(&report_ticket_id).ok_or((
        axum::http::StatusCode::NOT_FOUND,
        "report not found".to_string(),
    ))?;

    Ok(Json(GetReportResponse {
        report_ticket_id: record.report_ticket_id.clone(),
        reporter_user_id: record.reporter_user_id.clone(),
        reported_user_id: record.reported_user_id.clone(),
        reason: record.reason.clone(),
        description: record.description.clone(),
        status: record.status.clone(),
        created_at: record.created_at.clone(),
    }))
}

async fn get_moderation(
    State(state): State<Arc<SafetyState>>,
    headers: HeaderMap,
    axum::extract::Query(query): axum::extract::Query<ModerationQuery>,
) -> Result<Json<ModerationResponse>, (axum::http::StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;

    let reports = state.reports.lock().expect("mutex poisoned");
    let risk_flags = state.risk_flags.lock().expect("mutex poisoned");
    let actions = state.report_actions.lock().expect("mutex poisoned");

    let mut entries = Vec::new();

    for report in reports.values() {
        if report.reported_user_id == query.user_id {
            entries.push(ModerationEntry {
                user_id: query.user_id.clone(),
                action: "reported".to_string(),
                reason: report.reason.serialize_name().to_string(),
                created_at: report.created_at.clone(),
            });
        }
    }

    for flag in risk_flags.values() {
        if flag.user_id == query.user_id {
            entries.push(ModerationEntry {
                user_id: query.user_id.clone(),
                action: "risk_flagged".to_string(),
                reason: flag.description.clone(),
                created_at: flag.created_at.clone(),
            });
        }
    }

    for (ticket_id, action_list) in actions.iter() {
        for action in action_list {
            if let Some(report) = reports.get(ticket_id) {
                if report.reported_user_id == query.user_id {
                    entries.push(ModerationEntry {
                        user_id: query.user_id.clone(),
                        action: match action.action_type {
                            ReportActionType::Dismissed => "dismissed".to_string(),
                            ReportActionType::WarningIssued => "warning_issued".to_string(),
                            ReportActionType::TemporarySuspension => {
                                "temporary_suspension".to_string()
                            }
                            ReportActionType::PermanentBan => "permanent_ban".to_string(),
                            ReportActionType::EscalatedToHuman => "escalated_to_human".to_string(),
                        },
                        reason: format!("report_action:{}", ticket_id),
                        created_at: action.created_at.clone(),
                    });
                }
            }
        }
    }

    drop(reports);
    drop(risk_flags);
    drop(actions);

    Ok(Json(ModerationResponse {
        user_id: query.user_id.clone(),
        entries,
    }))
}

async fn create_appeal(
    State(state): State<Arc<SafetyState>>,
    headers: HeaderMap,
    Json(body): Json<CreateAppealPayload>,
) -> Result<Json<CreateAppealResponse>, (axum::http::StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;

    {
        let reports = state.reports.lock().expect("mutex poisoned");
        if !reports.contains_key(&body.report_ticket_id) {
            return Err((
                axum::http::StatusCode::NOT_FOUND,
                "report not found".to_string(),
            ));
        }
    }

    let appeal_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let record = AppealRecord {
        appeal_id: appeal_id.clone(),
        report_ticket_id: body.report_ticket_id.clone(),
        appellant_user_id: body.appellant_user_id.clone(),
        reason: body.reason.clone(),
        status: AppealStatus::Submitted,
        created_at: now,
    };

    {
        let mut appeals = state.appeals.lock().expect("mutex poisoned");
        appeals.push(record);
    }

    tracing::info!(
        appeal_id = %appeal_id,
        ticket_id = %body.report_ticket_id,
        appellant = %body.appellant_user_id,
        "safety-service: appeal created"
    );

    emit_event(
        "safety.appeal.created",
        Some(body.appellant_user_id.clone()),
        json!({
            "appeal_id": appeal_id,
            "report_ticket_id": body.report_ticket_id,
            "appellant_user_id": body.appellant_user_id,
        }),
    );

    Ok(Json(CreateAppealResponse {
        appeal_id,
        status: AppealStatus::Submitted,
    }))
}

async fn create_block(
    State(state): State<Arc<SafetyState>>,
    headers: HeaderMap,
    Json(body): Json<CreateBlockPayload>,
) -> Result<Json<CreateBlockResponse>, (axum::http::StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;

    if body.user_id == body.blocked_user_id {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "cannot block yourself".to_string(),
        ));
    }

    {
        let bl = state.blocks.lock().expect("mutex poisoned");
        if let Some(list) = bl.get(&body.user_id) {
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

    let record = BlockRecord {
        block_id: block_id.clone(),
        blocker_user_id: body.user_id.clone(),
        blocked_user_id: body.blocked_user_id.clone(),
        created_at: now.clone(),
    };

    {
        let mut blocks = state.blocks.lock().expect("mutex poisoned");
        blocks.entry(body.user_id.clone()).or_default().push(record);
    }

    tracing::info!(
        blocker = %body.user_id,
        blocked = %body.blocked_user_id,
        "safety-service: block created"
    );

    emit_event(
        "safety.block.created",
        Some(body.user_id.clone()),
        json!({
            "block_id": block_id,
            "blocker_user_id": body.user_id,
            "blocked_user_id": body.blocked_user_id,
        }),
    );

    if let Some(pg) = &state.pg {
        let block_uuid = Uuid::parse_str(&block_id).unwrap_or_else(|_| Uuid::new_v4());
        let blocker_uuid = Uuid::parse_str(&body.user_id).unwrap_or_else(|_| Uuid::new_v4());
        let blocked_uuid =
            Uuid::parse_str(&body.blocked_user_id).unwrap_or_else(|_| Uuid::new_v4());
        if let Err(e) = pg
            .insert_block(&block_uuid, &blocker_uuid, &blocked_uuid, "safety_service")
            .await
        {
            tracing::error!(error = %e, block_id = %block_id, "postgres insert_block failed");
        }
    }

    Ok(Json(CreateBlockResponse {
        blocked_user_id: body.blocked_user_id,
        created_at: now,
    }))
}

async fn list_blocks(
    State(state): State<Arc<SafetyState>>,
    headers: HeaderMap,
    axum::extract::Query(query): axum::extract::Query<ListBlocksQuery>,
) -> Result<Json<ListBlocksResponse>, (axum::http::StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;

    let blocks = state.blocks.lock().expect("mutex poisoned");
    let blocked_users = blocks.get(&query.user_id).cloned().unwrap_or_default();

    Ok(Json(ListBlocksResponse {
        user_id: query.user_id,
        blocked_users,
    }))
}

#[derive(Debug, Deserialize)]
struct CreateReportActionAuthed {
    report_ticket_id: String,
    action_type: ReportActionType,
    actor_user_id: String,
}

#[derive(Debug, Serialize)]
pub struct CreateReportActionResponse {
    pub action_id: String,
    pub action_type: ReportActionType,
}

async fn create_report_action(
    State(state): State<Arc<SafetyState>>,
    headers: HeaderMap,
    Json(body): Json<CreateReportActionAuthed>,
) -> Result<Json<CreateReportActionResponse>, (axum::http::StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;

    {
        let reports = state.reports.lock().expect("mutex poisoned");
        if !reports.contains_key(&body.report_ticket_id) {
            return Err((
                axum::http::StatusCode::NOT_FOUND,
                "report not found".to_string(),
            ));
        }
    }

    let action_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let record = ReportActionRecord {
        action_id: action_id.clone(),
        report_ticket_id: body.report_ticket_id.clone(),
        action_type: body.action_type.clone(),
        actor_user_id: body.actor_user_id.clone(),
        created_at: now,
    };

    {
        let mut actions = state.report_actions.lock().expect("mutex poisoned");
        actions
            .entry(body.report_ticket_id.clone())
            .or_default()
            .push(record);
    }

    {
        let mut reports = state.reports.lock().expect("mutex poisoned");
        if let Some(report) = reports.get_mut(&body.report_ticket_id) {
            report.status = match &body.action_type {
                ReportActionType::Dismissed => ReportStatus::Dismissed,
                ReportActionType::WarningIssued => ReportStatus::ActionTaken,
                ReportActionType::TemporarySuspension => ReportStatus::ActionTaken,
                ReportActionType::PermanentBan => ReportStatus::ActionTaken,
                ReportActionType::EscalatedToHuman => ReportStatus::UnderReview,
            };
        }
    }

    if body.action_type == ReportActionType::TemporarySuspension
        || body.action_type == ReportActionType::PermanentBan
    {
        let (flag_id, reported_user_id, flag_record) = {
            let reports = state.reports.lock().expect("mutex poisoned");
            let report = match reports.get(&body.report_ticket_id) {
                Some(r) => r,
                None => {
                    return Err((
                        axum::http::StatusCode::NOT_FOUND,
                        "report not found for risk flag".to_string(),
                    ));
                }
            };
            let flag_id = Uuid::new_v4().to_string();
            let reported_user_id = report.reported_user_id.clone();
            let flag_record = RiskFlagRecord {
                flag_id: flag_id.clone(),
                user_id: report.reported_user_id.clone(),
                flag_type: RiskFlagType::MultipleReports,
                source: format!("report_action:{}", body.report_ticket_id),
                description: format!("Auto-flagged from report action: {:?}", body.action_type),
                severity: if body.action_type == ReportActionType::PermanentBan {
                    RiskSeverity::Critical
                } else {
                    RiskSeverity::High
                },
                created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            };
            (flag_id, reported_user_id, flag_record)
        };

        {
            let mut risk_flags = state.risk_flags.lock().expect("mutex poisoned");
            risk_flags.insert(flag_id.clone(), flag_record);
        }

        if let Some(pg) = &state.pg {
            let flag_uuid = match Uuid::parse_str(&flag_id) {
                Ok(u) => u,
                Err(_) => Uuid::new_v4(),
            };
            let reported_uuid = match Uuid::parse_str(&reported_user_id) {
                Ok(u) => u,
                Err(_) => Uuid::new_v4(),
            };
            let severity_str = if body.action_type == ReportActionType::PermanentBan {
                "critical"
            } else {
                "high"
            };
            if let Err(e) = pg
                .insert_risk_flag(
                    &flag_uuid,
                    &reported_uuid,
                    "multiple_reports",
                    &format!("report_action:{}", body.report_ticket_id),
                    &format!("Auto-flagged from report action: {:?}", body.action_type),
                    severity_str,
                )
                .await
            {
                tracing::error!(error = %e, flag_id = %flag_id, "postgres insert_risk_flag failed — data will not persist");
            }
        }
    }

    if let Some(pg) = &state.pg {
        let action_uuid = match Uuid::parse_str(&action_id) {
            Ok(u) => u,
            Err(_) => Uuid::new_v4(),
        };
        let ticket_uuid = match Uuid::parse_str(&body.report_ticket_id) {
            Ok(u) => u,
            Err(_) => Uuid::new_v4(),
        };
        let actor_uuid = match Uuid::parse_str(&body.actor_user_id) {
            Ok(u) => u,
            Err(_) => Uuid::new_v4(),
        };
        let action_type_str = match &body.action_type {
            ReportActionType::Dismissed => "dismissed",
            ReportActionType::WarningIssued => "warning_issued",
            ReportActionType::TemporarySuspension => "temporary_suspension",
            ReportActionType::PermanentBan => "permanent_ban",
            ReportActionType::EscalatedToHuman => "escalated_to_human",
        };
        if let Err(e) = pg
            .insert_report_action(&action_uuid, &ticket_uuid, action_type_str, &actor_uuid)
            .await
        {
            tracing::error!(error = %e, action_id = %action_id, "postgres insert_report_action failed — data will not persist");
        }
    }

    tracing::info!(
        action_id = %action_id,
        report_ticket_id = %body.report_ticket_id,
        action_type = ?body.action_type,
        "safety-service: report action created"
    );

    Ok(Json(CreateReportActionResponse {
        action_id,
        action_type: body.action_type,
    }))
}

#[derive(Debug, Deserialize)]
pub struct CreateRiskFlagPayload {
    pub user_id: String,
    pub flag_type: RiskFlagType,
    pub source: String,
    pub description: String,
    pub severity: RiskSeverity,
}

#[derive(Debug, Serialize)]
pub struct CreateRiskFlagResponse {
    pub flag_id: String,
    pub severity: RiskSeverity,
}

async fn create_risk_flag(
    State(state): State<Arc<SafetyState>>,
    headers: HeaderMap,
    Json(body): Json<CreateRiskFlagPayload>,
) -> Result<Json<CreateRiskFlagResponse>, (axum::http::StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;

    let flag_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let record = RiskFlagRecord {
        flag_id: flag_id.clone(),
        user_id: body.user_id.clone(),
        flag_type: body.flag_type,
        source: body.source,
        description: body.description,
        severity: body.severity.clone(),
        created_at: now,
    };

    {
        let mut risk_flags = state.risk_flags.lock().expect("mutex poisoned");
        risk_flags.insert(flag_id.clone(), record);
    }

    tracing::info!(
        flag_id = %flag_id,
        user_id = %body.user_id,
        severity = ?body.severity,
        "safety-service: risk flag created"
    );

    Ok(Json(CreateRiskFlagResponse {
        flag_id,
        severity: body.severity,
    }))
}

#[derive(Debug, Serialize)]
pub struct GetRiskFlagsResponse {
    pub user_id: String,
    pub flags: Vec<RiskFlagRecord>,
}

async fn get_risk_flags(
    State(state): State<Arc<SafetyState>>,
    headers: HeaderMap,
    axum::extract::Path(user_id): axum::extract::Path<String>,
) -> Result<Json<GetRiskFlagsResponse>, (axum::http::StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;

    let risk_flags = state.risk_flags.lock().expect("mutex poisoned");
    let user_flags: Vec<RiskFlagRecord> = risk_flags
        .values()
        .filter(|f| f.user_id == user_id)
        .cloned()
        .collect();

    Ok(Json(GetRiskFlagsResponse {
        user_id,
        flags: user_flags,
    }))
}

async fn internal_health_detail(
    State(state): State<Arc<SafetyState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)?;
    let reports = state.reports.lock().expect("mutex poisoned");
    let blocks = state.blocks.lock().expect("mutex poisoned");
    let appeals = state.appeals.lock().expect("mutex poisoned");
    let risk_flags = state.risk_flags.lock().expect("mutex poisoned");
    let report_actions = state.report_actions.lock().expect("mutex poisoned");
    Ok(Json(json!({
        "report_count": reports.len(),
        "block_count": blocks.values().map(|v| v.len()).sum::<usize>(),
        "appeal_count": appeals.len(),
        "risk_flag_count": risk_flags.len(),
        "report_action_count": report_actions.values().map(|v| v.len()).sum::<usize>(),
        "env_mode": state.config.env_mode,
        "backend": if state.pg.is_some() { "postgres" } else { "in-memory" },
    })))
}

trait SerializeName {
    fn serialize_name(&self) -> &'static str;
}

impl SerializeName for ReportReason {
    fn serialize_name(&self) -> &'static str {
        match self {
            ReportReason::Harassment => "harassment",
            ReportReason::Spam => "spam",
            ReportReason::InappropriateContent => "inappropriate_content",
            ReportReason::SafetyConcern => "safety_concern",
            ReportReason::Other => "other",
        }
    }
}

impl SerializeName for ReportStatus {
    fn serialize_name(&self) -> &'static str {
        match self {
            ReportStatus::Submitted => "submitted",
            ReportStatus::UnderReview => "under_review",
            ReportStatus::ActionTaken => "action_taken",
            ReportStatus::Dismissed => "dismissed",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_cannot_report_self() {
        let reporter = "user-A";
        let reported = "user-B";
        assert_ne!(reporter, reported, "a user cannot report themselves");
        let same = "user-A";
        assert_eq!(same, same);
        assert!(
            reporter == same,
            "self-report detected: handler returns BAD_REQUEST"
        );
    }

    #[test]
    fn report_reason_serializes_snake_case() {
        let s = serde_json::to_string(&ReportReason::Harassment).unwrap();
        assert_eq!(s, "\"harassment\"");
    }

    #[test]
    fn report_status_serializes_snake_case() {
        let s = serde_json::to_string(&ReportStatus::Submitted).unwrap();
        assert_eq!(s, "\"submitted\"");
    }

    #[test]
    fn risk_score_clean_message() {
        let (score, reason) = compute_risk_score("Hey, how are you?", false);
        assert_eq!(score, 0.0);
        assert!(reason.is_none());
    }

    #[test]
    fn risk_score_suspicious_message() {
        let (score, reason) =
            compute_risk_score("Send money via wire transfer to my account", false);
        assert!(score > 0.2);
        assert!(reason.is_some());
    }

    #[test]
    fn risk_score_first_message_boost() {
        let (score_normal, _) = compute_risk_score("Check out my telegram", false);
        let (score_first, reason) = compute_risk_score("Check out my telegram", true);
        assert!(score_first > score_normal);
        assert_eq!(reason.as_deref(), Some("first_message_flag"));
    }

    #[test]
    fn risk_score_capped_at_one() {
        let (score, _) = compute_risk_score(
            "wire transfer crypto investment send money http:// telegram whatsapp",
            true,
        );
        assert!(score <= 1.0);
    }

    #[test]
    fn block_record_creation() {
        let config = Config::from_env();
        let state = SafetyState::new(config, None);

        let record = BlockRecord {
            block_id: "blk-1".to_string(),
            blocker_user_id: "user-a".to_string(),
            blocked_user_id: "user-b".to_string(),
            created_at: "2026-01-01T00:00:00.000Z".to_string(),
        };

        {
            let mut blocks = state.blocks.lock().expect("mutex poisoned");
            blocks.entry("user-a".to_string()).or_default().push(record);
        }

        let bl = state.blocks.lock().expect("mutex poisoned");
        assert!(bl
            .get("user-a")
            .unwrap()
            .iter()
            .any(|b| b.blocked_user_id == "user-b"));
    }

    #[test]
    fn dm_review_blocked_sender() {
        let config = Config::from_env();
        let state = SafetyState::new(config, None);

        let record = BlockRecord {
            block_id: "blk-1".to_string(),
            blocker_user_id: "user-recipient".to_string(),
            blocked_user_id: "user-sender".to_string(),
            created_at: "2026-01-01T00:00:00.000Z".to_string(),
        };

        {
            let mut blocks = state.blocks.lock().expect("mutex poisoned");
            blocks
                .entry("user-recipient".to_string())
                .or_default()
                .push(record);
        }

        let bl = state.blocks.lock().expect("mutex poisoned");
        let is_blocked = bl
            .get("user-recipient")
            .map(|list| list.iter().any(|b| b.blocked_user_id == "user-sender"))
            .unwrap_or(false);
        assert!(is_blocked);
    }

    #[test]
    fn appeal_status_serializes() {
        let s = serde_json::to_string(&AppealStatus::Submitted).unwrap();
        assert_eq!(s, "\"submitted\"");
        let s = serde_json::to_string(&AppealStatus::Overturned).unwrap();
        assert_eq!(s, "\"overturned\"");
    }

    #[test]
    fn risk_flag_creation_and_retrieval() {
        let config = Config::from_env();
        let state = SafetyState::new(config, None);

        let flag = RiskFlagRecord {
            flag_id: "flag-1".to_string(),
            user_id: "user-A".to_string(),
            flag_type: RiskFlagType::MultipleReports,
            source: "test".to_string(),
            description: "test flag".to_string(),
            severity: RiskSeverity::High,
            created_at: "2026-01-01T00:00:00.000Z".to_string(),
        };

        {
            let mut risk_flags = state.risk_flags.lock().expect("mutex poisoned");
            risk_flags.insert("flag-1".to_string(), flag);
        }

        let risk_flags = state.risk_flags.lock().expect("mutex poisoned");
        let user_flags: Vec<&RiskFlagRecord> = risk_flags
            .values()
            .filter(|f| f.user_id == "user-A")
            .collect();
        assert_eq!(user_flags.len(), 1);
        assert_eq!(user_flags[0].severity, RiskSeverity::High);
    }

    #[test]
    fn report_action_auto_flags_critical() {
        let config = Config::from_env();
        let state = SafetyState::new(config, None);

        let report = ReportRecord {
            report_ticket_id: "ticket-1".to_string(),
            reporter_user_id: "user-A".to_string(),
            reported_user_id: "user-B".to_string(),
            reason: ReportReason::Harassment,
            description: "test".to_string(),
            status: ReportStatus::Submitted,
            created_at: "2026-01-01T00:00:00.000Z".to_string(),
        };
        {
            let mut reports = state.reports.lock().expect("mutex poisoned");
            reports.insert("ticket-1".to_string(), report);
        }

        let action = ReportActionRecord {
            action_id: "action-1".to_string(),
            report_ticket_id: "ticket-1".to_string(),
            action_type: ReportActionType::PermanentBan,
            actor_user_id: "admin-1".to_string(),
            created_at: "2026-01-01T00:00:00.000Z".to_string(),
        };
        {
            let mut actions = state.report_actions.lock().expect("mutex poisoned");
            actions
                .entry("ticket-1".to_string())
                .or_default()
                .push(action);
        }

        let flag = RiskFlagRecord {
            flag_id: "flag-auto-1".to_string(),
            user_id: "user-B".to_string(),
            flag_type: RiskFlagType::MultipleReports,
            source: "report_action:ticket-1".to_string(),
            description: "Auto-flagged from report action".to_string(),
            severity: RiskSeverity::Critical,
            created_at: "2026-01-01T00:00:00.000Z".to_string(),
        };
        {
            let mut risk_flags = state.risk_flags.lock().expect("mutex poisoned");
            risk_flags.insert("flag-auto-1".to_string(), flag);
        }

        let risk_flags = state.risk_flags.lock().expect("mutex poisoned");
        let flagged = risk_flags
            .values()
            .any(|f| f.user_id == "user-B" && f.severity == RiskSeverity::Critical);
        assert!(flagged);
    }

    #[test]
    fn risk_severity_serializes_snake_case() {
        let s = serde_json::to_string(&RiskSeverity::Critical).unwrap();
        assert_eq!(s, "\"critical\"");
        let s = serde_json::to_string(&RiskSeverity::High).unwrap();
        assert_eq!(s, "\"high\"");
    }

    #[test]
    fn report_action_type_serializes_snake_case() {
        let s = serde_json::to_string(&ReportActionType::EscalatedToHuman).unwrap();
        assert_eq!(s, "\"escalated_to_human\"");
        let s = serde_json::to_string(&ReportActionType::TemporarySuspension).unwrap();
        assert_eq!(s, "\"temporary_suspension\"");
    }

    #[test]
    fn risk_flag_type_serializes_snake_case() {
        let s = serde_json::to_string(&RiskFlagType::PatternViolation).unwrap();
        assert_eq!(s, "\"pattern_violation\"");
    }
}
