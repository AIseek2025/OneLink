pub mod config;
pub mod errors;
pub mod health;
pub mod http;
pub mod store;

use std::net::SocketAddr;
use std::sync::Arc;

const SERVICE_NAME: &str = "identity-service";

pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cfg = config::Config::from_env();
    config::validate_secret_for_env(&cfg.internal_shared_secret, &cfg.env_mode)
        .map_err(std::io::Error::other)?;

    if cfg.env_mode != "dev" {
        onelink_internal_auth::observability_ip_allowlist::require_explicit_env_mode()
            .map_err(std::io::Error::other)?;
    }

    if cfg.env_mode != "dev" && cfg.database_url.is_none() {
        return Err(format!(
            "identity-service: FATAL: DATABASE_URL is not set in non-dev environment (ONELINK_ENV={}). \
             Persistent storage is mandatory outside dev. Set DATABASE_URL or set ONELINK_ENV=dev for local development.",
            cfg.env_mode
        ).into());
    }

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let persistence = store::PersistenceBackend::from_config(cfg.database_url.as_deref()).await;
    let identity_state = match &persistence {
        store::PersistenceBackend::Postgres(pg) => Arc::new(http::routes::IdentityState::with_pg(
            cfg.clone(),
            Arc::new(pg.clone()),
        )),
        store::PersistenceBackend::InMemory(_) => {
            Arc::new(http::routes::IdentityState::new(cfg.clone()))
        }
    };
    let _persistence = persistence;
    let app = axum::Router::new()
        .merge(health::router(identity_state.clone()))
        .merge(http::routes::router(identity_state));

    let bind_host = if cfg.env_mode == "dev" {
        "0.0.0.0"
    } else {
        &cfg.internal_bind_addr
    };
    let addr = SocketAddr::from((bind_host.parse::<std::net::IpAddr>().unwrap(), cfg.port));
    tracing::info!(service = SERVICE_NAME, port = cfg.port, "listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
