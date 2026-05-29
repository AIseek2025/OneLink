use std::fs;

const SERVICE_ROUTE_FILES: &[&str] = &[
    "../../services/identity-service/src/http/routes.rs",
    "../../services/profile-service/src/http/routes.rs",
    "../../services/ai-chat-service/src/http/routes.rs",
    "../../services/context-service/src/http/routes.rs",
    "../../services/question-service/src/http/routes.rs",
    "../../services/bff/src/http/routes.rs",
    "../../services/model-gateway/src/http/routes.rs",
    "../../services/match-service/src/http/routes.rs",
    "../../services/safety-service/src/http/routes.rs",
    "../../services/dm-service/src/http/routes.rs",
];

fn has_internal_route_definitions(content: &str) -> bool {
    content.contains(".route(\"/internal/")
}

#[test]
fn no_service_has_local_verify_internal_token_function() {
    let mut failures = Vec::new();
    for path in SERVICE_ROUTE_FILES {
        if let Ok(content) = fs::read_to_string(path) {
            let service_name = path.split('/').nth(2).unwrap_or("unknown");
            if content.contains("fn verify_internal_token(") && !content.contains("#[cfg(test)]") {
                let has_shared_import =
                    content.contains("onelink_internal_auth::verify_internal_token");
                if !has_shared_import {
                    failures.push(format!(
                        "{service_name}: routes.rs defines local verify_internal_token() without importing from shared crate"
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "auth dedup contract: services must use onelink_internal_auth::verify_internal_token, not local copies:\n{}",
        failures.join("\n")
    );
}

#[test]
fn no_service_has_local_internal_token_header_constant() {
    let mut failures = Vec::new();
    for path in SERVICE_ROUTE_FILES {
        if let Ok(content) = fs::read_to_string(path) {
            let service_name = path.split('/').nth(2).unwrap_or("unknown");
            if content.contains("const INTERNAL_TOKEN_HEADER") {
                failures.push(format!(
                    "{service_name}: routes.rs declares local INTERNAL_TOKEN_HEADER constant — must use onelink_internal_auth::INTERNAL_TOKEN_HEADER"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "auth dedup contract: services must use onelink_internal_auth::INTERNAL_TOKEN_HEADER:\n{}",
        failures.join("\n")
    );
}

#[test]
fn all_internal_routes_require_auth() {
    let mut failures = Vec::new();
    for path in SERVICE_ROUTE_FILES {
        if let Ok(content) = fs::read_to_string(path) {
            let service_name = path.split('/').nth(2).unwrap_or("unknown");
            if has_internal_route_definitions(&content) {
                let has_verify = content.contains("verify_internal_token");
                if !has_verify {
                    failures.push(format!(
                        "{service_name}: has /internal/ routes but does not call verify_internal_token"
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "auth contract: all internal routes must verify x-internal-token:\n{}",
        failures.join("\n")
    );
}
