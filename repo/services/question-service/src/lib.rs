//! OneLink `question-service` — Phase C MVP questionnaire API.

pub mod config;
pub mod errors;
pub mod health;
pub mod http;
pub mod store;

use std::net::SocketAddr;
use std::sync::Arc;

use http::routes::AppState;

const SERVICE_NAME: &str = "question-service";

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
            "question-service: FATAL: DATABASE_URL is not set in non-dev environment (ONELINK_ENV={}). \
             Persistent storage is mandatory outside dev. Set DATABASE_URL or set ONELINK_ENV=dev for local development.",
            config.env_mode
        ).into());
    }

    let port = config.port;
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let pg = match &config.database_url {
        Some(url) if !url.trim().is_empty() => match store::PostgresStore::connect(url).await {
            Ok(pg) => {
                if let Err(e) = pg.ensure_seed_catalog().await {
                    tracing::warn!(error = %e, "failed to seed question_catalog");
                }
                Some(Arc::new(pg))
            }
            Err(e) => {
                tracing::error!(error = %e, "question-service: Postgres connect failed — refusing silent in-memory fallback in shared environment");
                panic!(
                    "question-service: FATAL: Postgres connect failed (DATABASE_URL was set). \
                     Silent in-memory fallback is forbidden for shared environments. \
                     Fix the database connection or unset DATABASE_URL to explicitly use dev-only in-memory mode. \
                     Error: {e}"
                );
            }
        },
        _ => None,
    };

    let bind_host = if config.env_mode == "dev" {
        "0.0.0.0"
    } else {
        &config.internal_bind_addr
    };
    let addr = SocketAddr::from((bind_host.parse::<std::net::IpAddr>().unwrap(), port));

    let state = Arc::new(AppState {
        config,
        client: reqwest::Client::new(),
        store: Arc::new(store::QuestionStore::new()),
        pg,
    });

    let app = axum::Router::new()
        .merge(health::router())
        .merge(http::routes::router(state));

    tracing::info!(service = SERVICE_NAME, port = port, "listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
