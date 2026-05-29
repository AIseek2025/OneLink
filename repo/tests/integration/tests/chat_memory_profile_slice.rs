//! 纵切面验证壳：默认不连网、不启服务。
//!
//! 运行方式：
//! - `cargo test -p integration_chat_memory_profile_slice` — 始终通过（占位）。
//! - 六服务已启动后：`RUN_SLICE_HTTP_SMOKE=1 cargo test -p integration_chat_memory_profile_slice slice_http_smoke -- --exact --nocapture`
//! - Phase C 问卷纵切（含 BFF onboarding + 轮询式问卷 smoke）：`RUN_SLICE_QUESTIONNAIRE_SMOKE=1 cargo test -p integration_chat_memory_profile_slice slice_questionnaire_http_smoke -- --exact --nocapture`
//!
//! 等价 shell：自 `tests/integration/` 起算 `../../scripts/smoke-chat-memory-profile.sh`（chat 纵切）、`../../scripts/smoke-questionnaire-profile.sh`（Phase C 问卷）。

use std::path::PathBuf;
use std::process::Command;

#[test]
fn slice_docs_point_to_smoke_script() {
    let md = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("CHAT_MEMORY_PROFILE_SLICE.md");
    assert!(md.is_file(), "missing verification doc: {}", md.display());
}

#[test]
fn slice_http_smoke() {
    if std::env::var("RUN_SLICE_HTTP_SMOKE").ok().as_deref() != Some("1") {
        eprintln!("skip slice_http_smoke: set RUN_SLICE_HTTP_SMOKE=1 and start 6 services (see CHAT_MEMORY_PROFILE_SLICE.md)");
        return;
    }
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/smoke-chat-memory-profile.sh");
    let script = script
        .canonicalize()
        .unwrap_or_else(|_| panic!("smoke script not found: {}", script.display()));
    let status = Command::new("bash")
        .arg(script.as_os_str())
        .status()
        .expect("spawn bash for smoke script");
    assert!(
        status.success(),
        "smoke-chat-memory-profile.sh exited {:?}",
        status.code()
    );
}

#[test]
fn slice_questionnaire_http_smoke() {
    if std::env::var("RUN_SLICE_QUESTIONNAIRE_SMOKE")
        .ok()
        .as_deref()
        != Some("1")
    {
        eprintln!(
            "skip slice_questionnaire_http_smoke: set RUN_SLICE_QUESTIONNAIRE_SMOKE=1 and start services (see CHAT_MEMORY_PROFILE_SLICE.md)"
        );
        return;
    }
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/smoke-questionnaire-profile.sh");
    let script = script
        .canonicalize()
        .unwrap_or_else(|_| panic!("smoke script not found: {}", script.display()));
    let status = Command::new("bash")
        .arg(script.as_os_str())
        .status()
        .expect("spawn bash for questionnaire smoke script");
    assert!(
        status.success(),
        "smoke-questionnaire-profile.sh exited {:?}",
        status.code()
    );
}
