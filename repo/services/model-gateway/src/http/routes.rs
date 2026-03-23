//! Minimal internal invoke route for local vertical slice.

use axum::{
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

pub fn router() -> Router {
    Router::new()
        .route(
            "/api/v1/placeholder",
            get(|| async { "model-gateway skeleton" }),
        )
        .route("/internal/v1/invoke", post(invoke))
}

async fn invoke(Json(request): Json<InvokeRequest>) -> Json<InvokeResponse> {
    let trace_suffix = request
        .trace_id
        .as_deref()
        .map(|value| format!(" trace_id={value}"))
        .unwrap_or_default();
    let response_text = match request.capability_name.as_str() {
        "chat.respond" => {
            let context_preview = request
                .payload
                .get("context")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({}));
            format!(
                "Lumi reply (skeleton): context={}{}",
                context_preview, trace_suffix
            )
        }
        other => format!("model-gateway skeleton handled capability={other}{trace_suffix}"),
    };

    Json(InvokeResponse {
        capability_name: request.capability_name,
        model_id: "mock-model-v1".to_string(),
        output_text: response_text,
    })
}

#[derive(Debug, Deserialize)]
struct InvokeRequest {
    capability_name: String,
    trace_id: Option<String>,
    #[serde(default)]
    payload: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct InvokeResponse {
    capability_name: String,
    model_id: String,
    output_text: String,
}
