//! OneLink `dm-service` — direct messaging threads and messages.

pub mod config;
pub mod errors;
pub mod health;
pub mod http;
pub mod store;

use std::net::SocketAddr;

const SERVICE_NAME: &str = "dm-service";

pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = config::Config::from_env();
    config::validate_secret_for_env(&config.internal_shared_secret, &config.env_mode)
        .map_err(std::io::Error::other)?;

    if config.env_mode != "dev" {
        onelink_internal_auth::observability_ip_allowlist::require_explicit_env_mode()
            .map_err(std::io::Error::other)?;
    }

    if config.env_mode != "dev" && config.database_url.is_none() {
        return Err(format!(
            "dm-service: FATAL: DATABASE_URL is not set in non-dev environment (ONELINK_ENV={}). \
             Persistent storage is mandatory outside dev. Set DATABASE_URL or set ONELINK_ENV=dev for local development.",
            config.env_mode
        ).into());
    }

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let backend = store::PersistenceBackend::from_config(config.database_url.as_deref()).await;
    let pg = match &backend {
        store::PersistenceBackend::Postgres(pg_store) => {
            Some(std::sync::Arc::new(pg_store.clone()))
        }
        store::PersistenceBackend::InMemory => None,
    };

    let state = http::routes::DmState::new(config.clone(), pg);

    let app = axum::Router::new()
        .merge(health::router())
        .merge(http::routes::router(state));

    let bind_host = if config.env_mode == "dev" {
        "0.0.0.0"
    } else {
        &config.internal_bind_addr
    };
    let addr = SocketAddr::from((bind_host.parse::<std::net::IpAddr>().unwrap(), config.port));
    tracing::info!(service = SERVICE_NAME, port = config.port, %backend, "listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
