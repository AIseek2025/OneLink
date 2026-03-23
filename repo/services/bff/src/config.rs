//! Environment-driven config (skeleton).

#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
    pub identity_service_base_url: String,
    pub ai_chat_service_base_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        let port = std::env::var("PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8083);
        let identity_service_base_url = std::env::var("IDENTITY_SERVICE_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8081".to_string());
        let ai_chat_service_base_url = std::env::var("AI_CHAT_SERVICE_BASE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8085".to_string());
        Self {
            port,
            identity_service_base_url,
            ai_chat_service_base_url,
        }
    }
}
