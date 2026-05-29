use std::fs;

const DDL_DIR: &str = "../../data-platform/db-schema/drafts";

const EXPECTED_DDL_FILES: &[&str] = &[
    "001_identity.sql",
    "002_profile.sql",
    "003_context.sql",
    "003_context_activation.sql",
    "003_context_idempotency.sql",
    "004_ai_chat.sql",
    "005_dm.sql",
    "006_question.sql",
    "007_match.sql",
    "008_safety.sql",
    "009_model_gateway.sql",
    "010_optimization.sql",
    "011_runtime_observability.sql",
];

const FORBIDDEN_PATTERNS: &[&str] = &["DROP TABLE", "TRUNCATE", "DELETE FROM users", "GRANT ALL"];

const REQUIRED_PATTERNS: &[(&str, &str)] = &[
    ("001_identity.sql", "CREATE TABLE users"),
    ("001_identity.sql", "CREATE TABLE sessions"),
    ("001_identity.sql", "token_hash"),
    ("002_profile.sql", "CREATE TABLE profiles"),
    ("002_profile.sql", "CREATE TABLE profile_facts"),
    ("003_context.sql", "CREATE TABLE memory_artifacts"),
    ("003_context.sql", "CREATE TABLE memory_summaries"),
    ("003_context.sql", "CREATE TABLE memory_entities"),
    ("003_context.sql", "CREATE TABLE memory_entity_links"),
    ("003_context.sql", "CREATE TABLE context_logs"),
    ("003_context_activation.sql", "ALTER TABLE"),
    ("003_context_idempotency.sql", "CREATE TABLE"),
    ("004_ai_chat.sql", "CREATE TABLE ai_conversations"),
    ("004_ai_chat.sql", "CREATE TABLE ai_messages"),
    ("006_question.sql", "CREATE TABLE question_catalog"),
    ("006_question.sql", "CREATE TABLE question_deliveries"),
    ("010_optimization.sql", "CREATE TABLE"),
    ("011_runtime_observability.sql", "CREATE TABLE"),
];

#[test]
fn all_expected_ddl_files_exist() {
    let mut missing = Vec::new();
    for file in EXPECTED_DDL_FILES {
        let path = format!("{DDL_DIR}/{file}");
        if !std::path::Path::new(&path).exists() {
            missing.push(file.to_string());
        }
    }
    assert!(
        missing.is_empty(),
        "missing DDL files: {}",
        missing.join(", ")
    );
}

#[test]
fn ddl_files_contain_no_destructive_operations() {
    let mut violations = Vec::new();
    for file in EXPECTED_DDL_FILES {
        let path = format!("{DDL_DIR}/{file}");
        if let Ok(content) = fs::read_to_string(&path) {
            let upper = content.to_uppercase();
            for pattern in FORBIDDEN_PATTERNS {
                if upper.contains(pattern) {
                    violations.push(format!("{file}: contains forbidden pattern '{pattern}'"));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "DDL safety contract violated:\n{}",
        violations.join("\n")
    );
}

#[test]
fn ddl_files_contain_required_table_definitions() {
    let mut missing = Vec::new();
    for (file, pattern) in REQUIRED_PATTERNS {
        let path = format!("{DDL_DIR}/{file}");
        if let Ok(content) = fs::read_to_string(&path) {
            if !content.contains(pattern) {
                missing.push(format!("{file}: missing required pattern '{pattern}'"));
            }
        } else {
            missing.push(format!("{file}: file not found"));
        }
    }
    assert!(
        missing.is_empty(),
        "DDL completeness contract violated:\n{}",
        missing.join("\n")
    );
}

#[test]
fn ddl_files_have_valid_sql_structure() {
    let mut errors = Vec::new();
    for file in EXPECTED_DDL_FILES {
        let path = format!("{DDL_DIR}/{file}");
        if let Ok(content) = fs::read_to_string(&path) {
            let lines: Vec<&str> = content.lines().collect();
            if lines.is_empty() {
                errors.push(format!("{file}: empty file"));
                continue;
            }
            let create_count = content.matches("CREATE TABLE").count();
            let alter_count = content.matches("ALTER TABLE").count();
            if create_count == 0 && alter_count == 0 {
                errors.push(format!(
                    "{file}: no CREATE TABLE or ALTER TABLE statements found"
                ));
            }
            let has_semicolon = content.contains(';');
            if !has_semicolon {
                errors.push(format!(
                    "{file}: no semicolons found — likely incomplete SQL"
                ));
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
        "DDL structure validation failed:\n{}",
        errors.join("\n")
    );
}

#[test]
fn identity_sessions_store_token_hash_not_raw() {
    let path = format!("{DDL_DIR}/001_identity.sql");
    let content = fs::read_to_string(&path).expect("001_identity.sql must exist");
    assert!(
        content.contains("token_hash"),
        "sessions table must store token_hash (SHA-256), not raw tokens"
    );
    assert!(
        !content.contains("token VARCHAR") && !content.contains("token TEXT"),
        "sessions table must NOT have a raw 'token' column — use token_hash instead"
    );
}
