use std::fs;

const BFF_ROUTES_PATH: &str = "../../services/bff/src/http/routes.rs";
const PROFILE_ROUTES_PATH: &str = "../../services/profile-service/src/http/routes.rs";

#[test]
fn bff_public_endpoints_require_bearer_auth() {
    let content = fs::read_to_string(BFF_ROUTES_PATH)
        .unwrap_or_else(|e| panic!("cannot read {BFF_ROUTES_PATH}: {e}"));

    let has_auth_middleware = content.contains("bearer")
        || content.contains("Authorization")
        || content.contains("auth_layer")
        || content.contains("verify_bearer")
        || content.contains("extract_auth");
    assert!(
        has_auth_middleware,
        "BFF public endpoints must verify Bearer token / Authorization header before processing"
    );
}

#[test]
fn bff_transparent_auth_passthrough_documented() {
    let bff_yaml = fs::read_to_string("../../platform/contracts/openapi/bff.yaml")
        .expect("bff.yaml must be readable");
    assert!(
        bff_yaml.contains("透传") || bff_yaml.contains("passthrough") || bff_yaml.contains("Bearer"),
        "BFF OpenAPI must document that Authorization is transparently forwarded to identity-service"
    );
}

#[test]
fn profile_patch_me_rejects_unauthorized() {
    let content = fs::read_to_string(PROFILE_ROUTES_PATH)
        .unwrap_or_else(|e| panic!("cannot read {PROFILE_ROUTES_PATH}: {e}"));

    assert!(
        content.contains("UNAUTHORIZED")
            || content.contains("StatusCode::UNAUTHORIZED")
            || content.contains("401"),
        "PATCH /profile/me must return 401 UNAUTHORIZED when no valid identity is provided"
    );
}

#[test]
fn bff_sends_internal_token_only_to_non_bearer_services() {
    let routes_content = fs::read_to_string(BFF_ROUTES_PATH)
        .unwrap_or_else(|e| panic!("cannot read {BFF_ROUTES_PATH}: {e}"));

    let bearer_services = [
        ("identity-service", "identity_service_base_url"),
        ("profile-service", "profile_service_base_url"),
    ];

    let mut failures = Vec::new();

    for (svc_name, base_url_key) in &bearer_services {
        let pattern = format!("state.config.{}", base_url_key);
        if let Some(idx) = routes_content.find(&pattern) {
            let surrounding = &routes_content[idx.saturating_sub(200)..idx + pattern.len() + 100];
            if surrounding.contains("INTERNAL_TOKEN_HEADER") {
                failures.push(format!(
                    "BFF sends internal token to {svc_name} — {svc_name} uses Bearer auth, not internal token"
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "BFF auth routing contract violated:\n{}",
        failures.join("\n")
    );
}

#[test]
fn profile_service_non_dev_config_blocks_default_secret() {
    let config_path = "../../services/profile-service/src/config.rs";
    let content = fs::read_to_string(config_path)
        .unwrap_or_else(|e| panic!("cannot read {config_path}: {e}"));

    assert!(
        content.contains("validate_secret_for_env"),
        "profile-service config must call validate_secret_for_env to block default secret in non-dev"
    );
}
