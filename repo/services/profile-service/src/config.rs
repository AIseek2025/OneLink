//! Environment-driven config (skeleton).

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub identity_service_base_url: String,
    pub context_service_base_url: String,
    /// Dev-only shared secret for `x-internal-token` (must match ai-chat/context).
    pub internal_shared_secret: String,
}

fn internal_shared_secret_from_env() -> String {
    let s = std::env::var("INTERNAL_SHARED_SECRET")
        .unwrap_or_else(|_| "onelink-dev-internal-token".to_string());
    if s.trim().is_empty() {
        "onelink-dev-internal-token".to_string()
    } else {
        s
    }
}

impl Config {
    pub fn from_env() -> Self {
        let port = std::env::var("PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8082);
        let identity_service_base_url = std::env::var("IDENTITY_SERVICE_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8081".to_string());
        let context_service_base_url = std::env::var("CONTEXT_SERVICE_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8089".to_string());
        let internal_shared_secret = internal_shared_secret_from_env();
        Self {
            port,
            identity_service_base_url,
            context_service_base_url,
            internal_shared_secret,
        }
    }
}
