//! OneLink `context-service` — Memory Compute Layer (skeleton).
//!
//! MVP 只做三件事：memory extraction、memory distillation、context assembly。
//! 内部逻辑模块（单进程）：memory-extractor、memory-distiller、context-builder、
//! memory-store、vector-index、task-router（logical only）。
//! 向量检索超时或不可用时须降级：working memory + 最近 `memory_summaries`，跳过 persistent 向量召回（见 OneLink/docs/archive/rules-legacy-2026-05-15/Rules/19）。

pub mod app_state;
pub mod config;
pub mod context_builder;
pub mod errors;
pub mod evidence;
pub mod health;
pub mod http;
pub mod l1_policy;
pub mod memory_distiller;
pub mod memory_extractor;
pub mod memory_store;
pub mod policy;
pub mod runtime;
pub mod store;
pub mod task_router;
pub mod vector_index;

use std::net::SocketAddr;

const SERVICE_NAME: &str = "context-service";

pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = config::Config::from_env();
    config::validate_secret_for_env(&config.internal_shared_secret, &config.env_mode)
        .map_err(|e| format!("secret validation: {e}"))?;

    if config.env_mode != "dev" {
        onelink_internal_auth::observability_ip_allowlist::require_explicit_env_mode()
            .map_err(|e| format!("observability IP allowlist: {e}"))?;
    }

    if config.env_mode != "dev" && config.database_url.is_none() {
        return Err(format!(
            "context-service: FATAL: DATABASE_URL is not set in non-dev environment (ONELINK_ENV={}). \
             Persistent storage is mandatory outside dev. Set DATABASE_URL or set ONELINK_ENV=dev for local development.",
            config.env_mode
        ).into());
    }

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let mut policy_store = policy::PolicyConfigStore {
        default_reply_style: config.default_reply_style.clone(),
        ..Default::default()
    };
    let store = store::MemoryBackend::connect(&config).await;
    if store.is_postgres() {
        tracing::info!("context-service memory store: Postgres (DATABASE_URL set)");
        if let store::MemoryBackend::Postgres(pg) = &store {
            match pg.load_policy_overrides().await {
                Ok(o) => {
                    policy_store.apply_db_overrides(o);
                    tracing::info!(
                        graph_enabled = policy_store.graph_enabled,
                        rerank_enabled = policy_store.rerank_enabled,
                        "policy_configs overrides applied (best-effort)"
                    );
                }
                Err(e) => tracing::warn!(error = %e, "policy_configs load failed; using defaults"),
            }
        }
    } else {
        tracing::info!("context-service memory store: in-memory fallback (DATABASE_URL unset)");
    }
    let state = app_state::ContextAppState::new(policy_store, config.clone(), store);
    let app = axum::Router::new()
        .merge(health::router())
        .merge(http::routes::router(state));

    let bind_host = if config.env_mode == "dev" {
        "0.0.0.0"
    } else {
        &config.internal_bind_addr
    };
    let addr = SocketAddr::from((bind_host.parse::<std::net::IpAddr>().unwrap(), config.port));
    tracing::info!(service = SERVICE_NAME, port = config.port, "listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
