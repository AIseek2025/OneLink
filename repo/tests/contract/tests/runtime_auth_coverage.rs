use std::fs;

const SERVICE_ROUTE_FILES: &[(&str, &str)] = &[
    (
        "identity-service",
        "../../services/identity-service/src/http/routes.rs",
    ),
    (
        "context-service",
        "../../services/context-service/src/http/routes.rs",
    ),
    (
        "ai-chat-service",
        "../../services/ai-chat-service/src/http/routes.rs",
    ),
    (
        "profile-service",
        "../../services/profile-service/src/http/routes.rs",
    ),
    (
        "model-gateway",
        "../../services/model-gateway/src/http/routes.rs",
    ),
    (
        "match-service",
        "../../services/match-service/src/http/routes.rs",
    ),
    (
        "safety-service",
        "../../services/safety-service/src/http/routes.rs",
    ),
    ("dm-service", "../../services/dm-service/src/http/routes.rs"),
];

const RUNTIME_AUTH_TEST_FILES: &[(&str, &str)] = &[
    (
        "identity-service",
        "../../services/identity-service/tests/runtime_internal_auth.rs",
    ),
    (
        "context-service",
        "../../services/context-service/tests/runtime_internal_auth.rs",
    ),
    (
        "ai-chat-service",
        "../../services/ai-chat-service/tests/runtime_internal_auth.rs",
    ),
    (
        "model-gateway",
        "../../services/model-gateway/tests/runtime_internal_auth.rs",
    ),
    (
        "profile-service",
        "../../services/profile-service/tests/runtime_internal_auth.rs",
    ),
    (
        "match-service",
        "../../services/match-service/tests/runtime_internal_auth.rs",
    ),
    (
        "safety-service",
        "../../services/safety-service/tests/runtime_internal_auth.rs",
    ),
    (
        "dm-service",
        "../../services/dm-service/tests/runtime_internal_auth.rs",
    ),
];

#[test]
fn every_service_with_internal_routes_has_runtime_auth_test() {
    let mut failures = Vec::new();

    for (svc_name, route_path) in SERVICE_ROUTE_FILES {
        let content = fs::read_to_string(route_path)
            .unwrap_or_else(|e| panic!("cannot read {route_path}: {e}"));

        if !content.contains(".route(\"/internal/") {
            continue;
        }

        let has_runtime_test = RUNTIME_AUTH_TEST_FILES
            .iter()
            .any(|(name, test_path)| name == svc_name && fs::metadata(test_path).is_ok());

        if !has_runtime_test {
            failures.push(format!(
                "{svc_name}: has /internal/ routes but no tests/runtime_internal_auth.rs — runtime HTTP auth rejection must be verified"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "runtime auth test coverage contract violated:\n{}",
        failures.join("\n")
    );
}

#[test]
fn runtime_auth_tests_cover_missing_wrong_and_correct_token() {
    let mut failures = Vec::new();

    for (svc_name, test_path) in RUNTIME_AUTH_TEST_FILES {
        let content = match fs::read_to_string(test_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let has_missing =
            content.contains("rejects_missing_token") || content.contains("missing_token");
        let has_wrong = content.contains("rejects_wrong_token") || content.contains("wrong_token");
        let has_correct =
            content.contains("accepts_correct_token") || content.contains("correct_token");

        if !has_missing {
            failures.push(format!(
                "{svc_name}: runtime auth test does not verify rejection of missing token"
            ));
        }
        if !has_wrong {
            failures.push(format!(
                "{svc_name}: runtime auth test does not verify rejection of wrong token"
            ));
        }
        if !has_correct {
            failures.push(format!(
                "{svc_name}: runtime auth test does not verify acceptance of correct token"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "runtime auth test completeness contract violated:\n{}",
        failures.join("\n")
    );
}

#[test]
fn runtime_auth_tests_use_oneshot_not_real_http() {
    let mut failures = Vec::new();

    for (svc_name, test_path) in RUNTIME_AUTH_TEST_FILES {
        let content = match fs::read_to_string(test_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if !content.contains("oneshot") {
            failures.push(format!(
                "{svc_name}: runtime auth test does not use tower::ServiceExt::oneshot — must use in-process Axum router, not real HTTP listener"
            ));
        }

        if content.contains("TcpListener") || content.contains("tokio::net::TcpListener") {
            failures.push(format!(
                "{svc_name}: runtime auth test must not bind a real TCP listener — use tower::ServiceExt::oneshot for deterministic in-process testing"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "runtime auth test methodology contract violated:\n{}",
        failures.join("\n")
    );
}
