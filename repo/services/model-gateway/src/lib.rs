//! OneLink `model-gateway` service — capacity, cost & reliability layer.

pub mod budget;
pub mod bulkhead;
pub mod cache;
pub mod circuit_breaker;
pub mod compliance;
pub mod config;
pub mod cost_metrics;
pub mod errors;
pub mod fallback;
pub mod health;
pub mod http;
pub mod locale;
pub mod region;

use std::net::SocketAddr;
use std::sync::Arc;

use budget::TokenBudgetTracker;
use bulkhead::CapabilityBulkheads;
use cache::ResponseCache;
use circuit_breaker::CircuitBreakerRegistry;
use compliance::CompliancePolicy;
use cost_metrics::CostMetrics;
use fallback::FallbackConfig;
use locale::TerminologyRegistry;
use region::RegionRegistry;

const SERVICE_NAME: &str = "model-gateway";

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

    let state = Arc::new(http::routes::GatewayState {
        config: config.clone(),
        bulkheads: CapabilityBulkheads::with_defaults(),
        circuit_breakers: CircuitBreakerRegistry::with_default_capabilities(),
        budget_tracker: TokenBudgetTracker::with_default_capabilities(),
        cache: ResponseCache::with_defaults(),
        cost_metrics: CostMetrics::new(),
        fallback_config: FallbackConfig::default(),
        terminology: TerminologyRegistry::new(),
        compliance: CompliancePolicy::new(),
        regions: RegionRegistry::new(),
    });

    let app = axum::Router::new()
        .merge(health::router(state.clone()))
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
