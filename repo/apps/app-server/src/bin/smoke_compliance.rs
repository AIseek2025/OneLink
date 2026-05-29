use std::net::TcpListener;

use axum::response::IntoResponse;
use axum::Router;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

struct StubResponse {
    status: axum::http::StatusCode,
    body: Value,
}

struct StubState {
    responses: HashMap<String, StubResponse>,
    requests: Vec<Value>,
}

async fn stub_handler(
    axum::extract::State(state): axum::extract::State<Arc<RwLock<StubState>>>,
    req: axum::extract::Request,
) -> axum::response::Response {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let path_and_query = req
        .uri()
        .path_and_query()
        .map(|value| value.as_str().to_string())
        .unwrap_or_else(|| path.clone());
    let key = format!("{} {}", method, path);
    {
        let mut state_mut = state.write().await;
        state_mut.requests.push(json!({
            "method": method,
            "path": path,
            "path_and_query": path_and_query,
            "authorization_present": req.headers().contains_key("authorization"),
            "accept_language": req.headers().get("accept-language").and_then(|v| v.to_str().ok()),
            "user_region": req.headers().get("x-user-region").and_then(|v| v.to_str().ok()),
        }));
    }
    let state_ref = state.read().await;
    if let Some(resp) = state_ref.responses.get(&key) {
        return (resp.status, axum::Json(resp.body.clone())).into_response();
    }
    for (registered_key, resp) in &state_ref.responses {
        if let Some((reg_method, reg_path)) = registered_key.split_once(' ') {
            if reg_method == method.as_str() {
                let actual_parts: Vec<&str> = path.split('/').collect();
                let pattern_parts: Vec<&str> = reg_path.split('/').collect();
                if actual_parts.len() == pattern_parts.len() {
                    let matches = actual_parts
                        .iter()
                        .zip(pattern_parts.iter())
                        .all(|(a, p)| p.starts_with('{') && p.ends_with('}') || a == p);
                    if matches {
                        return (resp.status, axum::Json(resp.body.clone())).into_response();
                    }
                }
            }
        }
    }
    (
        axum::http::StatusCode::NOT_FOUND,
        format!("stub: no response for {}", key),
    )
        .into_response()
}

struct StubService {
    url: String,
    state: Arc<RwLock<StubState>>,
    _handle: tokio::task::JoinHandle<()>,
}

impl StubService {
    async fn start() -> Self {
        let state = Arc::new(RwLock::new(StubState {
            responses: HashMap::new(),
            requests: Vec::new(),
        }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let url = format!("http://127.0.0.1:{}", port);
        let app = Router::new()
            .fallback(axum::routing::any(stub_handler))
            .with_state(state.clone());
        let handle = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        Self {
            url,
            state,
            _handle: handle,
        }
    }

    async fn set_response(&self, method: &str, path: &str, body: Value) {
        let key = format!("{} {}", method, path);
        self.state.write().await.responses.insert(
            key,
            StubResponse {
                status: axum::http::StatusCode::OK,
                body,
            },
        );
    }

    async fn request_log(&self) -> Vec<Value> {
        self.state.read().await.requests.clone()
    }
}

fn find_available_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

#[tokio::main]
async fn main() {
    eprintln!("[smoke] Starting local upstream service stubs...");
    let identity_service = StubService::start().await;
    let profile_service = StubService::start().await;
    eprintln!(
        "[smoke] Identity stub at {}, profile stub at {}",
        identity_service.url, profile_service.url
    );

    identity_service
        .set_response(
            "POST",
            "/api/v1/identity/login",
            json!({
                "user_id": "00000000-0000-0000-0000-000000000001",
                "access_token": "smoke-test-token",
                "refresh_token": "smoke-refresh",
                "expires_at": "2026-06-28T00:00:00Z",
                "first_run": false
            }),
        )
        .await;

    identity_service
        .set_response(
            "POST",
            "/api/v1/identity/session/refresh",
            json!({
                "access_token": "smoke-test-token",
                "refresh_token": "smoke-refresh",
                "expires_at": "2026-06-28T00:00:00Z"
            }),
        )
        .await;

    identity_service
        .set_response(
            "GET",
            "/api/v1/identity/me",
            json!({
                "user_id": "00000000-0000-0000-0000-000000000001",
                "nickname": "SmokeUser",
                "avatar_url": null,
                "first_run": false,
                "locale": "zh-CN",
                "region": "CN"
            }),
        )
        .await;

    profile_service
        .set_response(
            "GET",
            "/api/v1/profile/me",
            json!({
                "nickname": "SmokeUser",
                "locale": "zh-CN",
                "region": "CN",
                "timezone": "Asia/Shanghai",
                "content_language": "zh-CN",
                "notification_language": "zh-CN",
                "notifications_enabled": true
            }),
        )
        .await;

    profile_service
        .set_response(
            "GET",
            "/api/v1/profile/me/completion",
            json!({
                "pending_facts": [],
                "completion_score": 1.0
            }),
        )
        .await;

    profile_service
        .set_response(
            "GET",
            "/api/v1/profile/me/compliance",
            json!({
                "user_id": "00000000-0000-0000-0000-000000000001",
                "data_export_available": true,
                "data_delete_available": true,
                "data_correction_available": true,
                "pending_requests": [],
                "profile_facts": [{"fact_id": "f1", "fact_text": "likes coffee"}],
                "memory_summaries": [],
                "key_artifacts": [],
                "settings": {"locale": "zh-CN"},
                "consent_records": [{"type": "privacy_policy", "version": "1.0", "accepted_at": "2026-05-25"}]
            }),
        )
        .await;

    profile_service
        .set_response(
            "POST",
            "/api/v1/profile/me/compliance/export",
            json!({
                "request_id": "00000000-0000-0000-0000-000000000099",
                "action_type": "export",
                "status": "processing",
                "created_at": "2026-05-25T12:00:00Z"
            }),
        )
        .await;

    profile_service
        .set_response(
            "POST",
            "/api/v1/profile/me/compliance/delete",
            json!({
                "request_id": "00000000-0000-0000-0000-000000000100",
                "action_type": "delete",
                "status": "processing",
                "created_at": "2026-05-25T12:00:00Z"
            }),
        )
        .await;

    profile_service
        .set_response(
            "POST",
            "/api/v1/profile/me/compliance/correction",
            json!({
                "request_id": "00000000-0000-0000-0000-000000000101",
                "action_type": "correction",
                "status": "processing",
                "created_at": "2026-05-25T12:00:00Z"
            }),
        )
        .await;

    eprintln!("[smoke] Starting real BFF service...");
    let bff_port = find_available_port();
    let bff_addr = format!("127.0.0.1:{}", bff_port);
    let bff_url = format!("http://{}", bff_addr);
    let bff_config = bff::config::Config {
        port: bff_port,
        identity_service_base_url: identity_service.url.clone(),
        ai_chat_service_base_url: identity_service.url.clone(),
        question_service_base_url: identity_service.url.clone(),
        profile_service_base_url: profile_service.url.clone(),
        match_service_base_url: identity_service.url.clone(),
        safety_service_base_url: identity_service.url.clone(),
        dm_service_base_url: identity_service.url.clone(),
        admin_service_base_url: identity_service.url.clone(),
        internal_shared_secret: "smoke-internal-secret".to_string(),
        env_mode: "dev".to_string(),
        cors_allowed_origins: "http://localhost:3000,http://localhost:5173".to_string(),
        default_locale: "zh-CN".to_string(),
        default_region: "CN".to_string(),
        supported_locales: vec!["zh-CN".to_string(), "en-US".to_string()],
        supported_regions: vec!["CN".to_string(), "US".to_string()],
        supported_timezones: vec!["Asia/Shanghai".to_string(), "America/New_York".to_string()],
        translations_source: "smoke_inline".to_string(),
        translations_inline:
            r#"{"zh-CN":{"app.title":"OneLink","btn.submit":"提交"},"en-US":{"app.title":"OneLink","btn.submit":"Submit"}}"#
                .to_string(),
    };
    let bff_state = bff::http::routes::BffState::new(bff_config);
    let bff_listener = tokio::net::TcpListener::bind(&bff_addr).await.unwrap();
    let bff_router: Router = bff::http::routes::router(bff_state);
    let bff_handle = tokio::spawn(async move {
        let _ = axum::serve(bff_listener, bff_router).await;
    });
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    eprintln!("[smoke] Verification mode: real local BFF + app server + local upstream stubs");
    eprintln!("[smoke] Real BFF at {}", bff_url);

    eprintln!("[smoke] Starting app server...");
    let app_port = find_available_port();
    let app_addr = format!("127.0.0.1:{}", app_port);

    let config = onelink_app_server::config::AppConfig {
        bff_base_url: bff_url.clone(),
        port: app_port,
        ..Default::default()
    };

    let state = onelink_app_server::state::AppState::new(config);
    let app: Router = onelink_app_server::router::router(state);

    let listener = tokio::net::TcpListener::bind(&app_addr).await.unwrap();
    let server_handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    eprintln!("[smoke] App server at http://{}", app_addr);

    let base = format!("http://{}", app_addr);
    let client = reqwest::Client::new();

    let mut transcript: Vec<serde_json::Value> = Vec::new();

    macro_rules! smoke_call {
        ($label:expr, $method:expr, $path:expr, $headers:expr, $body:expr) => {{
            let mut req = client.request($method, &format!("{}{}", base, $path));
            for (k, v) in $headers {
                req = req.header(k, v);
            }
            if let Some(b) = $body {
                req = req.json(&b);
            }
            let resp = req.send().await.unwrap();
            let status = resp.status().as_u16();
            let resp_body: Value = resp.json().await.unwrap_or(Value::Null);
            let entry = json!({
                "label": $label,
                "request": {
                    "method": stringify!($method).replace("reqwest::Method::", ""),
                    "path": $path,
                    "headers": $headers,
                    "body": $body,
                },
                "response": {
                    "status": status,
                    "body": resp_body,
                }
            });
            eprintln!(
                "[smoke] {} => status={} body={}",
                $label,
                status,
                serde_json::to_string(&resp_body).unwrap_or_default().chars().take(120).collect::<String>()
            );
            transcript.push(entry);
            (status, resp_body)
        }};
    }

    let (_login_status, login_body) = smoke_call!(
        "auth-login",
        reqwest::Method::POST,
        "/api/v1/bff/auth/login",
        vec![("Accept-Language", "zh-CN")],
        Some(json!({"phone": "+8613800138000", "code": "123456"}))
    );
    let access_token = login_body
        .get("access_token")
        .and_then(|v| v.as_str())
        .unwrap_or("smoke-test-token")
        .to_string();
    let auth_header = format!("Bearer {}", access_token);

    smoke_call!(
        "region-gate-CN",
        reqwest::Method::GET,
        "/api/v1/bff/region/gate",
        vec![("X-User-Region", "CN"), ("Accept-Language", "zh-CN")],
        None as Option<Value>
    );

    smoke_call!(
        "region-gate-US",
        reqwest::Method::GET,
        "/api/v1/bff/region/gate",
        vec![("X-User-Region", "US"), ("Accept-Language", "en")],
        None as Option<Value>
    );

    smoke_call!(
        "region-gate-EU",
        reqwest::Method::GET,
        "/api/v1/bff/region/gate",
        vec![("X-User-Region", "EU"), ("Accept-Language", "en")],
        None as Option<Value>
    );

    smoke_call!(
        "settings-locale-registry",
        reqwest::Method::GET,
        "/api/v1/bff/settings/locale",
        vec![
            ("Authorization", auth_header.as_str()),
            ("Accept-Language", "zh-CN"),
        ],
        None as Option<Value>
    );

    smoke_call!(
        "i18n-registry",
        reqwest::Method::GET,
        "/api/v1/bff/i18n/registry",
        vec![("Accept-Language", "zh-CN")],
        None as Option<Value>
    );

    smoke_call!(
        "i18n-translate-export-title-en",
        reqwest::Method::GET,
        "/api/v1/bff/i18n/translate?key=privacy.data_export.title&locale=en",
        vec![("Accept-Language", "en")],
        None as Option<Value>
    );

    smoke_call!(
        "i18n-translate-export-title-zh",
        reqwest::Method::GET,
        "/api/v1/bff/i18n/translate?key=privacy.data_export.title&locale=zh-CN",
        vec![("Accept-Language", "zh-CN")],
        None as Option<Value>
    );

    smoke_call!(
        "compliance-summary-CN",
        reqwest::Method::GET,
        "/api/v1/bff/compliance/summary",
        vec![
            ("Authorization", auth_header.as_str()),
            ("X-User-Region", "CN"),
            ("Accept-Language", "zh-CN"),
        ],
        None as Option<Value>
    );

    smoke_call!(
        "compliance-summary-EU",
        reqwest::Method::GET,
        "/api/v1/bff/compliance/summary",
        vec![
            ("Authorization", auth_header.as_str()),
            ("X-User-Region", "EU"),
            ("Accept-Language", "en"),
        ],
        None as Option<Value>
    );

    smoke_call!(
        "compliance-export-CN-allowed",
        reqwest::Method::POST,
        "/api/v1/bff/compliance/export",
        vec![
            ("Authorization", auth_header.as_str()),
            ("X-User-Region", "CN"),
            ("Accept-Language", "zh-CN"),
        ],
        Some(json!({"action_type": "export", "export_format": "json"}))
    );

    smoke_call!(
        "compliance-export-EU-blocked",
        reqwest::Method::POST,
        "/api/v1/bff/compliance/export",
        vec![
            ("Authorization", auth_header.as_str()),
            ("X-User-Region", "EU"),
            ("Accept-Language", "en"),
        ],
        Some(json!({"action_type": "export", "export_format": "json"}))
    );

    smoke_call!(
        "compliance-delete-CN-allowed",
        reqwest::Method::POST,
        "/api/v1/bff/compliance/delete",
        vec![
            ("Authorization", auth_header.as_str()),
            ("X-User-Region", "CN"),
            ("Accept-Language", "zh-CN"),
        ],
        Some(json!({"action_type": "delete", "scope": "all"}))
    );

    smoke_call!(
        "compliance-delete-EU-blocked",
        reqwest::Method::POST,
        "/api/v1/bff/compliance/delete",
        vec![
            ("Authorization", auth_header.as_str()),
            ("X-User-Region", "EU"),
            ("Accept-Language", "en"),
        ],
        Some(json!({"action_type": "delete", "scope": "all"}))
    );

    smoke_call!(
        "compliance-correction-CN-allowed",
        reqwest::Method::POST,
        "/api/v1/bff/compliance/correction",
        vec![
            ("Authorization", auth_header.as_str()),
            ("X-User-Region", "CN"),
            ("Accept-Language", "zh-CN"),
        ],
        Some(json!({"action_type": "correction", "field_name": "nickname"}))
    );

    smoke_call!(
        "compliance-correction-EU-blocked",
        reqwest::Method::POST,
        "/api/v1/bff/compliance/correction",
        vec![
            ("Authorization", auth_header.as_str()),
            ("X-User-Region", "EU"),
            ("Accept-Language", "en"),
        ],
        Some(json!({"action_type": "correction", "field_name": "nickname"}))
    );

    let identity_requests = identity_service.request_log().await;
    let profile_requests = profile_service.request_log().await;
    eprintln!(
        "[smoke] Upstream requests observed: identity={} profile={}",
        identity_requests.len(),
        profile_requests.len()
    );

    server_handle.abort();
    bff_handle.abort();

    let transcript_json = serde_json::to_string_pretty(&json!({
        "smoke_test": "phase10_i18n_compliance",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "verification_mode": "real_local_bff_with_stubbed_identity_profile_upstreams",
        "topology": {
            "identity_service": identity_service.url,
            "profile_service": profile_service.url,
            "bff_service": bff_url,
            "app_server": format!("http://{}", app_addr),
        },
        "total_requests": transcript.len(),
        "transcript": transcript,
        "upstream_request_log": {
            "identity_service": identity_requests,
            "profile_service": profile_requests,
        }
    }))
    .unwrap();

    println!("{}", transcript_json);

    eprintln!("[smoke] Done. Total requests: {}", transcript_json.len());
}
