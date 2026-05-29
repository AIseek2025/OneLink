use axum::extract::State as AxumState;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use axum::Router;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{oneshot, RwLock};

struct MockState {
    responses: HashMap<String, Value>,
}

pub struct MockBff {
    url: String,
    state: Arc<RwLock<MockState>>,
    shutdown_handle: Option<tokio::task::JoinHandle<()>>,
}

async fn mock_handler(
    AxumState(state): AxumState<Arc<RwLock<MockState>>>,
    req: axum::extract::Request,
) -> impl IntoResponse {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let key = format!("{} {}", method, path);
    let state_ref = state.read().await;
    if let Some(resp) = state_ref.responses.get(&key) {
        return (StatusCode::OK, Json(resp.clone())).into_response();
    }
    for (registered_key, resp) in &state_ref.responses {
        if let Some((reg_method, reg_path)) = registered_key.split_once(' ') {
            if reg_method == method && path_matches_pattern(&path, reg_path) {
                return (StatusCode::OK, Json(resp.clone())).into_response();
            }
        }
    }
    eprintln!(
        "mock MISS: key='{}', available={:?}",
        key,
        state_ref.responses.keys().collect::<Vec<_>>()
    );
    (
        StatusCode::NOT_FOUND,
        format!("mock: no response for {}", key),
    )
        .into_response()
}

fn path_matches_pattern(actual: &str, pattern: &str) -> bool {
    let actual_parts: Vec<&str> = actual.split('/').collect();
    let pattern_parts: Vec<&str> = pattern.split('/').collect();
    if actual_parts.len() != pattern_parts.len() {
        return false;
    }
    for (a, p) in actual_parts.iter().zip(pattern_parts.iter()) {
        if p.starts_with('{') && p.ends_with('}') {
            continue;
        }
        if a != p {
            return false;
        }
    }
    true
}

impl MockBff {
    pub async fn start() -> Self {
        let state = Arc::new(RwLock::new(MockState {
            responses: HashMap::new(),
        }));

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let url = format!("http://127.0.0.1:{}", port);

        let app = Router::new()
            .fallback(axum::routing::any(mock_handler))
            .with_state(state.clone());

        let (tx, rx) = oneshot::channel::<()>();
        let server = axum::serve(listener, app);
        let handle = tokio::spawn(async move {
            let _ = tx.send(());
            let _ = server.await;
        });

        rx.await.expect("mock bff server start signal");

        Self {
            url,
            state,
            shutdown_handle: Some(handle),
        }
    }

    pub fn url(&self) -> String {
        self.url.clone()
    }

    pub async fn set_response(&self, method: &str, path: &str, _status: u16, body: Value) {
        let key = format!("{} {}", method, path);
        let mut state_guard = self.state.write().await;
        state_guard.responses.insert(key, body);
    }

    pub fn shutdown(&mut self) {
        if let Some(handle) = self.shutdown_handle.take() {
            handle.abort();
        }
    }
}
