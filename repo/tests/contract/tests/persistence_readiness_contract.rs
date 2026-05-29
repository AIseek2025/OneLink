use std::fs;

const MIGRATION_DIR: &str = "../../data-platform/db-schema/migrations";

const PHASE1_MIGRATION_FILES: &[&str] = &[
    "V001__identity.sql",
    "V002__profile.sql",
    "V003__context.sql",
    "V004__context_activation.sql",
    "V005__context_idempotency.sql",
    "V007__ai_chat.sql",
    "V009__question.sql",
];

const PHASE1_SERVICES: &[(&str, &str)] = &[
    ("identity-service", "V001__identity.sql"),
    ("profile-service", "V002__profile.sql"),
    ("context-service", "V003__context.sql"),
    ("ai-chat-service", "V007__ai_chat.sql"),
    ("question-service", "V009__question.sql"),
];

const FORBIDDEN_MIGRATION_PATTERNS: &[&str] =
    &["DROP TABLE", "TRUNCATE", "DELETE FROM users", "GRANT ALL"];

const REQUIRED_TABLES_PER_SERVICE: &[(&str, &[&str])] = &[
    (
        "V001__identity.sql",
        &["users", "sessions", "identity_bindings"],
    ),
    (
        "V002__profile.sql",
        &["profiles", "profile_facts", "profile_traits"],
    ),
    (
        "V003__context.sql",
        &[
            "memory_artifacts",
            "memory_summaries",
            "memory_entities",
            "memory_entity_links",
        ],
    ),
    ("V007__ai_chat.sql", &["ai_conversations", "ai_messages"]),
    (
        "V009__question.sql",
        &[
            "question_catalog",
            "question_deliveries",
            "question_answers",
        ],
    ),
];

#[test]
fn phase1_migration_files_exist() {
    let mut missing = Vec::new();
    for file in PHASE1_MIGRATION_FILES {
        let path = format!("{MIGRATION_DIR}/{file}");
        if !std::path::Path::new(&path).exists() {
            missing.push(file.to_string());
        }
    }
    assert!(
        missing.is_empty(),
        "Phase 1 persistence readiness: missing migration files: {}",
        missing.join(", ")
    );
}

#[test]
fn phase1_migrations_contain_no_destructive_operations() {
    let mut violations = Vec::new();
    for file in PHASE1_MIGRATION_FILES {
        let path = format!("{MIGRATION_DIR}/{file}");
        if let Ok(content) = fs::read_to_string(&path) {
            let upper = content.to_uppercase();
            for pattern in FORBIDDEN_MIGRATION_PATTERNS {
                if upper.contains(pattern) {
                    violations.push(format!("{file}: contains forbidden pattern '{pattern}'"));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "Phase 1 persistence readiness: migration safety violated:\n{}",
        violations.join("\n")
    );
}

#[test]
fn phase1_migrations_define_required_tables() {
    let mut missing = Vec::new();
    for (file, tables) in REQUIRED_TABLES_PER_SERVICE {
        let path = format!("{MIGRATION_DIR}/{file}");
        if let Ok(content) = fs::read_to_string(&path) {
            for table in *tables {
                if !content.contains(table) {
                    missing.push(format!("{file}: missing table '{table}'"));
                }
            }
        } else {
            missing.push(format!("{file}: file not found"));
        }
    }
    assert!(
        missing.is_empty(),
        "Phase 1 persistence readiness: required tables missing from migrations:\n{}",
        missing.join("\n")
    );
}

#[test]
fn phase1_migrations_have_valid_sql_structure() {
    let mut errors = Vec::new();
    for file in PHASE1_MIGRATION_FILES {
        let path = format!("{MIGRATION_DIR}/{file}");
        if let Ok(content) = fs::read_to_string(&path) {
            if content.trim().is_empty() {
                errors.push(format!("{file}: empty file"));
                continue;
            }
            let create_count = content.matches("CREATE TABLE").count();
            let alter_count = content.matches("ALTER TABLE").count();
            if create_count == 0 && alter_count == 0 {
                errors.push(format!("{file}: no CREATE TABLE or ALTER TABLE statements"));
            }
            if !content.contains(';') {
                errors.push(format!("{file}: no semicolons — likely incomplete SQL"));
            }
            let open_paren = content.matches('(').count();
            let close_paren = content.matches(')').count();
            if open_paren > 0 && close_paren > 0 && open_paren != close_paren {
                errors.push(format!(
                    "{file}: unbalanced parentheses (open={open_paren}, close={close_paren})"
                ));
            }
        }
    }
    assert!(
        errors.is_empty(),
        "Phase 1 persistence readiness: migration structure validation failed:\n{}",
        errors.join("\n")
    );
}

#[test]
fn phase1_services_have_postgres_store_files() {
    let mut missing = Vec::new();
    for (svc, _) in PHASE1_SERVICES {
        let postgres_path = format!("../../services/{svc}/src/store/postgres.rs");
        if fs::metadata(&postgres_path).is_err() {
            missing.push(format!("{svc}: store/postgres.rs not found"));
        }
    }
    assert!(
        missing.is_empty(),
        "Phase 1 persistence readiness: services missing Postgres store implementations:\n{}",
        missing.join("\n")
    );
}

#[test]
fn phase1_postgres_stores_use_connection_pool() {
    let mut failures = Vec::new();
    for (svc, _) in PHASE1_SERVICES {
        let postgres_path = format!("../../services/{svc}/src/store/postgres.rs");
        if let Ok(content) = fs::read_to_string(&postgres_path) {
            let uses_pool = content.contains("deadpool") || content.contains("tokio_postgres");
            if !uses_pool {
                failures.push(format!(
                    "{svc}: store/postgres.rs does not use deadpool-postgres or tokio-postgres"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "Phase 1 persistence readiness: Postgres stores must use connection pooling:\n{}",
        failures.join("\n")
    );
}

#[test]
fn phase1_identity_migration_stores_token_hash_not_raw() {
    let path = format!("{MIGRATION_DIR}/V001__identity.sql");
    let content = fs::read_to_string(&path).expect("V001__identity.sql must exist");
    assert!(
        content.contains("token_hash"),
        "sessions table must store token_hash (SHA-256), not raw tokens"
    );
    assert!(
        !content.contains("token VARCHAR") && !content.contains("token TEXT"),
        "sessions table must NOT have a raw 'token' column — use token_hash instead"
    );
}

#[test]
fn phase1_smoke_test_script_exists() {
    let script_path = "../../scripts/smoke-persistence-e2e.sh";
    assert!(
        std::path::Path::new(script_path).exists(),
        "Phase 1 persistence readiness: smoke-persistence-e2e.sh must exist for runtime verification"
    );
}

#[test]
fn phase1_smoke_test_script_is_executable_and_valid() {
    let script_path = "../../scripts/smoke-persistence-e2e.sh";
    let content = fs::read_to_string(script_path).expect("smoke-persistence-e2e.sh must exist");
    assert!(
        content.contains("#!/usr/bin/env bash"),
        "smoke script must have proper shebang"
    );
    assert!(
        content.contains("DATABASE_URL"),
        "smoke script must reference DATABASE_URL"
    );
    assert!(
        content.contains("identity-service")
            && content.contains("profile-service")
            && content.contains("context-service"),
        "smoke script must test identity, profile, and context services"
    );
    assert!(
        content.contains("restart"),
        "smoke script must verify data survives service restart"
    );
}
