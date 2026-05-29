use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackConfig {
    pub enabled: bool,
    pub fallback_model_id: String,
    pub fallback_message: String,
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            fallback_model_id: "fallback-mini-v1".to_string(),
            fallback_message: "Service temporarily degraded. Using fallback response.".to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FallbackResponse {
    pub capability_name: String,
    pub model_id: String,
    pub output_text: String,
    pub fallback_reason: String,
    pub degraded: bool,
}

impl FallbackResponse {
    pub fn budget_exceeded(capability: &str, config: &FallbackConfig) -> Self {
        Self {
            capability_name: capability.to_string(),
            model_id: config.fallback_model_id.clone(),
            output_text: config.fallback_message.clone(),
            fallback_reason: "token_budget_exceeded".to_string(),
            degraded: true,
        }
    }

    pub fn circuit_open(capability: &str, config: &FallbackConfig) -> Self {
        Self {
            capability_name: capability.to_string(),
            model_id: config.fallback_model_id.clone(),
            output_text: config.fallback_message.clone(),
            fallback_reason: "circuit_breaker_open".to_string(),
            degraded: true,
        }
    }

    pub fn bulkhead_rejected(capability: &str, config: &FallbackConfig) -> Self {
        Self {
            capability_name: capability.to_string(),
            model_id: config.fallback_model_id.clone(),
            output_text: config.fallback_message.clone(),
            fallback_reason: "bulkhead_at_capacity".to_string(),
            degraded: true,
        }
    }

    pub fn provider_error(capability: &str, config: &FallbackConfig, error_msg: &str) -> Self {
        Self {
            capability_name: capability.to_string(),
            model_id: config.fallback_model_id.clone(),
            output_text: format!("{}. Original error: {}", config.fallback_message, error_msg),
            fallback_reason: "provider_error".to_string(),
            degraded: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_budget_exceeded_response() {
        let config = FallbackConfig::default();
        let resp = FallbackResponse::budget_exceeded("chat.respond", &config);
        assert!(resp.degraded);
        assert_eq!(resp.fallback_reason, "token_budget_exceeded");
        assert_eq!(resp.model_id, "fallback-mini-v1");
    }

    #[test]
    fn fallback_circuit_open_response() {
        let config = FallbackConfig::default();
        let resp = FallbackResponse::circuit_open("chat.respond", &config);
        assert!(resp.degraded);
        assert_eq!(resp.fallback_reason, "circuit_breaker_open");
    }

    #[test]
    fn fallback_bulkhead_rejected_response() {
        let config = FallbackConfig::default();
        let resp = FallbackResponse::bulkhead_rejected("match.recommend", &config);
        assert!(resp.degraded);
        assert_eq!(resp.fallback_reason, "bulkhead_at_capacity");
    }

    #[test]
    fn fallback_provider_error_includes_message() {
        let config = FallbackConfig::default();
        let resp = FallbackResponse::provider_error("safety.review", &config, "timeout");
        assert!(resp.degraded);
        assert!(resp.output_text.contains("timeout"));
    }
}
