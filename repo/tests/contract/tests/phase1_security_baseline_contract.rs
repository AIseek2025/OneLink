use std::fs;

const ALL_SERVICES: &[&str] = &[
    "identity-service",
    "ai-chat-service",
    "context-service",
    "profile-service",
    "question-service",
    "bff",
    "dm-service",
    "match-service",
    "safety-service",
    "model-gateway",
    "api-gateway",
];

#[test]
fn all_services_config_imports_validate_secret_for_env() {
    let mut failures = Vec::new();
    for svc in ALL_SERVICES {
        let config_path = format!("../../services/{}/src/config.rs", svc);
        let config_content = fs::read_to_string(&config_path)
            .unwrap_or_else(|e| panic!("cannot read {} config: {e}", svc));
        if !config_content.contains("onelink_internal_auth::validate_secret_for_env") {
            failures.push(format!(
                "{}: config.rs does not re-export validate_secret_for_env from shared crate",
                svc
            ));
        }
        if !config_content.contains("onelink_internal_auth::DEV_INTERNAL_SECRET") {
            failures.push(format!(
                "{}: config.rs does not import DEV_INTERNAL_SECRET from shared crate",
                svc
            ));
        }
        if config_content.contains("const DEV_INTERNAL_SECRET") {
            failures.push(format!(
                "{}: config.rs declares local DEV_INTERNAL_SECRET constant — must use shared crate",
                svc
            ));
        }
        if config_content.contains("pub fn validate_secret_for_env") {
            failures.push(format!(
                "{}: config.rs declares local validate_secret_for_env function — must use shared crate",
                svc
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "security baseline contract: all services must use shared auth crate in config.rs:\n{}",
        failures.join("\n")
    );
}

#[test]
fn all_services_lib_calls_validate_secret_for_env_at_startup() {
    let mut failures = Vec::new();
    for svc in ALL_SERVICES {
        let lib_path = format!("../../services/{}/src/lib.rs", svc);
        let lib_content = fs::read_to_string(&lib_path)
            .unwrap_or_else(|e| panic!("cannot read {} lib.rs: {e}", svc));
        if !lib_content.contains("validate_secret_for_env") {
            failures.push(format!(
                "{}: lib.rs does not call validate_secret_for_env at startup",
                svc
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "security baseline contract: all services must call validate_secret_for_env at startup in lib.rs:\n{}",
        failures.join("\n")
    );
}

#[test]
fn all_services_cargo_depends_on_internal_auth() {
    let mut failures = Vec::new();
    for svc in ALL_SERVICES {
        let cargo_path = format!("../../services/{}/Cargo.toml", svc);
        let cargo_content = fs::read_to_string(&cargo_path)
            .unwrap_or_else(|e| panic!("cannot read {} Cargo.toml: {e}", svc));
        if !cargo_content.contains("onelink-internal-auth") {
            failures.push(format!(
                "{}: Cargo.toml does not depend on onelink-internal-auth",
                svc
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "security baseline contract: all services must depend on onelink-internal-auth:\n{}",
        failures.join("\n")
    );
}

#[test]
fn all_services_call_require_explicit_env_mode_in_non_dev() {
    let mut failures = Vec::new();
    for svc in ALL_SERVICES {
        let lib_path = format!("../../services/{}/src/lib.rs", svc);
        let lib_content = fs::read_to_string(&lib_path)
            .unwrap_or_else(|e| panic!("cannot read {} lib.rs: {e}", svc));
        if !lib_content.contains("require_explicit_env_mode") {
            failures.push(format!(
                "{}: lib.rs does not call require_explicit_env_mode in non-dev guard — observability IP allowlist must be enforced at startup",
                svc
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "security baseline contract: all services must call require_explicit_env_mode in non-dev guard:\n{}",
        failures.join("\n")
    );
}
