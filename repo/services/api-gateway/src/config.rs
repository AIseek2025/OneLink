//! Environment-driven config (skeleton).

use onelink_internal_auth::DEV_INTERNAL_SECRET;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub internal_shared_secret: String,
    pub env_mode: String,
}

fn internal_shared_secret_from_env() -> String {
    let s =
        std::env::var("INTERNAL_SHARED_SECRET").unwrap_or_else(|_| DEV_INTERNAL_SECRET.to_string());
    if s.trim().is_empty() {
        DEV_INTERNAL_SECRET.to_string()
    } else {
        s
    }
}

fn env_mode_from_env() -> String {
    std::env::var("ONELINK_ENV")
        .unwrap_or_else(|_| "dev".to_string())
        .to_lowercase()
}

pub use onelink_internal_auth::validate_secret_for_env;

impl Config {
    pub fn from_env() -> Self {
        let port = std::env::var("PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8080);
        Self {
            port,
            internal_shared_secret: internal_shared_secret_from_env(),
            env_mode: env_mode_from_env(),
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
