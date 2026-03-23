//! Environment-driven config (skeleton).

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub default_reply_style: String,
    pub profile_service_base_url: String,
    /// Used to load user message text for `chat.user_message.created.v1` handling (payload has ids only).
    pub ai_chat_service_base_url: String,
    /// Dev-only shared secret for `x-internal-token` (must match ai-chat/profile).
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
            .unwrap_or(8089);
        let default_reply_style =
            std::env::var("DEFAULT_REPLY_STYLE").unwrap_or_else(|_| "brief".to_string());
        let profile_service_base_url = std::env::var("PROFILE_SERVICE_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8082".to_string());
        let ai_chat_service_base_url = std::env::var("AI_CHAT_SERVICE_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8085".to_string());
        let internal_shared_secret = internal_shared_secret_from_env();
        Self {
            port,
            default_reply_style,
            profile_service_base_url,
            ai_chat_service_base_url,
            internal_shared_secret,
        }
    }
}
