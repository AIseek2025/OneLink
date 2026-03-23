//! Placeholder API routes — replace with gateway forwarding logic.

use axum::{routing::get, Router};

pub fn router() -> Router {
    Router::new().route(
        "/api/v1/placeholder",
        get(|| async { "match-service skeleton" }),
    )
}
