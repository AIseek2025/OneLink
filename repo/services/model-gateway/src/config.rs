//! Environment-driven config with security baseline.

use onelink_internal_auth::DEV_INTERNAL_SECRET;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub internal_shared_secret: String,
    pub env_mode: String,
    pub internal_bind_addr: String,
    pub deepseek_base_url: String,
    pub deepseek_api_key: Option<String>,
    pub deepseek_model: String,
    pub deepseek_timeout_ms: u64,
    pub deepseek_thinking_type: String,
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
            .unwrap_or(8090);
        let internal_shared_secret = internal_shared_secret_from_env();
        let env_mode = env_mode_from_env();
        let internal_bind_addr =
            env::var("INTERNAL_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1".to_string());
        let deepseek_base_url = env::var("DEEPSEEK_BASE_URL")
            .unwrap_or_else(|_| "https://api.deepseek.com".to_string());
        let deepseek_api_key = env::var("DEEPSEEK_API_KEY")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let deepseek_model =
            env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| "deepseek-v4-flash".to_string());
        let deepseek_timeout_ms = env::var("DEEPSEEK_TIMEOUT_MS")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(60_000);
        let deepseek_thinking_type = env::var("DEEPSEEK_THINKING_TYPE")
            .unwrap_or_else(|_| "disabled".to_string())
            .to_lowercase();
        Self {
            port,
            internal_shared_secret,
            env_mode,
            internal_bind_addr,
            deepseek_base_url,
            deepseek_api_key,
            deepseek_model,
            deepseek_timeout_ms,
            deepseek_thinking_type,
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
