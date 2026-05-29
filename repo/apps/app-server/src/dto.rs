use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AuthFlowState {
    Idle,
    Submitting,
    Success,
    RequiresVerification,
    ExpiredSession,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    pub phone: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub user_id: Uuid,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
    pub first_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub phone: String,
    pub nickname: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub user_id: Uuid,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
    pub first_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeSummaryResponse {
    pub user_id: Uuid,
    pub nickname: String,
    pub avatar_url: Option<String>,
    pub first_run: bool,
    pub locale: String,
    pub region: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LumiChatState {
    ConversationBootstrapping,
    ReadyForFirstMessage,
    AwaitingReply,
    ReplyStreaming,
    ReplyComplete,
    BridgeDegraded,
    ModelFallback,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub conversation_id: Uuid,
    pub title: String,
    pub last_message_preview: Option<String>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub unread_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationListResponse {
    pub conversations: Vec<ConversationSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageDto {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub role: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub trace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub session_id: Option<Uuid>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub session_id: Uuid,
    pub reply: ChatMessageDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProfileConfirmationState {
    NotReady,
    PendingConfirmation,
    Accepted,
    Rejected,
    Snoozed,
    Edited,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileFactDto {
    pub fact_id: Uuid,
    pub fact_text: String,
    pub source: String,
    pub confidence: f64,
    pub state: ProfileConfirmationState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfirmationListResponse {
    pub pending_facts: Vec<ProfileFactDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileConfirmationAction {
    Accept,
    Reject,
    Snooze,
    Edit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfirmationActionRequest {
    pub action: ProfileConfirmationAction,
    pub feedback_payload: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfirmationActionResponse {
    pub fact_id: Uuid,
    pub new_state: ProfileConfirmationState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileDto {
    pub user_id: Uuid,
    pub nickname: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub age: Option<u8>,
    pub location: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppScreen {
    pub screen_id: String,
    pub title: String,
    pub route: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsDto {
    pub notifications_enabled: bool,
    pub language: String,
    pub theme: String,
    pub locale: String,
    pub region: String,
    pub timezone: String,
    pub content_language: String,
    pub notification_language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsSummaryResponse {
    pub settings: SettingsDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsUpdateRequest {
    pub locale: Option<String>,
    pub region: Option<String>,
    pub timezone: Option<String>,
    pub content_language: Option<String>,
    pub notification_language: Option<String>,
    pub notifications_enabled: Option<bool>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AppErrorCode {
    AuthError,
    NetworkError,
    ValidationError,
    RateLimited,
    SafetyBlocked,
    TemporarilyUnavailable,
    Retryable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppErrorBody {
    pub code: AppErrorCode,
    pub message_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub localized_message: Option<String>,
    pub trace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootState {
    pub state: BootPhase,
    pub has_session: bool,
    pub user: Option<MeSummaryResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BootPhase {
    Booting,
    RestoringSession,
    SessionRestored,
    NoSession,
    BootFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsEvent {
    pub event_name: String,
    pub screen_id: Option<String>,
    pub payload: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FindRequestState {
    Draft,
    Submitting,
    Submitted,
    ClarificationNeeded,
    Processing,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindRequestCreateRequest {
    pub intent_text: String,
    pub preferred_region: Option<String>,
    pub preferred_locale: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindRequestResponse {
    pub request_id: Uuid,
    pub state: FindRequestState,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindRequestDetailResponse {
    pub request_id: Uuid,
    pub state: FindRequestState,
    pub intent_text: String,
    pub clarification_questions: Vec<ClarificationQuestion>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarificationQuestion {
    pub question_id: Uuid,
    pub question_text: String,
    pub answer_options: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarificationAnswerRequest {
    pub answers: Vec<ClarificationAnswer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarificationAnswer {
    pub question_id: Uuid,
    pub answer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarificationAnswerResponse {
    pub request_id: Uuid,
    pub state: FindRequestState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationListState {
    WaitingResults,
    ResultsReady,
    EmptyResult,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationListResponse {
    pub request_id: Uuid,
    pub state: RecommendationListState,
    pub recommendations: Vec<RecommendationSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationSummary {
    pub recommendation_id: Uuid,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub match_score: f64,
    pub explanation_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationDetailResponse {
    pub recommendation_id: Uuid,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub match_score: f64,
    pub explanation: ExplanationDto,
    pub connection_eligible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplanationDto {
    pub version: String,
    pub summary: String,
    pub factors: Vec<ExplanationFactor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplanationFactor {
    pub factor_name: String,
    pub weight: f64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationFeedbackType {
    Like,
    Skip,
    Later,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationFeedbackRequest {
    pub feedback_type: RecommendationFeedbackType,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationFeedbackResponse {
    pub recommendation_id: Uuid,
    pub feedback_type: RecommendationFeedbackType,
    pub recorded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsBatchRequest {
    pub events: Vec<AnalyticsEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DmFirstMessageState {
    Draft,
    UnderReview,
    Approved,
    Blocked,
    Sent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmDraftRequest {
    pub recommendation_id: Uuid,
    pub initial_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmDraftResponse {
    pub thread_id: Uuid,
    pub state: DmFirstMessageState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmFirstMessageSubmitRequest {
    pub thread_id: Uuid,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmFirstMessageSubmitResponse {
    pub thread_id: Uuid,
    pub state: DmFirstMessageState,
    pub safety_decision: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmThreadMessage {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub sender_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmThreadDetailResponse {
    pub thread_id: Uuid,
    pub other_user_id: Uuid,
    pub other_user_name: String,
    pub messages: Vec<DmThreadMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ReportCategory {
    Harassment,
    Spam,
    InappropriateContent,
    PrivacyViolation,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSubmitRequest {
    pub target_user_id: Uuid,
    pub category: ReportCategory,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSubmitResponse {
    pub report_id: Uuid,
    pub category: ReportCategory,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockApplyRequest {
    pub target_user_id: Uuid,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockApplyResponse {
    pub blocked_user_id: Uuid,
    pub blocked_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AppealStatus {
    Pending,
    UnderReview,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppealStatusResponse {
    pub appeal_id: Uuid,
    pub status: AppealStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocaleRegistryResponse {
    pub available_locales: Vec<String>,
    pub available_regions: Vec<String>,
    pub available_timezones: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceActionType {
    Export,
    Delete,
    Correction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceSummaryResponse {
    pub user_id: Uuid,
    pub data_export_available: bool,
    pub data_delete_available: bool,
    pub data_correction_available: bool,
    pub pending_requests: Vec<ComplianceRequestSummary>,
    #[serde(default)]
    pub profile_facts: Vec<serde_json::Value>,
    #[serde(default)]
    pub memory_summaries: Vec<serde_json::Value>,
    #[serde(default)]
    pub key_artifacts: Vec<serde_json::Value>,
    #[serde(default)]
    pub settings: Option<serde_json::Value>,
    #[serde(default)]
    pub consent_records: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRequestSummary {
    pub request_id: Uuid,
    pub action_type: ComplianceActionType,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceActionRequest {
    pub action_type: ComplianceActionType,
    pub scope: Option<String>,
    pub field_name: Option<String>,
    pub export_format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AdminReportStatus {
    Pending,
    UnderReview,
    Resolved,
    Dismissed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AdminActionType {
    Warn,
    Ban,
    Dismiss,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminReportQueueItem {
    pub report_id: Uuid,
    pub reporter_id: Uuid,
    pub target_user_id: Uuid,
    pub category: ReportCategory,
    pub status: AdminReportStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminReportQueueResponse {
    pub reports: Vec<AdminReportQueueItem>,
    pub total_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminReportDetailResponse {
    pub report_id: Uuid,
    pub reporter_id: Uuid,
    pub target_user_id: Uuid,
    pub category: ReportCategory,
    pub description: Option<String>,
    pub status: AdminReportStatus,
    pub created_at: DateTime<Utc>,
    pub appeal: Option<AdminAppealSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminAppealSummary {
    pub appeal_id: Uuid,
    pub status: AppealStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminAppealQueueItem {
    pub appeal_id: Uuid,
    pub report_id: Uuid,
    pub appellant_id: Uuid,
    pub status: AppealStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminAppealQueueResponse {
    pub appeals: Vec<AdminAppealQueueItem>,
    pub total_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminActionRequest {
    pub action_type: AdminActionType,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminActionResponse {
    pub report_id: Uuid,
    pub action_type: AdminActionType,
    pub result: String,
    pub acted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceActionResponse {
    pub request_id: Uuid,
    pub action_type: ComplianceActionType,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionGateCheckResponse {
    pub region: String,
    pub allowed: bool,
    pub reason: Option<String>,
    pub degraded: bool,
    pub fallback_region: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionResidencyPolicy {
    pub version: String,
    pub updated_at: String,
    pub user_region_determination: RegionPolicySection,
    pub data_residency: RegionPolicySection,
    pub model_call_region: RegionPolicySection,
    pub log_storage_region: RegionPolicySection,
    pub cross_border_basis: RegionPolicySection,
    pub region_failure_degradation: RegionPolicySection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionPolicySection {
    pub summary_en: String,
    pub summary_zh: String,
    pub details: Vec<RegionPolicyDetail>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionPolicyDetail {
    pub region: String,
    pub policy_en: String,
    pub policy_zh: String,
}
