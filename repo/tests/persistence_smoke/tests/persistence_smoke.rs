use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
#[ignore = "requires RUN_PERSISTENCE_SMOKE=1 and Docker Postgres with DDL applied"]
fn persistence_smoke_e2e() {
    let script = repo_root().join("scripts/smoke-persistence-e2e.sh");
    let script = script
        .canonicalize()
        .unwrap_or_else(|_| panic!("smoke script not found: {}", script.display()));
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://onelink:onelink-local-dev-password@127.0.0.1:5432/onelink".to_string()
    });
    let status = Command::new("bash")
        .arg(script.as_os_str())
        .env("DATABASE_URL", &database_url)
        .env("INTERNAL_SHARED_SECRET", "onelink-dev-internal-token")
        .env("ONELINK_ENV", "dev")
        .status()
        .expect("spawn bash for persistence smoke script");
    assert!(
        status.success(),
        "smoke-persistence-e2e.sh exited {:?}",
        status.code()
    );
}
