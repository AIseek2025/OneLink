//! OneLink `context-service` — Memory Compute Layer (skeleton).
//!
//! MVP 只做三件事：memory extraction、memory distillation、context assembly。
//! 内部逻辑模块（单进程）：memory-extractor、memory-distiller、context-builder、
//! memory-store、vector-index、task-router（logical only）。
//! 向量检索超时或不可用时须降级：working memory + 最近 `memory_summaries`，跳过 persistent 向量召回（见 Rules/19）。

pub mod app_state;
pub mod config;
pub mod context_builder;
pub mod errors;
pub mod health;
pub mod http;
pub mod l1_policy;
pub mod memory_distiller;
pub mod memory_extractor;
pub mod memory_store;
pub mod policy;
pub mod runtime;
pub mod task_router;
pub mod vector_index;

use std::net::SocketAddr;

const SERVICE_NAME: &str = "context-service";

/// Minimal HTTP server: health + internal API placeholders (no DB / Kafka / Qdrant / LLM).
pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = config::Config::from_env();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let policy_store = policy::PolicyConfigStore {
        default_reply_style: config.default_reply_style.clone(),
        ..Default::default()
    };
    let state = app_state::ContextAppState::new(policy_store, config.clone());
    let app = axum::Router::new()
        .merge(health::router())
        .merge(http::routes::router(state));

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!(service = SERVICE_NAME, port = config.port, "listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
