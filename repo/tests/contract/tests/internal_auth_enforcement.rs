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

fn extract_internal_handler_names(content: &str) -> Vec<String> {
    let mut handlers = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.contains(".route(\"/internal/") {
            if let Some(handler) = trimmed
                .split(',')
                .find(|part| part.contains("get(") || part.contains("post("))
            {
                for call in ["get(", "post("] {
                    if let Some(inner) = handler.split(call).nth(1) {
                        let name = inner.trim().trim_end_matches(')').trim();
                        if !name.is_empty() && !name.starts_with('|') && !name.starts_with('"') {
                            handlers.push(name.to_string());
                        }
                    }
                }
            }
        }
    }
    handlers
}

fn find_handler_body(content: &str, handler_name: &str) -> Option<String> {
    let sig_patterns = [
        format!("async fn {handler_name}("),
        format!("fn {handler_name}("),
    ];
    for sig in &sig_patterns {
        if let Some(start) = content.find(sig) {
            let body_start = content[start..]
                .find('{')
                .map(|offset| start + offset + 1)?;
            let mut depth = 1;
            let mut end = body_start;
            for ch in content[body_start..].chars() {
                if ch == '{' {
                    depth += 1;
                } else if ch == '}' {
                    depth -= 1;
                }
                end += ch.len_utf8();
                if depth == 0 {
                    break;
                }
            }
            return Some(content[body_start..end - 1].to_string());
        }
    }
    None
}

#[test]
fn every_internal_handler_calls_verify_internal_token_before_logic() {
    let mut failures = Vec::new();
    for (svc_name, route_path) in SERVICE_ROUTE_FILES {
        let content = fs::read_to_string(route_path)
            .unwrap_or_else(|e| panic!("cannot read {route_path}: {e}"));
        let handlers = extract_internal_handler_names(&content);
        for handler_name in &handlers {
            let body = find_handler_body(&content, handler_name);
            match body {
                Some(body) => {
                    let first_lines = body.lines().take(5).collect::<Vec<_>>();
                    let joined = first_lines.join("\n");
                    if !joined.contains("verify_internal_token") {
                        failures.push(format!(
                            "{svc_name}: handler '{handler_name}' does not call verify_internal_token within first 5 lines of body — internal auth must be checked before any business logic"
                        ));
                    }
                }
                None => {
                    failures.push(format!(
                        "{svc_name}: could not extract body for handler '{handler_name}' — verify manually"
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "internal auth enforcement contract violated:\n{}",
        failures.join("\n")
    );
}

#[test]
fn no_internal_handler_returns_success_without_auth_check() {
    let mut failures = Vec::new();
    for (svc_name, route_path) in SERVICE_ROUTE_FILES {
        let content = fs::read_to_string(route_path)
            .unwrap_or_else(|e| panic!("cannot read {route_path}: {e}"));
        let handlers = extract_internal_handler_names(&content);
        for handler_name in &handlers {
            let body = find_handler_body(&content, handler_name);
            if let Some(body) = body {
                let lines = body.lines().collect::<Vec<_>>();
                let auth_line_idx = lines
                    .iter()
                    .position(|l| l.contains("verify_internal_token"));
                let success_line_idx = lines.iter().position(|l| {
                    l.contains("Ok(Json") || l.contains("Ok(") || l.contains("StatusCode::OK")
                });
                match (auth_line_idx, success_line_idx) {
                    (Some(auth), Some(success)) if success < auth => {
                        failures.push(format!(
                                "{svc_name}: handler '{handler_name}' returns success (line ~{success}) before auth check (line ~{auth}) — internal routes must reject unauthenticated requests first"
                            ));
                    }
                    (None, Some(_)) => {
                        failures.push(format!(
                            "{svc_name}: handler '{handler_name}' returns success without any verify_internal_token call"
                        ));
                    }
                    _ => {}
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "internal auth ordering contract violated:\n{}",
        failures.join("\n")
    );
}
