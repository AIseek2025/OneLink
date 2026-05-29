//! Environment-driven config with security baseline.

use onelink_internal_auth::DEV_INTERNAL_SECRET;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub identity_service_base_url: String,
    pub ai_chat_service_base_url: String,
    pub question_service_base_url: String,
    pub profile_service_base_url: String,
    pub match_service_base_url: String,
    pub safety_service_base_url: String,
    pub dm_service_base_url: String,
    pub admin_service_base_url: String,
    pub internal_shared_secret: String,
    pub env_mode: String,
    pub cors_allowed_origins: String,
    pub default_locale: String,
    pub default_region: String,
    pub supported_locales: Vec<String>,
    pub supported_regions: Vec<String>,
    pub supported_timezones: Vec<String>,
    pub translations_source: String,
    pub translations_inline: String,
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
            .unwrap_or(8083);
        let identity_service_base_url = env::var("IDENTITY_SERVICE_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8081".to_string());
        let ai_chat_service_base_url = env::var("AI_CHAT_SERVICE_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8085".to_string());
        let question_service_base_url = env::var("QUESTION_SERVICE_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8086".to_string());
        let profile_service_base_url = env::var("PROFILE_SERVICE_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8082".to_string());
        let match_service_base_url = env::var("MATCH_SERVICE_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8087".to_string());
        let safety_service_base_url = env::var("SAFETY_SERVICE_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8088".to_string());
        let dm_service_base_url =
            env::var("DM_SERVICE_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:8089".to_string());
        let admin_service_base_url = env::var("ADMIN_SERVICE_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8090".to_string());
        let internal_shared_secret = internal_shared_secret_from_env();
        let env_mode = env_mode_from_env();
        let cors_allowed_origins = env::var("CORS_ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:3000,http://localhost:5173".to_string());
        let default_locale = env::var("BFF_DEFAULT_LOCALE").unwrap_or_else(|_| "zh-CN".to_string());
        let default_region = env::var("BFF_DEFAULT_REGION").unwrap_or_else(|_| "CN".to_string());
        let supported_locales: Vec<String> = env::var("BFF_SUPPORTED_LOCALES")
            .unwrap_or_else(|_| "zh-CN,en-US,ja-JP,ko-KR,zh-TW".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        let supported_regions: Vec<String> = env::var("BFF_SUPPORTED_REGIONS")
            .unwrap_or_else(|_| "CN,US,JP,KR,TW".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        let supported_timezones: Vec<String> = env::var("BFF_SUPPORTED_TIMEZONES")
            .unwrap_or_else(|_| {
                "Asia/Shanghai,America/New_York,America/Los_Angeles,Asia/Tokyo,Asia/Seoul,Asia/Taipei,Europe/London,Europe/Berlin".to_string()
            })
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        let translations_source =
            env::var("BFF_TRANSLATIONS_SOURCE").unwrap_or_else(|_| "env_inline".to_string());
        let translations_inline = env::var("BFF_TRANSLATIONS_INLINE").unwrap_or_else(|_| {
            r#"{"zh-CN":{"app.title":"OneLink","btn.submit":"提交","btn.cancel":"取消","msg.welcome":"欢迎"},"en-US":{"app.title":"OneLink","btn.submit":"Submit","btn.cancel":"Cancel","msg.welcome":"Welcome"},"ja-JP":{"app.title":"OneLink","btn.submit":"送信","btn.cancel":"キャンセル","msg.welcome":"ようこそ"},"ko-KR":{"app.title":"OneLink","btn.submit":"제출","btn.cancel":"취소","msg.welcome":"환영합니다"}}"#.to_string()
        });
        Self {
            port,
            identity_service_base_url,
            ai_chat_service_base_url,
            question_service_base_url,
            profile_service_base_url,
            match_service_base_url,
            safety_service_base_url,
            dm_service_base_url,
            admin_service_base_url,
            internal_shared_secret,
            env_mode,
            cors_allowed_origins,
            default_locale,
            default_region,
            supported_locales,
            supported_regions,
            supported_timezones,
            translations_source,
            translations_inline,
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
