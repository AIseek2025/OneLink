use std::fs;

const SERVICE_CONFIG_FILES: &[(&str, &str)] = &[
    (
        "identity-service",
        "../../services/identity-service/src/config.rs",
    ),
    (
        "context-service",
        "../../services/context-service/src/config.rs",
    ),
    (
        "ai-chat-service",
        "../../services/ai-chat-service/src/config.rs",
    ),
    (
        "profile-service",
        "../../services/profile-service/src/config.rs",
    ),
    (
        "question-service",
        "../../services/question-service/src/config.rs",
    ),
    (
        "model-gateway",
        "../../services/model-gateway/src/config.rs",
    ),
    (
        "match-service",
        "../../services/match-service/src/config.rs",
    ),
    (
        "safety-service",
        "../../services/safety-service/src/config.rs",
    ),
    ("dm-service", "../../services/dm-service/src/config.rs"),
];

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

#[test]
fn services_with_internal_routes_have_internal_bind_addr_config() {
    let mut failures = Vec::new();
    for (svc_name, route_path) in SERVICE_ROUTE_FILES {
        if let Ok(content) = fs::read_to_string(route_path) {
            if content.contains(".route(\"/internal/") {
                let config_path = SERVICE_CONFIG_FILES
                    .iter()
                    .find(|(n, _)| n == svc_name)
                    .map(|(_, p)| p)
                    .unwrap();
                if let Ok(config_content) = fs::read_to_string(config_path) {
                    if !config_content.contains("internal_bind_addr") {
                        failures.push(format!(
                            "{svc_name}: has /internal/ routes but config.rs lacks internal_bind_addr field — network-level protection not configurable"
                        ));
                    }
                    if !config_content.contains("INTERNAL_BIND_ADDR") {
                        failures.push(format!(
                            "{svc_name}: has /internal/ routes but config.rs does not read INTERNAL_BIND_ADDR env var"
                        ));
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "internal network protection contract violated:\n{}",
        failures.join("\n")
    );
}

#[test]
fn internal_bind_addr_defaults_to_localhost() {
    let mut failures = Vec::new();
    for (svc_name, config_path) in SERVICE_CONFIG_FILES {
        if let Ok(content) = fs::read_to_string(config_path) {
            if content.contains("internal_bind_addr") && !content.contains("\"127.0.0.1\"") {
                failures.push(format!(
                    "{svc_name}: internal_bind_addr default is not 127.0.0.1 — internal routes may be exposed to non-localhost interfaces"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "internal bind address default contract violated:\n{}",
        failures.join("\n")
    );
}

#[test]
fn services_bind_to_internal_addr_in_non_dev() {
    let lib_files: &[(&str, &str)] = &[
        (
            "identity-service",
            "../../services/identity-service/src/lib.rs",
        ),
        (
            "context-service",
            "../../services/context-service/src/lib.rs",
        ),
        (
            "ai-chat-service",
            "../../services/ai-chat-service/src/lib.rs",
        ),
        (
            "profile-service",
            "../../services/profile-service/src/lib.rs",
        ),
        (
            "question-service",
            "../../services/question-service/src/lib.rs",
        ),
        ("model-gateway", "../../services/model-gateway/src/lib.rs"),
        ("match-service", "../../services/match-service/src/lib.rs"),
        ("safety-service", "../../services/safety-service/src/lib.rs"),
        ("dm-service", "../../services/dm-service/src/lib.rs"),
    ];
    let mut failures = Vec::new();
    for (svc_name, lib_path) in lib_files {
        if let Ok(content) = fs::read_to_string(lib_path) {
            if content.contains("internal_bind_addr") {
                if !content.contains("env_mode") || !content.contains("\"dev\"") {
                    failures.push(format!(
                        "{svc_name}: lib.rs uses internal_bind_addr but does not conditionally bind to 0.0.0.0 in dev mode"
                    ));
                }
            } else if content.contains("0, 0, 0, 0") {
                failures.push(format!(
                    "{svc_name}: lib.rs hardcodes 0.0.0.0 bind address — must use internal_bind_addr in non-dev environments"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "service bind address contract violated:\n{}",
        failures.join("\n")
    );
}
