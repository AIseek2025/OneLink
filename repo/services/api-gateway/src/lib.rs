//! OneLink `api-gateway` service skeleton.

pub mod config;
pub mod errors;
pub mod health;
pub mod http;

use std::net::SocketAddr;

const SERVICE_NAME: &str = "api-gateway";

/// Minimal HTTP server: health + placeholder routes.
pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = config::Config::from_env();
    config::validate_secret_for_env(&config.internal_shared_secret, &config.env_mode)
        .map_err(std::io::Error::other)?;

    if config.env_mode != "dev" {
        onelink_internal_auth::observability_ip_allowlist::require_explicit_env_mode()
            .map_err(std::io::Error::other)?;
    }

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let app = axum::Router::new()
        .merge(health::router())
        .merge(http::routes::router());

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!(service = SERVICE_NAME, port = config.port, "listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
