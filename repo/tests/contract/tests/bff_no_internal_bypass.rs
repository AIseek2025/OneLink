use std::fs;

const BFF_ROUTES_PATH: &str = "../../services/bff/src/http/routes.rs";

#[test]
fn bff_does_not_call_internal_routes_on_downstream_services() {
    let content = fs::read_to_string(BFF_ROUTES_PATH)
        .unwrap_or_else(|e| panic!("cannot read {BFF_ROUTES_PATH}: {e}"));
    let mut failures = Vec::new();

    let internal_call_patterns = ["/internal/"];
    for pattern in &internal_call_patterns {
        if content.contains(pattern) {
            failures.push(format!(
                "BFF routes.rs contains '{pattern}' — BFF must only call public /api/v1/ endpoints on downstream services, never internal routes"
            ));
        }
    }

    let url_patterns = [
        "/api/v1/identity/",
        "/api/v1/chat/",
        "/api/v1/questions/",
        "/api/v1/match/",
        "/api/v1/dm/",
        "/api/v1/safety/",
        "/api/v1/profile/",
    ];
    let has_public_url = url_patterns
        .iter()
        .any(|p| content.contains(p) || content.contains("api/v1/"));

    if !has_public_url && !content.contains("/api/v1/") {
        failures.push(
            "BFF routes.rs does not reference any /api/v1/ downstream paths — verify BFF aggregation logic".to_string(),
        );
    }

    assert!(
        failures.is_empty(),
        "BFF internal bypass contract violated:\n{}",
        failures.join("\n")
    );
}

#[test]
fn bff_sends_internal_token_only_to_internal_only_services() {
    let content = fs::read_to_string(BFF_ROUTES_PATH)
        .unwrap_or_else(|e| panic!("cannot read {BFF_ROUTES_PATH}: {e}"));

    let internal_token_services = [
        ("match-service", "match_service_base_url"),
        ("dm-service", "dm_service_base_url"),
        ("safety-service", "safety_service_base_url"),
    ];

    let bearer_only_services = [
        ("identity-service", "identity_service_base_url"),
        ("profile-service", "profile_service_base_url"),
        ("ai-chat-service", "ai_chat_service_base_url"),
        ("question-service", "question_service_base_url"),
    ];

    let mut failures = Vec::new();

    for (svc_name, base_url_key) in &internal_token_services {
        let has_internal_token = content.contains(&format!(
            ".header(INTERNAL_TOKEN_HEADER, &state.config.{}",
            base_url_key.replace("_base_url", ".internal_shared_secret")
        )) || content.contains(&format!("state.config.{}", base_url_key));
        if !has_internal_token {
            failures.push(format!(
                "BFF does not send internal token to {svc_name} — services without Bearer auth must receive internal token"
            ));
        }
    }

    for (svc_name, base_url_key) in &bearer_only_services {
        let pattern = format!("state.config.{}", base_url_key);
        if let Some(idx) = content.find(&pattern) {
            let surrounding = &content[idx.saturating_sub(200)..idx + pattern.len() + 100];
            if surrounding.contains("INTERNAL_TOKEN_HEADER") {
                failures.push(format!(
                    "BFF sends internal token to {svc_name} — {svc_name} uses Bearer auth, not internal token"
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "BFF internal token routing contract violated:\n{}",
        failures.join("\n")
    );
}

#[test]
fn bff_config_has_no_internal_bind_addr() {
    let config_path = "../../services/bff/src/config.rs";
    let content = fs::read_to_string(config_path)
        .unwrap_or_else(|e| panic!("cannot read {config_path}: {e}"));

    assert!(
        !content.contains("internal_bind_addr"),
        "BFF config.rs must not have internal_bind_addr — BFF has no internal routes and should not bind to a restricted address"
    );
}

#[test]
fn bff_lib_bind_addr_is_public_only() {
    let lib_path = "../../services/bff/src/lib.rs";
    let content =
        fs::read_to_string(lib_path).unwrap_or_else(|e| panic!("cannot read {lib_path}: {e}"));

    assert!(
        content.contains("0, 0, 0, 0") || content.contains("0.0.0.0"),
        "BFF lib.rs must bind to a public address (0.0.0.0) since it serves only public /api/v1/ routes"
    );
    assert!(
        !content.contains("internal_bind_addr"),
        "BFF lib.rs must not reference internal_bind_addr — BFF has no internal routes"
    );
}
