use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::{
    extract::State,
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use onelink_event_envelope::EventEnvelope;
use onelink_internal_auth::verify_internal_token;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::config::Config;
use crate::store::postgres::PostgresStore;

pub fn router(state: Arc<IdentityState>) -> Router {
    Router::new()
        .route("/api/v1/identity/register", post(register))
        .route("/api/v1/identity/login", post(login))
        .route("/api/v1/identity/me", get(me))
        .route("/api/v1/identity/logout", post(logout))
        .route("/api/v1/identity/logout-all", delete(logout_all))
        .route(
            "/internal/identity/health-detail",
            get(internal_health_detail),
        )
        .with_state(state)
}

#[derive(Debug)]
pub struct IdentityState {
    pub config: Config,
    pub inner: Mutex<Inner>,
    pub pg: Option<Arc<PostgresStore>>,
}

impl IdentityState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            inner: Mutex::new(Inner::default()),
            pg: None,
        }
    }

    pub fn with_pg(config: Config, pg: Arc<PostgresStore>) -> Self {
        Self {
            config,
            inner: Mutex::new(Inner::default()),
            pg: Some(pg),
        }
    }
}

#[derive(Debug, Default)]
pub struct Inner {
    email_index: HashMap<String, String>,
    users: HashMap<String, UserRecord>,
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
    password: Option<String>,
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
    password: Option<String>,
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

fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|ph| ph.to_string())
        .map_err(|e| format!("argon2 hash failed: {e}"))
}

fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash).ok().is_some_and(|ph| {
        Argon2::default()
            .verify_password(password.as_bytes(), &ph)
            .is_ok()
    })
}

fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

fn issue_token() -> String {
    format!("olk_{}", Uuid::new_v4())
}

fn hash_token_for_storage(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("sha256:{}", hex::encode(hasher.finalize()))
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
    let password = body.password.clone().unwrap_or_default();
    if password.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "password required".to_string()));
    }

    let hashed = hash_password(&password).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    if let Some(pg) = &state.pg {
        return register_pg(pg, &email, &hashed, &body).await;
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
        password_hash: hashed,
    };
    g.email_index.insert(email, user_id.clone());
    g.users.insert(user_id.clone(), user);

    let token = issue_token();
    let (expires_at_dt, expires_at_str) = new_session_expiry();
    let token_hash = hash_token_for_storage(&token);
    g.sessions.insert(
        token_hash,
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

async fn register_pg(
    pg: &PostgresStore,
    email: &str,
    hashed: &str,
    body: &RegisterRequest,
) -> Result<Json<RegisterResponse>, (StatusCode, String)> {
    if pg
        .find_user_by_email(email)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .is_some()
    {
        return Err((StatusCode::CONFLICT, "already registered".to_string()));
    }

    let user_id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let user_row = crate::store::memory::UserRow {
        user_id: user_id.clone(),
        status: "active".to_string(),
        primary_region: body.primary_region.clone(),
        primary_language: body.primary_language.clone(),
        timezone: "UTC".to_string(),
        created_at: created_at.clone(),
        password_hash: hashed.to_string(),
    };
    pg.insert_user(&user_row)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    pg.insert_email_binding(&user_id, email)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let token = issue_token();
    let (expires_at_dt, expires_at_str) = new_session_expiry();
    let token_hash = hash_token_for_storage(&token);
    let session_row = crate::store::memory::SessionRow {
        token: token_hash.clone(),
        user_id: user_id.clone(),
        expires_at: expires_at_dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    };
    pg.insert_session(&token_hash, &session_row)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tracing::info!(user_id = %user_id, "identity.user.registered (session issued, pg)");

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
    let password = body.password.clone().unwrap_or_default();

    if let Some(pg) = &state.pg {
        return login_pg(pg, &email, &password).await;
    }

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
    if !verify_password(&password, &user.password_hash) {
        return Err((StatusCode::UNAUTHORIZED, "invalid credentials".to_string()));
    }

    let token = issue_token();
    let (expires_at_dt, expires_at_str) = new_session_expiry();
    let token_hash = hash_token_for_storage(&token);
    g.sessions.insert(
        token_hash,
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

async fn login_pg(
    pg: &PostgresStore,
    email: &str,
    password: &str,
) -> Result<Json<LoginResponse>, (StatusCode, String)> {
    let (_uid, user_row) = pg
        .find_user_by_email(email)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::UNAUTHORIZED, "invalid credentials".to_string()))?;
    if !verify_password(password, &user_row.password_hash) {
        return Err((StatusCode::UNAUTHORIZED, "invalid credentials".to_string()));
    }

    let token = issue_token();
    let (expires_at_dt, expires_at_str) = new_session_expiry();
    let token_hash = hash_token_for_storage(&token);
    let session_row = crate::store::memory::SessionRow {
        token: token_hash.clone(),
        user_id: user_row.user_id.clone(),
        expires_at: expires_at_dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
    };
    pg.insert_session(&token_hash, &session_row)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(LoginResponse {
        user_id: user_row.user_id,
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
    let user_id = resolve_bearer(&state, &headers).await?;

    if let Some(pg) = &state.pg {
        let user_row = pg
            .find_user_by_id(&user_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or((StatusCode::UNAUTHORIZED, "invalid session".to_string()))?;
        return Ok(Json(MeResponse {
            user_id: user_row.user_id,
            status: user_row.status,
            primary_region: user_row.primary_region,
            primary_language: user_row.primary_language,
            timezone: user_row.timezone,
            created_at: user_row.created_at,
        }));
    }

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

async fn logout(
    State(state): State<Arc<IdentityState>>,
    headers: HeaderMap,
) -> Result<StatusCode, (StatusCode, String)> {
    let token = extract_bearer_token(&headers)?;
    let token_hash = hash_token_for_storage(token);

    if let Some(pg) = &state.pg {
        pg.delete_session(&token_hash)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    } else {
        let mut g = state.inner.lock().expect("identity mutex poisoned");
        g.sessions.remove(&token_hash);
    }

    tracing::info!("identity.session.revoked (logout)");
    Ok(StatusCode::NO_CONTENT)
}

async fn logout_all(
    State(state): State<Arc<IdentityState>>,
    headers: HeaderMap,
) -> Result<StatusCode, (StatusCode, String)> {
    let user_id = resolve_bearer(&state, &headers).await?;

    if let Some(pg) = &state.pg {
        pg.delete_sessions_by_user_id(&user_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    } else {
        let mut g = state.inner.lock().expect("identity mutex poisoned");
        g.sessions.retain(|_, rec| rec.user_id != user_id);
    }

    tracing::info!(user_id = %user_id, "identity.sessions.revoked_all (logout-all)");
    Ok(StatusCode::NO_CONTENT)
}

fn extract_bearer_token(headers: &HeaderMap) -> Result<&str, (StatusCode, String)> {
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
    Ok(token)
}

async fn internal_health_detail(
    State(state): State<Arc<IdentityState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    verify_internal_token(&headers, &state.config.internal_shared_secret)?;

    if let Some(pg) = &state.pg {
        let user_count = pg.count_users().await.unwrap_or(0);
        let session_count = pg.count_sessions().await.unwrap_or(0);
        return Ok(Json(json!({
            "user_count": user_count,
            "session_count": session_count,
            "env_mode": state.config.env_mode,
            "uses_argon2": true,
            "backend": "postgres",
        })));
    }

    let g = state.inner.lock().expect("identity mutex poisoned");
    Ok(Json(json!({
        "user_count": g.users.len(),
        "session_count": g.sessions.len(),
        "env_mode": state.config.env_mode,
        "uses_argon2": true,
        "backend": "in-memory",
    })))
}

async fn resolve_bearer(
    state: &IdentityState,
    headers: &HeaderMap,
) -> Result<String, (StatusCode, String)> {
    let token = extract_bearer_token(headers)?;

    if let Some(pg) = &state.pg {
        let token_hash = hash_token_for_storage(token);
        let session = pg
            .find_session(&token_hash)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        match session {
            Some(s) => {
                let expires_at = DateTime::parse_from_rfc3339(&s.expires_at)
                    .map(|dt| dt.to_utc())
                    .unwrap_or_else(|_| Utc::now());
                if Utc::now() <= expires_at {
                    return Ok(s.user_id);
                }
                let _ = pg.delete_session(&token_hash).await;
                return Err((StatusCode::UNAUTHORIZED, "token expired".to_string()));
            }
            None => return Err((StatusCode::UNAUTHORIZED, "invalid token".to_string())),
        }
    }

    let mut g = state.inner.lock().expect("identity mutex poisoned");
    let now = Utc::now();
    let token_hash = hash_token_for_storage(token);
    match g.sessions.get(&token_hash) {
        Some(rec) if now <= rec.expires_at => Ok(rec.user_id.clone()),
        Some(_) => {
            g.sessions.remove(&token_hash);
            Err((StatusCode::UNAUTHORIZED, "token expired".to_string()))
        }
        None => Err((StatusCode::UNAUTHORIZED, "invalid token".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use onelink_internal_auth::DEV_INTERNAL_SECRET as DEV_SECRET;
    use onelink_internal_auth::{validate_secret_for_env, INTERNAL_TOKEN_HEADER};

    fn test_config() -> Config {
        Config {
            port: 8081,
            internal_shared_secret: "test-internal-secret-at-least-32-chars!!".to_string(),
            env_mode: "dev".to_string(),
            database_url: None,
            internal_bind_addr: "127.0.0.1".to_string(),
        }
    }

    #[test]
    fn argon2_hash_and_verify_roundtrip() {
        let password = "test-password-123";
        let hash = hash_password(password).unwrap();
        assert!(verify_password(password, &hash));
        assert!(!verify_password("wrong-password", &hash));
    }

    #[test]
    fn argon2_different_salts_produce_different_hashes() {
        let h1 = hash_password("same-password").unwrap();
        let h2 = hash_password("same-password").unwrap();
        assert_ne!(h1, h2);
        assert!(verify_password("same-password", &h1));
        assert!(verify_password("same-password", &h2));
    }

    #[test]
    fn verify_password_rejects_invalid_hash_format() {
        assert!(!verify_password("any-password", "not-a-valid-hash"));
    }

    #[test]
    fn verify_internal_token_accepts_valid() {
        let config = test_config();
        let state = Arc::new(IdentityState::new(config));
        let mut headers = HeaderMap::new();
        headers.insert(
            INTERNAL_TOKEN_HEADER,
            "test-internal-secret-at-least-32-chars!!".parse().unwrap(),
        );
        assert!(verify_internal_token(&headers, &state.config.internal_shared_secret).is_ok());
    }

    #[test]
    fn verify_internal_token_rejects_invalid() {
        let config = test_config();
        let state = Arc::new(IdentityState::new(config));
        let mut headers = HeaderMap::new();
        headers.insert(INTERNAL_TOKEN_HEADER, "wrong-token".parse().unwrap());
        assert_eq!(
            verify_internal_token(&headers, &state.config.internal_shared_secret),
            Err(StatusCode::UNAUTHORIZED)
        );
    }

    #[test]
    fn verify_internal_token_rejects_missing() {
        let config = test_config();
        let state = Arc::new(IdentityState::new(config));
        let headers = HeaderMap::new();
        assert_eq!(
            verify_internal_token(&headers, &state.config.internal_shared_secret),
            Err(StatusCode::UNAUTHORIZED)
        );
    }

    #[test]
    fn secret_validation_blocks_default_in_staging() {
        assert!(validate_secret_for_env(DEV_SECRET, "staging").is_err());
        assert!(validate_secret_for_env(DEV_SECRET, "production").is_err());
    }

    #[test]
    fn secret_validation_accepts_default_in_dev() {
        assert!(validate_secret_for_env(DEV_SECRET, "dev").is_ok());
    }

    #[test]
    fn secret_validation_blocks_short_in_production() {
        assert!(validate_secret_for_env("short-secret-only-20chars", "production").is_err());
    }

    #[test]
    fn secret_validation_accepts_long_in_production() {
        assert!(validate_secret_for_env(
            "a-very-long-secret-that-is-at-least-32-characters-long",
            "production"
        )
        .is_ok());
    }

    #[test]
    fn register_stores_argon2_hash_not_plaintext() {
        let config = test_config();
        let state = Arc::new(IdentityState::new(config));
        let password = "mypassword";
        let hash = hash_password(password).unwrap();

        {
            let mut g = state.inner.lock().expect("mutex");
            let user_id = Uuid::new_v4().to_string();
            g.email_index
                .insert("test@example.com".to_string(), user_id.clone());
            g.users.insert(
                user_id.clone(),
                UserRecord {
                    user_id,
                    status: "active".to_string(),
                    primary_region: "CN".to_string(),
                    primary_language: "zh".to_string(),
                    timezone: "UTC".to_string(),
                    created_at: "2026-01-01T00:00:00Z".to_string(),
                    password_hash: hash,
                },
            );
        }

        let g = state.inner.lock().expect("mutex");
        let user = g
            .users
            .get(g.email_index.get("test@example.com").unwrap())
            .unwrap();
        assert!(!user.password_hash.contains(password));
        assert!(user.password_hash.starts_with("$argon2"));
        assert!(verify_password(password, &user.password_hash));
        assert!(!verify_password("wrongpassword", &user.password_hash));
    }

    #[test]
    fn hash_token_for_storage_is_deterministic_and_opaque() {
        let token = "olk_00000000-0000-0000-0000-000000000001";
        let h1 = hash_token_for_storage(token);
        let h2 = hash_token_for_storage(token);
        assert_eq!(h1, h2, "same token must produce same hash");
        assert!(h1.starts_with("sha256:"), "hash must be prefixed");
        assert!(!h1.contains(token), "hash must not contain raw token");
        let different = "olk_00000000-0000-0000-0000-000000000002";
        let h3 = hash_token_for_storage(different);
        assert_ne!(h1, h3, "different tokens must produce different hashes");
    }
}
