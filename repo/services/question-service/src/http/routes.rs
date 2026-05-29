//! Question API (Phase C MVP) + dev relay to context-service.

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use onelink_event_envelope::EventEnvelope;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::config::Config;
use crate::store::{PostgresStore, QuestionStore};

use onelink_internal_auth::INTERNAL_TOKEN_HEADER;

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub client: Client,
    pub store: Arc<QuestionStore>,
    pub pg: Option<Arc<PostgresStore>>,
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/v1/questions/status", get(get_status))
        .route("/api/v1/questions/pending", get(get_pending))
        .route("/api/v1/questions/answers", post(post_answers))
        .route("/api/v1/questions/completion", get(get_completion))
        .with_state(state)
}

#[derive(Debug, Deserialize)]
pub struct PendingQuery {
    pub channel: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    5
}

#[derive(Debug, Deserialize)]
pub struct SubmitAnswerBody {
    pub delivery_id: String,
    pub variant_id: String,
    pub answer_payload: Value,
    pub answer_state: String,
    pub idempotency_key: Option<String>,
}

async fn identity_user_id(state: &AppState, headers: &HeaderMap) -> Result<String, StatusCode> {
    let auth = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
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
        .map_err(|_| StatusCode::BAD_GATEWAY)?;
    if !response.status().is_success() {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let me: Value = response.json().await.map_err(|_| StatusCode::BAD_GATEWAY)?;
    me.get("user_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or(StatusCode::BAD_GATEWAY)
}

async fn get_status(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    let user_id = identity_user_id(&state, &headers).await?;
    if let Some(pg) = &state.pg {
        match pg.status_json(&user_id).await {
            Ok(v) => return Ok(Json(v)),
            Err(e) => {
                tracing::warn!(error = %e, "Postgres status_json failed, falling back to in-memory");
            }
        }
    }
    Ok(Json(state.store.status_json(&user_id)))
}

async fn get_completion(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    let user_id = identity_user_id(&state, &headers).await?;
    if let Some(pg) = &state.pg {
        match pg.status_json(&user_id).await {
            Ok(v) => return Ok(Json(v)),
            Err(e) => {
                tracing::warn!(error = %e, "Postgres status_json failed, falling back to in-memory");
            }
        }
    }
    Ok(Json(state.store.status_json(&user_id)))
}

async fn get_pending(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<PendingQuery>,
) -> Result<Json<Value>, StatusCode> {
    let user_id = identity_user_id(&state, &headers).await?;
    let limit = q.limit.clamp(1, 20);
    if let Some(pg) = &state.pg {
        match pg.pending_json(&user_id, limit).await {
            Ok(v) => return Ok(Json(v)),
            Err(e) => {
                tracing::warn!(error = %e, "Postgres pending_json failed, falling back to in-memory");
            }
        }
    }
    Ok(Json(state.store.pending_json(
        &user_id,
        q.channel.as_deref(),
        limit,
    )))
}

async fn post_answers(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<SubmitAnswerBody>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let user_id = identity_user_id(&state, &headers)
        .await
        .map_err(|c| (c, "unauthorized".to_string()))?;

    let (stored, is_new) = if let Some(pg) = &state.pg {
        match pg
            .submit_answer(
                &user_id,
                &body.delivery_id,
                &body.variant_id,
                body.answer_payload.clone(),
                &body.answer_state,
            )
            .await
        {
            Ok(result) => result,
            Err(e) => {
                tracing::warn!(error = %e, "Postgres submit_answer failed, falling back to in-memory");
                state
                    .store
                    .submit_answer(
                        &user_id,
                        &body.delivery_id,
                        &body.variant_id,
                        body.answer_payload.clone(),
                        &body.answer_state,
                    )
                    .map_err(|e| (StatusCode::BAD_REQUEST, e))?
            }
        }
    } else {
        state
            .store
            .submit_answer(
                &user_id,
                &body.delivery_id,
                &body.variant_id,
                body.answer_payload.clone(),
                &body.answer_state,
            )
            .map_err(|e| (StatusCode::BAD_REQUEST, e))?
    };

    if is_new && stored.answer_state == "answered" {
        let st = Arc::clone(&state);
        let trace_id = Uuid::new_v4().to_string();
        let envelope = EventEnvelope::new_v1(
            "question.answered.v1",
            "question-service",
            Some(user_id.clone()),
            Some(trace_id.clone()),
            json!({
                "answer_id": stored.answer_id,
                "delivery_id": stored.delivery_id,
                "user_id": user_id,
                "variant_id": stored.variant_id,
                "answer_state": stored.answer_state,
                "question_text": stored.question_text,
                "answer_payload": stored.answer_payload,
                "answer_text": stored.answer_text,
                "requirement_tier": stored.requirement_tier,
                "question_style": stored.question_style,
            }),
        );
        let url = format!(
            "{}/internal/events/receive",
            st.config.context_service_base_url
        );
        let client = st.client.clone();
        let secret = st.config.internal_shared_secret.clone();
        tokio::spawn(async move {
            for attempt in 1..=3_u32 {
                match client
                    .post(&url)
                    .header(INTERNAL_TOKEN_HEADER, secret.as_str())
                    .json(&envelope)
                    .send()
                    .await
                {
                    Ok(r) if r.status().is_success() => return,
                    Ok(r) => {
                        tracing::warn!(
                            status = %r.status(),
                            attempt,
                            "context relay question.answered.v1 non-success"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, attempt, "context relay question.answered.v1 failed");
                    }
                }
                if attempt < 3 {
                    tokio::time::sleep(std::time::Duration::from_millis(120 * u64::from(attempt)))
                        .await;
                }
            }
        });
    }

    Ok(Json(json!({
        "answer_id": stored.answer_id,
        "delivery_id": stored.delivery_id,
        "answered_at": stored.answered_at,
    })))
}
