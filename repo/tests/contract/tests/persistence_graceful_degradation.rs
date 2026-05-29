use std::fs;

struct ServiceStore {
    name: &'static str,
    store_path: &'static str,
    config_path: &'static str,
}

const SERVICES_WITH_DB: &[ServiceStore] = &[
    ServiceStore {
        name: "identity-service",
        store_path: "../../services/identity-service/src/store/mod.rs",
        config_path: "../../services/identity-service/src/config.rs",
    },
    ServiceStore {
        name: "context-service",
        store_path: "../../services/context-service/src/store/mod.rs",
        config_path: "../../services/context-service/src/config.rs",
    },
    ServiceStore {
        name: "profile-service",
        store_path: "../../services/profile-service/src/store/mod.rs",
        config_path: "../../services/profile-service/src/config.rs",
    },
    ServiceStore {
        name: "ai-chat-service",
        store_path: "../../services/ai-chat-service/src/store/mod.rs",
        config_path: "../../services/ai-chat-service/src/config.rs",
    },
    ServiceStore {
        name: "question-service",
        store_path: "../../services/question-service/src/store/mod.rs",
        config_path: "../../services/question-service/src/config.rs",
    },
];

const SERVICES_WITH_OPTIONAL_DB: &[ServiceStore] = &[
    ServiceStore {
        name: "profile-service",
        store_path: "../../services/profile-service/src/store/mod.rs",
        config_path: "../../services/profile-service/src/config.rs",
    },
    ServiceStore {
        name: "ai-chat-service",
        store_path: "../../services/ai-chat-service/src/store/mod.rs",
        config_path: "../../services/ai-chat-service/src/config.rs",
    },
    ServiceStore {
        name: "question-service",
        store_path: "../../services/question-service/src/store/mod.rs",
        config_path: "../../services/question-service/src/config.rs",
    },
];

#[test]
fn services_with_optional_db_refuse_silent_fallback() {
    let mut failures = Vec::new();
    for svc in SERVICES_WITH_OPTIONAL_DB {
        let store_content = fs::read_to_string(svc.store_path)
            .unwrap_or_else(|e| panic!("cannot read {} store: {e}", svc.name));
        if store_content.contains("falling back to in-memory") {
            failures.push(format!(
                "{}: silent in-memory fallback found in store/mod.rs — forbidden when DATABASE_URL is set",
                svc.name
            ));
        }
        if !store_content.contains("panic!")
            && !store_content.contains("refusing silent in-memory fallback")
        {
            failures.push(format!(
                "{}: store/mod.rs does not panic on Postgres connect failure — silent fallback risk when DATABASE_URL is set",
                svc.name
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "persistence contract violated: services with optional DB must not silently fall back to in-memory when DATABASE_URL is set:\n{}",
        failures.join("\n")
    );
}

#[test]
fn services_with_mandatory_db_do_not_silently_fallback() {
    let mut failures = Vec::new();
    for svc in SERVICES_WITH_DB {
        let store_content = fs::read_to_string(svc.store_path)
            .unwrap_or_else(|e| panic!("cannot read {} store: {e}", svc.name));
        if store_content.contains("falling back to in-memory") {
            failures.push(format!(
                "{}: silent in-memory fallback found in store/mod.rs — forbidden when DATABASE_URL is set",
                svc.name
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "persistence contract violated: services must not silently fall back to in-memory when DATABASE_URL is set:\n{}",
        failures.join("\n")
    );
}

#[test]
fn all_services_use_shared_auth_crate() {
    let all_services: Vec<&ServiceStore> = SERVICES_WITH_DB
        .iter()
        .chain(SERVICES_WITH_OPTIONAL_DB.iter())
        .collect();
    let mut failures = Vec::new();
    for svc in &all_services {
        let config_content = fs::read_to_string(svc.config_path)
            .unwrap_or_else(|e| panic!("cannot read {} config: {e}", svc.name));
        if !config_content.contains("onelink_internal_auth::DEV_INTERNAL_SECRET") {
            failures.push(format!(
                "{}: config.rs does not import DEV_INTERNAL_SECRET from shared crate — local duplication detected",
                svc.name
            ));
        }
        if !config_content.contains("onelink_internal_auth::validate_secret_for_env") {
            failures.push(format!(
                "{}: config.rs does not re-export validate_secret_for_env from shared crate — local duplication detected",
                svc.name
            ));
        }
        if config_content.contains("const DEV_INTERNAL_SECRET") {
            failures.push(format!(
                "{}: config.rs still declares local DEV_INTERNAL_SECRET constant — must use shared crate",
                svc.name
            ));
        }
        if config_content.contains("pub fn validate_secret_for_env") {
            failures.push(format!(
                "{}: config.rs still declares local validate_secret_for_env function — must use shared crate",
                svc.name
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "auth dedup contract violated:\n{}",
        failures.join("\n")
    );
}

#[test]
fn services_with_mandatory_db_panic_on_connect_failure() {
    let mut failures = Vec::new();
    for svc in SERVICES_WITH_DB {
        let store_content = fs::read_to_string(svc.store_path)
            .unwrap_or_else(|e| panic!("cannot read {} store: {e}", svc.name));
        if !store_content.contains("panic!") && !store_content.contains("panic") {
            failures.push(format!(
                "{}: store/mod.rs does not panic on Postgres connect failure — silent fallback risk",
                svc.name
            ));
        }
    }
    for svc in SERVICES_WITH_OPTIONAL_DB {
        let store_content = fs::read_to_string(svc.store_path)
            .unwrap_or_else(|e| panic!("cannot read {} store: {e}", svc.name));
        if !store_content.contains("panic!")
            && !store_content.contains("refusing silent in-memory fallback")
        {
            failures.push(format!(
                "{}: store/mod.rs does not panic on Postgres connect failure when DATABASE_URL is set — silent fallback risk",
                svc.name
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "persistence panic-on-failure contract violated:\n{}",
        failures.join("\n")
    );
}

#[test]
fn services_with_optional_db_do_not_silently_fallback() {
    let mut failures = Vec::new();
    for svc in SERVICES_WITH_OPTIONAL_DB {
        let store_content = fs::read_to_string(svc.store_path)
            .unwrap_or_else(|e| panic!("cannot read {} store: {e}", svc.name));
        if store_content.contains("falling back to in-memory") {
            failures.push(format!(
                "{}: silent in-memory fallback found in store/mod.rs — forbidden when DATABASE_URL is set",
                svc.name
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "persistence contract violated: services with optional DB must not silently fall back to in-memory when DATABASE_URL is set:\n{}",
        failures.join("\n")
    );
}

#[test]
fn services_with_optional_db_panic_on_connect_failure() {
    let mut failures = Vec::new();
    for svc in SERVICES_WITH_OPTIONAL_DB {
        let store_content = fs::read_to_string(svc.store_path)
            .unwrap_or_else(|e| panic!("cannot read {} store: {e}", svc.name));
        if store_content.contains("DATABASE_URL")
            && !store_content.contains("panic!")
            && !store_content.contains("panic")
        {
            failures.push(format!(
                "{}: store/mod.rs uses DATABASE_URL but does not panic on Postgres connect failure — silent fallback risk",
                svc.name
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "persistence panic-on-failure contract violated (optional DB services):\n{}",
        failures.join("\n")
    );
}
