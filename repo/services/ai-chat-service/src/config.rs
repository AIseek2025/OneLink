//! Environment-driven config with security baseline.

use onelink_internal_auth::DEV_INTERNAL_SECRET;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub identity_service_base_url: String,
    pub context_service_base_url: String,
    pub model_gateway_base_url: String,
    pub internal_shared_secret: String,
    pub env_mode: String,
    pub database_url: Option<String>,
    pub internal_bind_addr: String,
}

fn internal_shared_secret_from_env() -> String {
    let s = env::var("INTERNAL_SHARED_SECRET").unwrap_or_else(|_| DEV_INTERNAL_SECRET.to_string());
    if s.trim().is_empty() {
        DEV_INTERNAL_SECRET.to_string()
    } else {
        s
    }
}

fn env_mode_from_env() -> String {
    env::var("ONELINK_ENV")
        .unwrap_or_else(|_| "dev".to_string())
        .to_lowercase()
}

pub use onelink_internal_auth::validate_secret_for_env;

impl Config {
    pub fn from_env() -> Self {
        let port = env::var("PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8085);
        let identity_service_base_url = env::var("IDENTITY_SERVICE_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8081".to_string());
        let context_service_base_url = env::var("CONTEXT_SERVICE_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8089".to_string());
        let model_gateway_base_url = env::var("MODEL_GATEWAY_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8090".to_string());
        let internal_shared_secret = internal_shared_secret_from_env();
        let env_mode = env_mode_from_env();
        let database_url = env::var("DATABASE_URL")
            .ok()
            .filter(|s| !s.trim().is_empty());
        let internal_bind_addr =
            env::var("INTERNAL_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1".to_string());
        Self {
            port,
            identity_service_base_url,
            context_service_base_url,
            model_gateway_base_url,
            internal_shared_secret,
            env_mode,
            database_url,
            internal_bind_addr,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use onelink_internal_auth::DEV_INTERNAL_SECRET as DEV_SECRET;

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
}
