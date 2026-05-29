pub mod config;
pub mod i18n;
pub mod router;
pub mod state;

use axum::Router;
use config::AppConfig;
use state::AppState;

use axum::http::{HeaderValue, Method};
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};

pub fn create_app() -> Router {
    let config = AppConfig::from_env();
    let state = AppState::new(config.clone());
    let cors = build_cors_layer(&config.cors_allowed_origins);

    Router::new().merge(router::router(state)).layer(cors)
}

fn build_cors_layer(origins: &str) -> CorsLayer {
    let allowed: Vec<HeaderValue> = origins
        .split(',')
        .filter_map(|o| o.trim().parse().ok())
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed))
        .allow_methods(AllowMethods::list([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ]))
        .allow_headers(AllowHeaders::list([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
        ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_app_not_empty() {
        let _app = create_app();
    }

    #[test]
    fn test_config_default() {
        let config = AppConfig::default();
        assert_eq!(config.app_name, "OneLink");
        assert_eq!(config.version, "0.1.0");
        assert!(!config.bff_base_url.is_empty());
    }

    #[test]
    fn test_validate_secret_dev() {
        assert!(config::validate_secret_for_env("dev-only-shared-secret", "dev").is_ok());
    }

    #[test]
    fn test_validate_secret_prod_rejects_default() {
        assert!(config::validate_secret_for_env("dev-only-shared-secret", "production").is_err());
    }

    #[test]
    fn test_validate_secret_prod_allows_custom() {
        assert!(config::validate_secret_for_env("real-secret-value", "production").is_ok());
    }
}
