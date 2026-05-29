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

fn spawn_identity_service(port: u16) -> ServiceGuard {
    let bin = std::env::var("IDENTITY_SERVICE_BIN").unwrap_or_else(|_| {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        format!("{manifest_dir}/../../target/debug/identity-service")
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
fn identity_service_process_level_auth_rejection() {
    if std::env::var("RUN_PROCESS_LEVEL_SMOKE").ok().as_deref() != Some("1") {
        eprintln!("skip: set RUN_PROCESS_LEVEL_SMOKE=1 to run process-level tests (requires identity-service binary)");
        return;
    }

    let port: u16 = 19091;
    let guard = spawn_identity_service(port);

    assert!(
        wait_for_health(port, Duration::from_secs(15)),
        "identity-service did not become healthy on port {port}"
    );

    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .build()
        .unwrap();

    let resp_no_token = client
        .get(format!(
            "http://127.0.0.1:{port}/internal/identity/health-detail"
        ))
        .send()
        .unwrap();
    assert_eq!(
        resp_no_token.status(),
        reqwest::StatusCode::UNAUTHORIZED,
        "internal route without token must return 401"
    );

    let resp_wrong_token = client
        .get(format!(
            "http://127.0.0.1:{port}/internal/identity/health-detail"
        ))
        .header("x-internal-token", "wrong-token-value")
        .send()
        .unwrap();
    assert_eq!(
        resp_wrong_token.status(),
        reqwest::StatusCode::UNAUTHORIZED,
        "internal route with wrong token must return 401"
    );

    let resp_correct_token = client
        .get(format!(
            "http://127.0.0.1:{port}/internal/identity/health-detail"
        ))
        .header("x-internal-token", "onelink-dev-internal-token")
        .send()
        .unwrap();
    assert_eq!(
        resp_correct_token.status(),
        reqwest::StatusCode::OK,
        "internal route with correct token must return 200"
    );

    let body: serde_json::Value = resp_correct_token.json().unwrap();
    assert_eq!(body["env_mode"], "dev");
    assert!(body["uses_argon2"].as_bool().unwrap());

    drop(guard);
}

#[test]
fn identity_service_process_level_register_login_me() {
    if std::env::var("RUN_PROCESS_LEVEL_SMOKE").ok().as_deref() != Some("1") {
        eprintln!("skip: set RUN_PROCESS_LEVEL_SMOKE=1 to run process-level tests");
        return;
    }

    let port: u16 = 19092;
    let guard = spawn_identity_service(port);

    assert!(
        wait_for_health(port, Duration::from_secs(15)),
        "identity-service did not become healthy on port {port}"
    );

    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .build()
        .unwrap();
    let base = format!("http://127.0.0.1:{port}");

    let reg_resp = client
        .post(format!("{base}/api/v1/identity/register"))
        .json(&serde_json::json!({
            "provider": "email",
            "email": "process-level@example.com",
            "password": "process-level-pw-123",
            "primary_region": "US",
            "primary_language": "en"
        }))
        .send()
        .unwrap();
    assert_eq!(reg_resp.status(), reqwest::StatusCode::OK);
    let reg_body: serde_json::Value = reg_resp.json().unwrap();
    let token = reg_body["session"]["token"].as_str().unwrap();
    let user_id = reg_body["user_id"].as_str().unwrap();

    let me_resp = client
        .get(format!("{base}/api/v1/identity/me"))
        .header("authorization", format!("Bearer {token}"))
        .send()
        .unwrap();
    assert_eq!(me_resp.status(), reqwest::StatusCode::OK);
    let me_body: serde_json::Value = me_resp.json().unwrap();
    assert_eq!(me_body["user_id"], user_id);
    assert_eq!(me_body["status"], "active");
    assert_eq!(me_body["primary_region"], "US");
    assert_eq!(me_body["primary_language"], "en");

    let login_resp = client
        .post(format!("{base}/api/v1/identity/login"))
        .json(&serde_json::json!({
            "provider": "email",
            "email": "process-level@example.com",
            "password": "process-level-pw-123"
        }))
        .send()
        .unwrap();
    assert_eq!(login_resp.status(), reqwest::StatusCode::OK);
    let login_body: serde_json::Value = login_resp.json().unwrap();
    assert_eq!(login_body["user_id"], user_id);

    drop(guard);
}

#[test]
fn identity_service_rejects_default_secret_in_staging() {
    if std::env::var("RUN_PROCESS_LEVEL_SMOKE").ok().as_deref() != Some("1") {
        eprintln!("skip: set RUN_PROCESS_LEVEL_SMOKE=1 to run process-level negative tests");
        return;
    }

    let bin = std::env::var("IDENTITY_SERVICE_BIN").unwrap_or_else(|_| {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        format!("{manifest_dir}/../../target/debug/identity-service")
    });

    let mut child = Command::new(&bin)
        .env("PORT", "19093")
        .env("ONELINK_ENV", "staging")
        .env("INTERNAL_SHARED_SECRET", "onelink-dev-internal-token")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {bin}: {e}"));

    let status = child.wait().expect("wait for child");
    assert!(
        !status.success(),
        "identity-service must NOT start with default secret in staging env, but it exited successfully"
    );
}

#[test]
fn identity_service_rejects_short_secret_in_production() {
    if std::env::var("RUN_PROCESS_LEVEL_SMOKE").ok().as_deref() != Some("1") {
        eprintln!("skip: set RUN_PROCESS_LEVEL_SMOKE=1 to run process-level negative tests");
        return;
    }

    let bin = std::env::var("IDENTITY_SERVICE_BIN").unwrap_or_else(|_| {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        format!("{manifest_dir}/../../target/debug/identity-service")
    });

    let mut child = Command::new(&bin)
        .env("PORT", "19094")
        .env("ONELINK_ENV", "production")
        .env("INTERNAL_SHARED_SECRET", "short-secret-only-20chars")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("failed to spawn {bin}: {e}"));

    let status = child.wait().expect("wait for child");
    assert!(
        !status.success(),
        "identity-service must NOT start with short secret in production env, but it exited successfully"
    );
}
