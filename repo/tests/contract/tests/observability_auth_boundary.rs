use std::fs;

const OBSERVABILITY_ROUTE_FILES: &[(&str, &str)] = &[
    (
        "context-service",
        "../../services/context-service/src/http/routes.rs",
    ),
    (
        "ai-chat-service",
        "../../services/ai-chat-service/src/http/routes.rs",
    ),
    (
        "identity-service",
        "../../services/identity-service/src/http/routes.rs",
    ),
    (
        "profile-service",
        "../../services/profile-service/src/http/routes.rs",
    ),
    (
        "question-service",
        "../../services/question-service/src/http/routes.rs",
    ),
    ("bff", "../../services/bff/src/http/routes.rs"),
    ("dm-service", "../../services/dm-service/src/http/routes.rs"),
    (
        "match-service",
        "../../services/match-service/src/http/routes.rs",
    ),
    (
        "safety-service",
        "../../services/safety-service/src/http/routes.rs",
    ),
    (
        "model-gateway",
        "../../services/model-gateway/src/http/routes.rs",
    ),
    (
        "api-gateway",
        "../../services/api-gateway/src/http/routes.rs",
    ),
];

#[test]
fn observability_endpoints_require_internal_token_auth() {
    let mut failures = Vec::new();
    for (svc_name, route_path) in OBSERVABILITY_ROUTE_FILES {
        let content = fs::read_to_string(route_path)
            .unwrap_or_else(|e| panic!("cannot read {route_path}: {e}"));
        let has_obs_route = content.contains("/internal/observability/");
        if !has_obs_route {
            continue;
        }
        let obs_handler_names: Vec<String> = content
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.contains("/internal/observability/") {
                    if let Some(handler) = trimmed
                        .split(',')
                        .find(|part| part.contains("get(") || part.contains("post("))
                    {
                        for call in ["get(", "post("] {
                            if let Some(inner) = handler.split(call).nth(1) {
                                let name = inner.trim().trim_end_matches(')').trim();
                                if !name.is_empty() && !name.starts_with('|') {
                                    return Some(name.to_string());
                                }
                            }
                        }
                    }
                }
                None
            })
            .collect();
        for handler_name in &obs_handler_names {
            let sig = format!("async fn {handler_name}(");
            if let Some(start) = content.find(&sig) {
                let body_region = &content[start..];
                let first_lines = body_region.lines().take(5).collect::<Vec<_>>();
                let joined = first_lines.join("\n");
                if !joined.contains("verify_internal_token") {
                    failures.push(format!(
                        "{svc_name}: observability handler '{handler_name}' does not call verify_internal_token within first 5 lines — observability endpoints must require internal auth"
                    ));
                }
            } else {
                failures.push(format!(
                    "{svc_name}: could not find handler body for '{handler_name}'"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "observability auth contract violated:\n{}",
        failures.join("\n")
    );
}

#[test]
fn observability_routes_are_under_internal_bind_addr() {
    let mut failures = Vec::new();
    for (svc_name, route_path) in OBSERVABILITY_ROUTE_FILES {
        let content = fs::read_to_string(route_path)
            .unwrap_or_else(|e| panic!("cannot read {route_path}: {e}"));
        if content.contains("/internal/observability/") {
            let config_path = format!("../../services/{svc_name}/src/config.rs");
            let config_content = fs::read_to_string(&config_path)
                .unwrap_or_else(|e| panic!("cannot read {config_path}: {e}"));
            if !config_content.contains("internal_bind_addr") {
                failures.push(format!(
                    "{svc_name}: has /internal/observability/ routes but config does not define internal_bind_addr — observability endpoints must only be accessible on internal bind address"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "observability network protection contract violated:\n{}",
        failures.join("\n")
    );
}

#[test]
fn observability_routes_have_ip_allowlist_middleware() {
    let mut failures = Vec::new();
    for (svc_name, route_path) in OBSERVABILITY_ROUTE_FILES {
        let content = fs::read_to_string(route_path)
            .unwrap_or_else(|e| panic!("cannot read {route_path}: {e}"));
        if content.contains("/internal/observability/") && !content.contains("ip_allowlist_layer") {
            failures.push(format!(
                "{svc_name}: has /internal/observability/ routes but does not wire ip_allowlist_layer middleware — observability endpoints must be protected by IP allowlist"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "observability IP allowlist contract violated:\n{}",
        failures.join("\n")
    );
}

#[test]
fn ip_allowlist_layer_denies_undetermined_ip_in_non_dev() {
    let lib_path = "../../platform/shared-libs/onelink-internal-auth/src/lib.rs";
    let content =
        fs::read_to_string(lib_path).unwrap_or_else(|e| panic!("cannot read {lib_path}: {e}"));

    let has_none_dev_allow = content.contains("None if env_mode == \"dev\"");
    let has_none_non_dev_deny =
        content.contains("None =>") && content.contains("StatusCode::FORBIDDEN.into_response()");

    assert!(
        has_none_dev_allow,
        "ip_allowlist_layer must explicitly allow requests with undetermined client IP in dev mode (None if env_mode == \"dev\")"
    );
    assert!(
        has_none_non_dev_deny,
        "ip_allowlist_layer must deny requests with undetermined client IP in non-dev mode (None => FORBIDDEN), not allow them"
    );
}

#[test]
fn ip_allowlist_denies_unknown_ip_in_non_dev() {
    let auth_lib_path = "../../platform/shared-libs/onelink-internal-auth/src/lib.rs";
    let content = fs::read_to_string(auth_lib_path)
        .unwrap_or_else(|e| panic!("cannot read {auth_lib_path}: {e}"));

    if !content.contains("ip_allowlist_layer") {
        panic!("ip_allowlist_layer function not found in onelink-internal-auth");
    }

    let mut failures = Vec::new();

    if !content.contains("env_mode == \"dev\"") && !content.contains("env_mode_from_request") {
        failures.push(
            "ip_allowlist_layer does not check ONELINK_ENV — must differentiate dev vs non-dev behavior for undetermined IP".to_string(),
        );
    }

    let has_dev_allow = content.contains("env_mode == \"dev\"")
        && (content.contains("next.run(request).await") || content.contains("allowed"));
    if !has_dev_allow {
        failures.push(
            "ip_allowlist_layer must allow requests with undetermined IP in dev mode (in-process/test scenario)".to_string(),
        );
    }

    let has_non_dev_deny = content.contains("FORBIDDEN")
        && content.contains("undetermined")
        && content.contains("non-dev");
    if !has_non_dev_deny {
        failures.push(
            "ip_allowlist_layer must deny requests with undetermined IP in non-dev mode — 'unknown IP = deny' is the safe default for production/staging".to_string(),
        );
    }

    if !content.contains("x-forwarded-for") && !content.contains("ConnectInfo") {
        failures.push(
            "ip_allowlist_layer must attempt to extract client IP from x-forwarded-for header or ConnectInfo<SocketAddr>".to_string(),
        );
    }

    assert!(
        failures.is_empty(),
        "IP allowlist deny-in-non-dev contract violated:\n{}",
        failures.join("\n")
    );
}

#[test]
fn ip_allowlist_non_dev_deny_matches_exact_pattern() {
    let auth_lib_path = "../../platform/shared-libs/onelink-internal-auth/src/lib.rs";
    let content = fs::read_to_string(auth_lib_path)
        .unwrap_or_else(|e| panic!("cannot read {auth_lib_path}: {e}"));

    let has_non_dev_deny = content.contains("None =>")
        && content.contains("non-dev")
        && content.contains("StatusCode::FORBIDDEN");
    assert!(
        has_non_dev_deny,
        "ip_allowlist_layer must deny requests with undetermined client IP in non-dev environments (deny-by-default). Found logic does not match expected pattern."
    );

    let has_dev_allow = content.contains("None if env_mode == \"dev\"");
    assert!(
        has_dev_allow,
        "ip_allowlist_layer must allow requests with undetermined client IP in dev mode only. Found logic does not match expected pattern."
    );
}
