use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub bff_base_url: String,
    pub app_name: String,
    pub version: String,
    pub port: u16,
    pub cors_allowed_origins: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            bff_base_url: env::var("BFF_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:3000".into()),
            app_name: "OneLink".into(),
            version: "0.1.0".into(),
            port: env::var("APP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8081),
            cors_allowed_origins: env::var("CORS_ALLOWED_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:19006,http://localhost:3000".into()),
        }
    }
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self::default()
    }
}

pub fn validate_secret_for_env(secret: &str, env_mode: &str) -> Result<(), String> {
    if env_mode != "dev" && secret == "dev-only-shared-secret" {
        return Err("default INTERNAL_SHARED_SECRET not allowed outside dev".into());
    }
    Ok(())
}
