use axum::http::{HeaderMap, StatusCode};
use onelink_internal_auth::verify_internal_token;

#[derive(Debug)]
pub struct AppError(pub String);

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for AppError {}

impl From<StatusCode> for AppError {
    fn from(code: StatusCode) -> Self {
        AppError(format!("HTTP {code}"))
    }
}

pub fn require_internal_auth(headers: &HeaderMap, secret: &str) -> Result<(), AppError> {
    verify_internal_token(headers, secret).map_err(|code| AppError(format!("unauthorized: {code}")))
}
