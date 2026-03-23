//! Identity API — MVP in-memory session/token.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::{
    extract::State,
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use onelink_event_envelope::EventEnvelope;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

pub fn router(state: Arc<IdentityState>) -> Router {
    Router::new()
        .route("/api/v1/identity/register", post(register))
        .route("/api/v1/identity/login", post(login))
        .route("/api/v1/identity/me", get(me))
        .with_state(state)
}

#[derive(Debug, Default)]
pub struct IdentityState {
    inner: Mutex<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    /// email (normalized) -> user_id
    email_index: HashMap<String, String>,
    users: HashMap<String, UserRecord>,
    /// bearer token -> session
    sessions: HashMap<String, SessionRecord>,
}

#[derive(Debug, Clone)]
struct SessionRecord {
    user_id: String,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
struct UserRecord {
    user_id: String,
    status: String,
    primary_region: String,
    primary_language: String,
    timezone: String,
    created_at: String,
    password_hash: String,
}

#[derive(Debug, Deserialize)]
struct RegisterRequest {
    provider: String,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    phone: Option<String>,
    #[serde(default)]
    password_hash: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    verification_code: Option<String>,
    primary_region: String,
    primary_language: String,
}

#[derive(Debug, Serialize)]
struct RegisterResponse {
    user_id: String,
    session: SessionPayload,
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    provider: String,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    phone: Option<String>,
    #[serde(default)]
    password_hash: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    oauth_token: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    verification_code: Option<String>,
}

#[derive(Debug, Serialize)]
struct LoginResponse {
    user_id: String,
    session: SessionPayload,
}

#[derive(Debug, Serialize)]
struct SessionPayload {
    token: String,
    expires_at: String,
}

#[derive(Debug, Serialize)]
struct MeResponse {
    user_id: String,
    status: String,
    primary_region: String,
    primary_language: String,
    timezone: String,
    created_at: String,
}

fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

fn issue_token() -> String {
    format!("olk_{}", Uuid::new_v4())
}

fn session_ttl() -> Duration {
    Duration::hours(24 * 30)
}

fn new_session_expiry() -> (DateTime<Utc>, String) {
    let expires_at = Utc::now() + session_ttl();
    let s = expires_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    (expires_at, s)
}

async fn register(
    State(state): State<Arc<IdentityState>>,
    Json(body): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, (StatusCode, String)> {
    if body.provider != "email" && body.provider != "password" {
        return Err((
            StatusCode::BAD_REQUEST,
            "unsupported provider for MVP".to_string(),
        ));
    }
    let email = body
        .email
        .as_deref()
        .map(normalize_email)
        .filter(|e| !e.is_empty())
        .ok_or((StatusCode::BAD_REQUEST, "email required".to_string()))?;
    let password_hash = body.password_hash.clone().unwrap_or_default();
    if password_hash.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "password_hash required".to_string(),
        ));
    }

    let mut g = state.inner.lock().expect("identity mutex poisoned");
    if g.email_index.contains_key(&email) {
        return Err((StatusCode::CONFLICT, "already registered".to_string()));
    }

    let user_id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let user = UserRecord {
        user_id: user_id.clone(),
        status: "active".to_string(),
        primary_region: body.primary_region.clone(),
        primary_language: body.primary_language.clone(),
        timezone: "UTC".to_string(),
        created_at: created_at.clone(),
        password_hash,
    };
    g.email_index.insert(email, user_id.clone());
    g.users.insert(user_id.clone(), user);

    let token = issue_token();
    let (expires_at_dt, expires_at_str) = new_session_expiry();
    g.sessions.insert(
        token.clone(),
        SessionRecord {
            user_id: user_id.clone(),
            expires_at: expires_at_dt,
        },
    );

    tracing::info!(user_id = %user_id, "identity.user.registered (session issued)");

    let env = EventEnvelope::new_v1(
        "identity.user.registered.v1",
        "identity-service",
        Some(user_id.clone()),
        None,
        json!({ "user_id": user_id.clone() }),
    );
    tracing::debug!(envelope = %serde_json::to_string(&env).unwrap_or_default(), "event envelope (audit)");

    Ok(Json(RegisterResponse {
        user_id,
        session: SessionPayload {
            token,
            expires_at: expires_at_str,
        },
    }))
}

async fn login(
    State(state): State<Arc<IdentityState>>,
    Json(body): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, String)> {
    if body.provider != "email" && body.provider != "password" {
        return Err((
            StatusCode::BAD_REQUEST,
            "unsupported provider for MVP".to_string(),
        ));
    }
    let email = body
        .email
        .as_deref()
        .map(normalize_email)
        .filter(|e| !e.is_empty())
        .ok_or((StatusCode::BAD_REQUEST, "email required".to_string()))?;
    let password_hash = body.password_hash.clone().unwrap_or_default();

    let mut g = state.inner.lock().expect("identity mutex poisoned");
    let user_id = g
        .email_index
        .get(&email)
        .cloned()
        .ok_or((StatusCode::UNAUTHORIZED, "invalid credentials".to_string()))?;
    let user = g
        .users
        .get(&user_id)
        .ok_or((StatusCode::UNAUTHORIZED, "invalid credentials".to_string()))?;
    if user.password_hash != password_hash {
        return Err((StatusCode::UNAUTHORIZED, "invalid credentials".to_string()));
    }

    let token = issue_token();
    let (expires_at_dt, expires_at_str) = new_session_expiry();
    g.sessions.insert(
        token.clone(),
        SessionRecord {
            user_id: user_id.clone(),
            expires_at: expires_at_dt,
        },
    );

    Ok(Json(LoginResponse {
        user_id,
        session: SessionPayload {
            token,
            expires_at: expires_at_str,
        },
    }))
}

async fn me(
    State(state): State<Arc<IdentityState>>,
    headers: HeaderMap,
) -> Result<Json<MeResponse>, (StatusCode, String)> {
    let user_id = resolve_bearer(&state, &headers)?;
    let g = state.inner.lock().expect("identity mutex poisoned");
    let user = g
        .users
        .get(&user_id)
        .ok_or((StatusCode::UNAUTHORIZED, "invalid session".to_string()))?;

    Ok(Json(MeResponse {
        user_id: user.user_id.clone(),
        status: user.status.clone(),
        primary_region: user.primary_region.clone(),
        primary_language: user.primary_language.clone(),
        timezone: user.timezone.clone(),
        created_at: user.created_at.clone(),
    }))
}

fn resolve_bearer(
    state: &IdentityState,
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
    let mut g = state.inner.lock().expect("identity mutex poisoned");
    let now = Utc::now();
    match g.sessions.get(token) {
        Some(rec) if now <= rec.expires_at => Ok(rec.user_id.clone()),
        Some(_) => {
            g.sessions.remove(token);
            Err((
                StatusCode::UNAUTHORIZED,
                "token expired".to_string(),
            ))
        }
        None => Err((StatusCode::UNAUTHORIZED, "invalid token".to_string())),
    }
}
