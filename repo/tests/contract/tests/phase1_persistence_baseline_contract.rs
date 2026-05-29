use std::fs;

struct ServicePersistence {
    name: &'static str,
    store_path: &'static str,
    migration_file: &'static str,
    lib_path: &'static str,
}

const PHASE1_PERSISTENCE_SERVICES: &[ServicePersistence] = &[
    ServicePersistence {
        name: "identity-service",
        store_path: "../../services/identity-service/src/store/mod.rs",
        migration_file: "../../data-platform/db-schema/migrations/V001__identity.sql",
        lib_path: "../../services/identity-service/src/lib.rs",
    },
    ServicePersistence {
        name: "ai-chat-service",
        store_path: "../../services/ai-chat-service/src/store/mod.rs",
        migration_file: "../../data-platform/db-schema/migrations/V007__ai_chat.sql",
        lib_path: "../../services/ai-chat-service/src/lib.rs",
    },
    ServicePersistence {
        name: "context-service",
        store_path: "../../services/context-service/src/store/mod.rs",
        migration_file: "../../data-platform/db-schema/migrations/V003__context.sql",
        lib_path: "../../services/context-service/src/lib.rs",
    },
    ServicePersistence {
        name: "question-service",
        store_path: "../../services/question-service/src/store/mod.rs",
        migration_file: "../../data-platform/db-schema/migrations/V009__question.sql",
        lib_path: "../../services/question-service/src/lib.rs",
    },
    ServicePersistence {
        name: "profile-service",
        store_path: "../../services/profile-service/src/store/mod.rs",
        migration_file: "../../data-platform/db-schema/migrations/V002__profile.sql",
        lib_path: "../../services/profile-service/src/lib.rs",
    },
];

#[test]
fn phase1_services_have_dual_persistence_backend() {
    let mut failures = Vec::new();
    for svc in PHASE1_PERSISTENCE_SERVICES {
        let store_content = fs::read_to_string(svc.store_path)
            .unwrap_or_else(|e| panic!("cannot read {} store: {e}", svc.name));
        let has_postgres_variant =
            store_content.contains("Postgres") || store_content.contains("postgres::PostgresStore");
        let has_in_memory_variant = store_content.contains("InMemory");
        if !has_postgres_variant || !has_in_memory_variant {
            failures.push(format!(
                "{}: store/mod.rs must have both Postgres and InMemory backend variants",
                svc.name
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "Phase 1 persistence contract: all Phase 1 services must have dual persistence backend:\n{}",
        failures.join("\n")
    );
}

#[test]
fn phase1_services_panic_on_postgres_connect_failure() {
    let mut failures = Vec::new();
    for svc in PHASE1_PERSISTENCE_SERVICES {
        let store_content = fs::read_to_string(svc.store_path)
            .unwrap_or_else(|e| panic!("cannot read {} store: {e}", svc.name));
        if !store_content.contains("panic!") {
            failures.push(format!(
                "{}: store/mod.rs does not panic on Postgres connect failure — silent fallback risk",
                svc.name
            ));
        }
        if store_content.contains("falling back to in-memory") {
            failures.push(format!(
                "{}: store/mod.rs contains silent in-memory fallback — forbidden when DATABASE_URL is set",
                svc.name
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "Phase 1 persistence contract: services must panic on Postgres connect failure, not silently fall back:\n{}",
        failures.join("\n")
    );
}

#[test]
fn phase1_services_have_migration_files() {
    let mut failures = Vec::new();
    for svc in PHASE1_PERSISTENCE_SERVICES {
        if fs::metadata(svc.migration_file).is_err() {
            failures.push(format!(
                "{}: migration file {} not found",
                svc.name, svc.migration_file
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "Phase 1 persistence contract: all Phase 1 services must have DDL migration files:\n{}",
        failures.join("\n")
    );
}

#[test]
fn phase1_services_in_memory_limited_to_dev() {
    let mut failures = Vec::new();
    for svc in PHASE1_PERSISTENCE_SERVICES {
        let store_content = fs::read_to_string(svc.store_path)
            .unwrap_or_else(|e| panic!("cannot read {} store: {e}", svc.name));
        if !store_content.contains("dev") && !store_content.contains("smoke") {
            failures.push(format!(
                "{}: store/mod.rs in-memory path does not indicate dev/smoke-only limitation",
                svc.name
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "Phase 1 persistence contract: in-memory fallback must be clearly limited to dev/smoke:\n{}",
        failures.join("\n")
    );
}

#[test]
fn phase1_services_block_startup_without_database_url_in_non_dev() {
    let mut failures = Vec::new();
    for svc in PHASE1_PERSISTENCE_SERVICES {
        let lib_content = fs::read_to_string(svc.lib_path)
            .unwrap_or_else(|e| panic!("cannot read {} lib.rs: {e}", svc.name));
        if !lib_content.contains("DATABASE_URL is not set in non-dev") {
            failures.push(format!(
                "{}: lib.rs does not block startup when DATABASE_URL is unset in non-dev environment",
                svc.name
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "Phase 1 persistence contract: all Phase 1 services must refuse to start without DATABASE_URL in non-dev:\n{}",
        failures.join("\n")
    );
}

#[test]
fn phase1_services_have_postgres_store_implementation() {
    let mut failures = Vec::new();
    for svc in PHASE1_PERSISTENCE_SERVICES {
        let postgres_path = format!("../../services/{}/src/store/postgres.rs", svc.name);
        if fs::metadata(&postgres_path).is_err() {
            failures.push(format!(
                "{}: store/postgres.rs not found — Phase 1 services must have a real Postgres store implementation",
                svc.name
            ));
        } else {
            let pg_content = fs::read_to_string(&postgres_path)
                .unwrap_or_else(|e| panic!("cannot read {} postgres store: {e}", svc.name));
            if !pg_content.contains("deadpool") && !pg_content.contains("tokio_postgres") {
                failures.push(format!(
                    "{}: store/postgres.rs does not use deadpool-postgres or tokio-postgres",
                    svc.name
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "Phase 1 persistence contract: all Phase 1 services must have a real Postgres store implementation:\n{}",
        failures.join("\n")
    );
}
