use std::process::{Child, Command};
use std::time::Duration;

fn wait_for_health(port: u16, timeout: Duration) -> bool {
    let start = std::time::Instant::now();
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .no_proxy()
        .build()
        .unwrap();
    while start.elapsed() < timeout {
        if client
            .get(format!("http://127.0.0.1:{port}/health"))
            .send()
            .is_ok()
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

struct ServiceGuard {
    child: Child,
    #[allow(dead_code)]
    port: u16,
}

impl Drop for ServiceGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn spawn_context_service(port: u16) -> ServiceGuard {
    let bin = std::env::var("CONTEXT_SERVICE_BIN").unwrap_or_else(|_| {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        format!("{manifest_dir}/../../target/debug/context-service")
    });

    let child = Command::new(&bin)
        .env("PORT", port.to_string())
        .env("ONELINK_ENV", "dev")
        .env("INTERNAL_SHARED_SECRET", "onelink-dev-internal-token")
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {bin}: {e}"));

    ServiceGuard { child, port }
}

#[test]
fn context_service_process_level_auth_rejection() {
    if std::env::var("RUN_PROCESS_LEVEL_SMOKE").ok().as_deref() != Some("1") {
        eprintln!("skip: set RUN_PROCESS_LEVEL_SMOKE=1 to run process-level tests (requires context-service binary)");
        return;
    }

    let port: u16 = 19093;
    let guard = spawn_context_service(port);

    assert!(
        wait_for_health(port, Duration::from_secs(15)),
        "context-service did not become healthy on port {port}"
    );

    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .build()
        .unwrap();

    let event_body = serde_json::json!({
        "event_id": "evt-process-test-001",
        "event_name": "chat.user_message.created.v1",
        "event_version": "v1",
        "occurred_at": "2026-05-17T23:00:00.000Z",
        "producer": "process-level-test",
        "payload": {"user_id": "u-test", "conversation_id": "conv-test", "content": "hello"}
    })
    .to_string();

    let resp_no_token = client
        .post(format!("http://127.0.0.1:{port}/internal/events/receive"))
        .header("content-type", "application/json")
        .body(event_body.clone())
        .send()
        .unwrap();
    assert_eq!(
        resp_no_token.status(),
        reqwest::StatusCode::UNAUTHORIZED,
        "internal route without token must return 401"
    );

    let resp_wrong_token = client
        .post(format!("http://127.0.0.1:{port}/internal/events/receive"))
        .header("content-type", "application/json")
        .header("x-internal-token", "wrong-token-value")
        .body(event_body.clone())
        .send()
        .unwrap();
    assert_eq!(
        resp_wrong_token.status(),
        reqwest::StatusCode::UNAUTHORIZED,
        "internal route with wrong token must return 401"
    );

    let resp_correct_token = client
        .post(format!("http://127.0.0.1:{port}/internal/events/receive"))
        .header("content-type", "application/json")
        .header("x-internal-token", "onelink-dev-internal-token")
        .body(event_body)
        .send()
        .unwrap();
    assert_eq!(
        resp_correct_token.status(),
        reqwest::StatusCode::ACCEPTED,
        "internal route with correct token must return 202, got {}",
        resp_correct_token.status()
    );

    let build_body = serde_json::json!({
        "user_id": "u-test",
        "agent_id": "lumi",
        "conversation_id": "conv-test",
        "input": "hello",
        "task_type": "chat",
        "max_tokens": 512,
        "memory_limit": 10,
        "summary_limit": 5
    })
    .to_string();

    let resp_build_no_token = client
        .post(format!("http://127.0.0.1:{port}/internal/context/build"))
        .header("content-type", "application/json")
        .body(build_body.clone())
        .send()
        .unwrap();
    assert_eq!(
        resp_build_no_token.status(),
        reqwest::StatusCode::UNAUTHORIZED,
        "build_context without token must return 401"
    );

    let resp_build_correct = client
        .post(format!("http://127.0.0.1:{port}/internal/context/build"))
        .header("content-type", "application/json")
        .header("x-internal-token", "onelink-dev-internal-token")
        .body(build_body)
        .send()
        .unwrap();
    assert_eq!(
        resp_build_correct.status(),
        reqwest::StatusCode::OK,
        "build_context with correct token must return 200"
    );

    let bad_body = r#"{"not_an_envelope":true}"#;
    let resp_bad_body_no_token = client
        .post(format!("http://127.0.0.1:{port}/internal/events/receive"))
        .header("content-type", "application/json")
        .body(bad_body)
        .send()
        .unwrap();
    assert_eq!(
        resp_bad_body_no_token.status(),
        reqwest::StatusCode::UNAUTHORIZED,
        "malformed body without token must still return 401, not 400"
    );

    drop(guard);
}

#[test]
fn context_service_rejects_default_secret_in_staging() {
    if std::env::var("RUN_PROCESS_LEVEL_SMOKE").ok().as_deref() != Some("1") {
        eprintln!("skip: set RUN_PROCESS_LEVEL_SMOKE=1 to run process-level negative tests");
        return;
    }

    let bin = std::env::var("CONTEXT_SERVICE_BIN").unwrap_or_else(|_| {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        format!("{manifest_dir}/../../target/debug/context-service")
    });

    let mut child = Command::new(&bin)
        .env("PORT", "19095")
        .env("ONELINK_ENV", "staging")
        .env("INTERNAL_SHARED_SECRET", "onelink-dev-internal-token")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {bin}: {e}"));

    let status = child.wait().expect("wait for child");
    assert!(
        !status.success(),
        "context-service must NOT start with default secret in staging env, but it exited successfully"
    );
}

#[test]
fn context_service_rejects_short_secret_in_production() {
    if std::env::var("RUN_PROCESS_LEVEL_SMOKE").ok().as_deref() != Some("1") {
        eprintln!("skip: set RUN_PROCESS_LEVEL_SMOKE=1 to run process-level negative tests");
        return;
    }

    let bin = std::env::var("CONTEXT_SERVICE_BIN").unwrap_or_else(|_| {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        format!("{manifest_dir}/../../target/debug/context-service")
    });

    let mut child = Command::new(&bin)
        .env("PORT", "19096")
        .env("ONELINK_ENV", "production")
        .env("INTERNAL_SHARED_SECRET", "short-secret-only-20chars")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {bin}: {e}"));

    let status = child.wait().expect("wait for child");
    assert!(
        !status.success(),
        "context-service must NOT start with short secret in production env, but it exited successfully"
    );
}
