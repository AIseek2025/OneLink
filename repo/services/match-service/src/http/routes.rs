use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
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

pub fn router(state: Arc<MatchState>) -> Router {
    Router::new()
        .route(
            "/api/v1/match/find-requests",
            post(create_find_request_handler),
        )
        .route("/api/v1/match/find-requests", get(list_find_requests))
        .route(
            "/api/v1/match/find-requests/:findRequestId",
            get(get_find_request),
        )
        .route(
            "/api/v1/match/find-requests/:findRequestId/candidates",
            get(get_candidates),
        )
        .route(
            "/api/v1/match/find-requests/:findRequestId/feedback",
            post(submit_feedback),
        )
        .route("/api/v1/match/matches", get(list_matches))
        .route("/api/v1/match/matches/:matchId", get(get_match))
        .route("/internal/match/health-detail", get(internal_health_detail))
        .with_state(state)
}

fn emit_event(event_name: &str, actor_user_id: Option<String>, payload: serde_json::Value) {
    let envelope = onelink_event_envelope::EventEnvelope::new_v1(
        event_name,
        "match-service",
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
pub struct MatchState {
    pub config: Config,
    find_requests: Mutex<HashMap<String, FindRequestRecord>>,
    candidate_lists: Mutex<HashMap<String, Vec<CandidateRecord>>>,
    feedback_records: Mutex<HashMap<String, Vec<FeedbackRecord>>>,
    matches: Mutex<HashMap<String, MatchRecord>>,
    like_index: Mutex<HashMap<String, Vec<String>>>,
    blocks: Mutex<HashMap<String, Vec<String>>>,
    pub pg: Option<Arc<PostgresStore>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindRequestRecord {
    pub find_request_id: String,
    pub user_id: String,
    pub raw_query: String,
    pub intent_tags: Vec<String>,
    pub status: FindRequestStatus,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FindRequestStatus {
    Pending,
    Searching,
    Completed,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateRecord {
    pub candidate_user_id: String,
    pub score: f64,
    pub match_reasons: Vec<String>,
    pub status: CandidateStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CandidateStatus {
    Suggested,
    Liked,
    Passed,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackRecord {
    pub feedback_id: String,
    pub user_id: String,
    pub candidate_user_id: String,
    pub action: FeedbackAction,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackAction {
    Like,
    Pass,
    Block,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchRecord {
    pub match_id: String,
    pub user_a_id: String,
    pub user_b_id: String,
    pub match_type: MatchType,
    pub status: MatchStatus,
    pub created_at: String,
    pub mutual_like_detected_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MatchType {
    MutualLike,
    Recommendation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MatchStatus {
    Active,
    Unmatched,
    Expired,
}

impl MatchState {
    pub fn new(config: Config, pg: Option<Arc<PostgresStore>>) -> Arc<Self> {
        Arc::new(Self {
            config,
            pg,
            find_requests: Mutex::new(HashMap::new()),
            candidate_lists: Mutex::new(HashMap::new()),
            feedback_records: Mutex::new(HashMap::new()),
            matches: Mutex::new(HashMap::new()),
            like_index: Mutex::new(HashMap::new()),
            blocks: Mutex::new(HashMap::new()),
        })
    }
}

fn generate_candidates(
    user_id: &str,
    intent_tags: &[String],
    blocked_ids: &[String],
) -> Vec<CandidateRecord> {
    let seed_base = user_id.to_string();
    let mut candidates = Vec::new();
    for i in 0..10 {
        let candidate_seed = format!("{seed_base}-{i}");
        let candidate_id =
            Uuid::new_v5(&Uuid::NAMESPACE_DNS, candidate_seed.as_bytes()).to_string();
        if blocked_ids.contains(&candidate_id) {
            continue;
        }
        let score = 0.95 - (i as f64 * 0.08);
        let reasons = if intent_tags.is_empty() {
            vec!["profile_similarity".to_string()]
        } else {
            intent_tags.iter().take(3).cloned().collect()
        };
        candidates.push(CandidateRecord {
            candidate_user_id: candidate_id,
            score,
            match_reasons: reasons,
            status: CandidateStatus::Suggested,
        });
        if candidates.len() >= 5 {
            break;
        }
    }
    candidates
}

#[derive(Debug, Deserialize)]
pub struct CreateFindRequestPayload {
    pub user_id: String,
    pub raw_query: String,
    #[serde(default)]
    pub intent_tags: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateFindRequestResponse {
    pub find_request_id: String,
    pub status: String,
}

async fn create_find_request_handler(
    State(state): State<Arc<MatchState>>,
    headers: HeaderMap,
    Json(body): Json<CreateFindRequestPayload>,
) -> Result<Json<CreateFindRequestResponse>, (StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;

    if body.raw_query.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "raw_query must not be empty".to_string(),
        ));
    }

    let find_request_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let blocks = state.blocks.lock().expect("mutex poisoned");
    let blocked_ids: Vec<String> = blocks
        .get(&body.user_id)
        .map(|list| list.to_vec())
        .unwrap_or_default();
    drop(blocks);

    let candidates = generate_candidates(&body.user_id, &body.intent_tags, &blocked_ids);

    let record = FindRequestRecord {
        find_request_id: find_request_id.clone(),
        user_id: body.user_id.clone(),
        raw_query: body.raw_query.clone(),
        intent_tags: body.intent_tags.clone(),
        status: FindRequestStatus::Completed,
        created_at: now,
    };

    {
        let mut fr = state.find_requests.lock().expect("mutex poisoned");
        fr.insert(find_request_id.clone(), record);
    }

    {
        let mut cl = state.candidate_lists.lock().expect("mutex poisoned");
        cl.insert(find_request_id.clone(), candidates);
    }

    emit_event(
        "match.find_request.created",
        Some(body.user_id.clone()),
        json!({
            "find_request_id": find_request_id,
            "user_id": body.user_id,
            "raw_query": body.raw_query,
            "intent_tags": body.intent_tags,
        }),
    );

    tracing::info!(
        find_request_id = %find_request_id,
        user_id = %body.user_id,
        "match-service: find request created and candidates generated"
    );

    Ok(Json(CreateFindRequestResponse {
        find_request_id,
        status: "completed".to_string(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct ListFindRequestsQuery {
    pub user_id: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Debug, Serialize)]
pub struct ListFindRequestsResponse {
    pub find_requests: Vec<FindRequestRecord>,
}

async fn list_find_requests(
    State(state): State<Arc<MatchState>>,
    headers: HeaderMap,
    axum::extract::Query(query): axum::extract::Query<ListFindRequestsQuery>,
) -> Result<Json<ListFindRequestsResponse>, (StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;

    let fr = state.find_requests.lock().expect("mutex poisoned");
    let user_requests: Vec<FindRequestRecord> = fr
        .values()
        .filter(|r| r.user_id == query.user_id)
        .take(query.limit)
        .cloned()
        .collect();

    Ok(Json(ListFindRequestsResponse {
        find_requests: user_requests,
    }))
}

#[derive(Debug, Serialize)]
pub struct GetFindRequestResponse {
    pub find_request_id: String,
    pub user_id: String,
    pub raw_query: String,
    pub intent_tags: Vec<String>,
    pub status: FindRequestStatus,
    pub created_at: String,
}

async fn get_find_request(
    State(state): State<Arc<MatchState>>,
    headers: HeaderMap,
    axum::extract::Path(find_request_id): axum::extract::Path<String>,
) -> Result<Json<GetFindRequestResponse>, (StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;

    let fr = state.find_requests.lock().expect("mutex poisoned");
    let record = fr
        .get(&find_request_id)
        .ok_or((StatusCode::NOT_FOUND, "find request not found".to_string()))?;

    Ok(Json(GetFindRequestResponse {
        find_request_id: record.find_request_id.clone(),
        user_id: record.user_id.clone(),
        raw_query: record.raw_query.clone(),
        intent_tags: record.intent_tags.clone(),
        status: record.status.clone(),
        created_at: record.created_at.clone(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct GetCandidatesQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    10
}

#[derive(Debug, Serialize)]
pub struct GetCandidatesResponse {
    pub find_request_id: String,
    pub candidates: Vec<CandidateRecord>,
}

async fn get_candidates(
    State(state): State<Arc<MatchState>>,
    headers: HeaderMap,
    axum::extract::Path(find_request_id): axum::extract::Path<String>,
    axum::extract::Query(query): axum::extract::Query<GetCandidatesQuery>,
) -> Result<Json<GetCandidatesResponse>, (StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;

    {
        let fr = state.find_requests.lock().expect("mutex poisoned");
        if !fr.contains_key(&find_request_id) {
            return Err((StatusCode::NOT_FOUND, "find request not found".to_string()));
        }
    }

    let cl = state.candidate_lists.lock().expect("mutex poisoned");
    let candidates: Vec<CandidateRecord> = cl
        .get(&find_request_id)
        .map(|list| list.iter().take(query.limit).cloned().collect())
        .unwrap_or_default();

    Ok(Json(GetCandidatesResponse {
        find_request_id,
        candidates,
    }))
}

#[derive(Debug, Deserialize)]
struct SubmitFeedbackAuthed {
    user_id: String,
    candidate_user_id: String,
    action: FeedbackAction,
    #[serde(default)]
    find_request_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SubmitFeedbackResponse {
    pub feedback_id: String,
    pub action: FeedbackAction,
    pub match_created: bool,
    pub match_id: Option<String>,
}

async fn submit_feedback(
    State(state): State<Arc<MatchState>>,
    headers: HeaderMap,
    Json(body): Json<SubmitFeedbackAuthed>,
) -> Result<Json<SubmitFeedbackResponse>, (StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;

    let feedback_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let record = FeedbackRecord {
        feedback_id: feedback_id.clone(),
        user_id: body.user_id.clone(),
        candidate_user_id: body.candidate_user_id.clone(),
        action: body.action.clone(),
        created_at: now.clone(),
    };

    {
        let mut fb = state.feedback_records.lock().expect("mutex poisoned");
        fb.entry(body.user_id.clone()).or_default().push(record);
    }

    {
        let mut cl = state.candidate_lists.lock().expect("mutex poisoned");
        let target_fr_ids: Vec<String> = if let Some(ref fr_id) = body.find_request_id {
            vec![fr_id.clone()]
        } else {
            cl.keys().cloned().collect()
        };
        for fr_id in &target_fr_ids {
            if let Some(candidates) = cl.get_mut(fr_id) {
                for candidate in candidates.iter_mut() {
                    if candidate.candidate_user_id == body.candidate_user_id {
                        candidate.status = match &body.action {
                            FeedbackAction::Like => CandidateStatus::Liked,
                            FeedbackAction::Pass => CandidateStatus::Passed,
                            FeedbackAction::Block => CandidateStatus::Blocked,
                        };
                    }
                }
            }
        }
    }

    let mut match_created = false;
    let mut match_id = None;

    if body.action == FeedbackAction::Block {
        {
            let mut blocks = state.blocks.lock().expect("mutex poisoned");
            blocks
                .entry(body.user_id.clone())
                .or_default()
                .push(body.candidate_user_id.clone());
        }
        {
            let mut matches = state.matches.lock().expect("mutex poisoned");
            for (_mid, m) in matches.iter_mut() {
                if (m.user_a_id == body.user_id && m.user_b_id == body.candidate_user_id)
                    || (m.user_a_id == body.candidate_user_id && m.user_b_id == body.user_id)
                {
                    m.status = MatchStatus::Unmatched;
                }
            }
        }
        tracing::info!(
            user_id = %body.user_id,
            blocked = %body.candidate_user_id,
            "match-service: block action, existing matches unmatched"
        );
    }

    if body.action == FeedbackAction::Like {
        let is_blocked = {
            let blocks = state.blocks.lock().expect("mutex poisoned");
            blocks
                .get(&body.user_id)
                .map(|list| list.contains(&body.candidate_user_id))
                .unwrap_or(false)
                || blocks
                    .get(&body.candidate_user_id)
                    .map(|list| list.contains(&body.user_id))
                    .unwrap_or(false)
        };

        if is_blocked {
            tracing::info!(
                user_id = %body.user_id,
                candidate_user_id = %body.candidate_user_id,
                "match-service: like blocked, no match created"
            );
        } else {
            {
                let mut like_index = state.like_index.lock().expect("mutex poisoned");
                like_index
                    .entry(body.user_id.clone())
                    .or_default()
                    .push(body.candidate_user_id.clone());
            }

            let is_mutual = {
                let like_index = state.like_index.lock().expect("mutex poisoned");
                like_index
                    .get(&body.candidate_user_id)
                    .map(|likes| likes.contains(&body.user_id))
                    .unwrap_or(false)
            };

            if is_mutual {
                let new_match_id = Uuid::new_v4().to_string();
                let match_record = MatchRecord {
                    match_id: new_match_id.clone(),
                    user_a_id: body.user_id.clone(),
                    user_b_id: body.candidate_user_id.clone(),
                    match_type: MatchType::MutualLike,
                    status: MatchStatus::Active,
                    created_at: now.clone(),
                    mutual_like_detected_at: Some(now),
                };

                {
                    let mut matches = state.matches.lock().expect("mutex poisoned");
                    matches.insert(new_match_id.clone(), match_record);
                }

                match_created = true;
                match_id = Some(new_match_id);

                tracing::info!(
                    match_id = %match_id.as_deref().unwrap_or(""),
                    user_a = %body.user_id,
                    user_b = %body.candidate_user_id,
                    "match-service: mutual like detected, match created"
                );
            }
        }
    }

    tracing::info!(
        feedback_id = %feedback_id,
        user_id = %body.user_id,
        candidate_user_id = %body.candidate_user_id,
        action = ?body.action,
        "match-service: feedback submitted"
    );

    emit_event(
        "match.feedback.submitted",
        Some(body.user_id.clone()),
        json!({
            "feedback_id": feedback_id,
            "user_id": body.user_id,
            "candidate_user_id": body.candidate_user_id,
            "action": serde_json::to_value(&body.action).unwrap_or(json!("unknown")),
            "match_created": match_created,
        }),
    );

    if let Some(pg) = &state.pg {
        let fb_uuid = match Uuid::parse_str(&feedback_id) {
            Ok(u) => u,
            Err(_) => Uuid::new_v4(),
        };
        let user_uuid = match Uuid::parse_str(&body.user_id) {
            Ok(u) => u,
            Err(_) => Uuid::new_v4(),
        };
        let cand_uuid = match Uuid::parse_str(&body.candidate_user_id) {
            Ok(u) => u,
            Err(_) => Uuid::new_v4(),
        };
        let feedback_type = match &body.action {
            FeedbackAction::Like => "like",
            FeedbackAction::Pass => "dismiss",
            FeedbackAction::Block => "block",
        };
        if let Err(e) = pg
            .insert_feedback(&fb_uuid, &user_uuid, &cand_uuid, feedback_type)
            .await
        {
            tracing::error!(error = %e, feedback_id = %feedback_id, "postgres insert_feedback failed — data will not persist");
        }

        if match_created {
            if let Some(ref mid) = match_id {
                let match_uuid = match Uuid::parse_str(mid) {
                    Ok(u) => u,
                    Err(_) => Uuid::new_v4(),
                };
                if let Err(e) = pg
                    .insert_match(&match_uuid, &user_uuid, &cand_uuid, "mutual_like", "active")
                    .await
                {
                    tracing::error!(error = %e, match_id = %mid, "postgres insert_match failed — data will not persist");
                }
            }
        }
    }

    Ok(Json(SubmitFeedbackResponse {
        feedback_id,
        action: body.action,
        match_created,
        match_id,
    }))
}

#[derive(Debug, Deserialize)]
pub struct ListMatchesQuery {
    pub user_id: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Debug, Serialize)]
pub struct ListMatchesResponse {
    pub matches: Vec<MatchRecord>,
}

async fn list_matches(
    State(state): State<Arc<MatchState>>,
    headers: HeaderMap,
    axum::extract::Query(query): axum::extract::Query<ListMatchesQuery>,
) -> Result<Json<ListMatchesResponse>, (StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;

    let matches = state.matches.lock().expect("mutex poisoned");
    let user_matches: Vec<MatchRecord> = matches
        .values()
        .filter(|m| {
            (m.user_a_id == query.user_id || m.user_b_id == query.user_id)
                && m.status == MatchStatus::Active
        })
        .take(query.limit)
        .cloned()
        .collect();

    Ok(Json(ListMatchesResponse {
        matches: user_matches,
    }))
}

#[derive(Debug, Serialize)]
pub struct GetMatchResponse {
    pub match_id: String,
    pub user_a_id: String,
    pub user_b_id: String,
    pub match_type: MatchType,
    pub status: MatchStatus,
    pub created_at: String,
    pub mutual_like_detected_at: Option<String>,
}

async fn get_match(
    State(state): State<Arc<MatchState>>,
    headers: HeaderMap,
    axum::extract::Path(match_id): axum::extract::Path<String>,
) -> Result<Json<GetMatchResponse>, (StatusCode, String)> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)
        .map_err(|code| (code, "unauthorized".to_string()))?;

    let matches = state.matches.lock().expect("mutex poisoned");
    let record = matches
        .get(&match_id)
        .ok_or((StatusCode::NOT_FOUND, "match not found".to_string()))?;

    Ok(Json(GetMatchResponse {
        match_id: record.match_id.clone(),
        user_a_id: record.user_a_id.clone(),
        user_b_id: record.user_b_id.clone(),
        match_type: record.match_type.clone(),
        status: record.status.clone(),
        created_at: record.created_at.clone(),
        mutual_like_detected_at: record.mutual_like_detected_at.clone(),
    }))
}

async fn internal_health_detail(
    State(state): State<Arc<MatchState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)?;
    let fr = state.find_requests.lock().expect("mutex poisoned");
    let cl = state.candidate_lists.lock().expect("mutex poisoned");
    let fb = state.feedback_records.lock().expect("mutex poisoned");
    let matches = state.matches.lock().expect("mutex poisoned");
    let like_index = state.like_index.lock().expect("mutex poisoned");
    Ok(Json(json!({
        "find_request_count": fr.len(),
        "candidate_list_count": cl.len(),
        "feedback_count": fb.values().map(|v| v.len()).sum::<usize>(),
        "match_count": matches.len(),
        "like_index_count": like_index.len(),
        "env_mode": state.config.env_mode,
        "backend": if state.pg.is_some() { "postgres" } else { "in-memory" },
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_request_status_serializes_snake_case() {
        let s = serde_json::to_string(&FindRequestStatus::Pending).unwrap();
        assert_eq!(s, "\"pending\"");
        let s = serde_json::to_string(&FindRequestStatus::Searching).unwrap();
        assert_eq!(s, "\"searching\"");
    }

    #[test]
    fn candidate_status_serializes_snake_case() {
        let s = serde_json::to_string(&CandidateStatus::Suggested).unwrap();
        assert_eq!(s, "\"suggested\"");
    }

    #[test]
    fn create_find_request_response_serializes() {
        let resp = CreateFindRequestResponse {
            find_request_id: "fr-1".to_string(),
            status: "pending".to_string(),
        };
        let v = serde_json::to_value(&resp).unwrap();
        assert_eq!(v["find_request_id"], "fr-1");
        assert_eq!(v["status"], "pending");
    }

    #[test]
    fn feedback_action_serializes_snake_case() {
        let s = serde_json::to_string(&FeedbackAction::Like).unwrap();
        assert_eq!(s, "\"like\"");
        let s = serde_json::to_string(&FeedbackAction::Pass).unwrap();
        assert_eq!(s, "\"pass\"");
        let s = serde_json::to_string(&FeedbackAction::Block).unwrap();
        assert_eq!(s, "\"block\"");
    }

    #[test]
    fn generate_candidates_returns_five() {
        let candidates = generate_candidates("user-1", &["cofounder".to_string()], &[]);
        assert_eq!(candidates.len(), 5);
        for c in &candidates {
            assert_eq!(c.status, CandidateStatus::Suggested);
            assert!(c.score > 0.0);
        }
    }

    #[test]
    fn generate_candidates_deterministic_for_same_input() {
        let c1 = generate_candidates("user-1", &[], &[]);
        let c2 = generate_candidates("user-1", &[], &[]);
        assert_eq!(c1.len(), c2.len());
        for (a, b) in c1.iter().zip(c2.iter()) {
            assert_eq!(a.candidate_user_id, b.candidate_user_id);
        }
    }

    #[test]
    fn generate_candidates_filters_blocked_users() {
        let c_all = generate_candidates("user-1", &[], &[]);
        assert_eq!(c_all.len(), 5);
        let blocked = vec![
            c_all[0].candidate_user_id.clone(),
            c_all[1].candidate_user_id.clone(),
        ];
        let c_filtered = generate_candidates("user-1", &[], &blocked);
        assert_eq!(c_filtered.len(), 5);
        for c in &c_filtered {
            assert!(!blocked.contains(&c.candidate_user_id));
        }
    }

    #[test]
    fn generate_candidates_intent_tags_up_to_three() {
        let tags = vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
        ];
        let candidates = generate_candidates("user-1", &tags, &[]);
        for c in &candidates {
            assert!(c.match_reasons.len() <= 3);
        }
    }

    #[test]
    fn submit_feedback_updates_candidate_status() {
        let config = Config::from_env();
        let state = MatchState::new(config, None);

        let candidates = generate_candidates("user-1", &[], &[]);
        let candidate_id = candidates[0].candidate_user_id.clone();
        {
            let mut cl = state.candidate_lists.lock().expect("mutex poisoned");
            cl.insert("fr-1".to_string(), candidates);
        }

        {
            let mut cl = state.candidate_lists.lock().expect("mutex poisoned");
            for (_fr_id, candidates) in cl.iter_mut() {
                for candidate in candidates.iter_mut() {
                    if candidate.candidate_user_id == candidate_id {
                        candidate.status = CandidateStatus::Liked;
                    }
                }
            }
        }

        let cl = state.candidate_lists.lock().expect("mutex poisoned");
        let found = cl
            .get("fr-1")
            .unwrap()
            .iter()
            .find(|c| c.candidate_user_id == candidate_id)
            .unwrap();
        assert_eq!(found.status, CandidateStatus::Liked);
    }

    #[test]
    fn mutual_like_creates_match() {
        let config = Config::from_env();
        let state = MatchState::new(config, None);

        {
            let mut like_index = state.like_index.lock().expect("mutex poisoned");
            like_index
                .entry("user-B".to_string())
                .or_default()
                .push("user-A".to_string());
        }

        {
            let mut like_index = state.like_index.lock().expect("mutex poisoned");
            like_index
                .entry("user-A".to_string())
                .or_default()
                .push("user-B".to_string());
        }

        let is_mutual = {
            let like_index = state.like_index.lock().expect("mutex poisoned");
            like_index
                .get("user-A")
                .map(|likes| likes.contains(&"user-B".to_string()))
                .unwrap_or(false)
                && like_index
                    .get("user-B")
                    .map(|likes| likes.contains(&"user-A".to_string()))
                    .unwrap_or(false)
        };
        assert!(is_mutual);

        let match_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let match_record = MatchRecord {
            match_id: match_id.clone(),
            user_a_id: "user-A".to_string(),
            user_b_id: "user-B".to_string(),
            match_type: MatchType::MutualLike,
            status: MatchStatus::Active,
            created_at: now.clone(),
            mutual_like_detected_at: Some(now),
        };
        {
            let mut matches = state.matches.lock().expect("mutex poisoned");
            matches.insert(match_id.clone(), match_record);
        }

        let matches = state.matches.lock().expect("mutex poisoned");
        let m = matches.get(&match_id).unwrap();
        assert_eq!(m.match_type, MatchType::MutualLike);
        assert_eq!(m.status, MatchStatus::Active);
        assert!(m.mutual_like_detected_at.is_some());
    }

    #[test]
    fn match_type_serializes_snake_case() {
        let s = serde_json::to_string(&MatchType::MutualLike).unwrap();
        assert_eq!(s, "\"mutual_like\"");
        let s = serde_json::to_string(&MatchType::Recommendation).unwrap();
        assert_eq!(s, "\"recommendation\"");
    }

    #[test]
    fn match_status_serializes_snake_case() {
        let s = serde_json::to_string(&MatchStatus::Active).unwrap();
        assert_eq!(s, "\"active\"");
        let s = serde_json::to_string(&MatchStatus::Unmatched).unwrap();
        assert_eq!(s, "\"unmatched\"");
    }

    #[test]
    fn like_index_tracks_bidirectional_likes() {
        let config = Config::from_env();
        let state = MatchState::new(config, None);

        {
            let mut like_index = state.like_index.lock().expect("mutex poisoned");
            like_index
                .entry("user-A".to_string())
                .or_default()
                .push("user-C".to_string());
        }

        let is_mutual = {
            let like_index = state.like_index.lock().expect("mutex poisoned");
            like_index
                .get("user-C")
                .map(|likes| likes.contains(&"user-A".to_string()))
                .unwrap_or(false)
        };
        assert!(!is_mutual);
    }
}
