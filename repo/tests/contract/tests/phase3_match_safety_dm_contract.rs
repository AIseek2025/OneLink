use std::fs;

const BFF_ROUTES_PATH: &str = "../../services/bff/src/http/routes.rs";
const BFF_OPENAPI_PATH: &str = "../../platform/contracts/openapi/bff.yaml";
const MATCH_ROUTES_PATH: &str = "../../services/match-service/src/http/routes.rs";
const SAFETY_ROUTES_PATH: &str = "../../services/safety-service/src/http/routes.rs";
const DM_ROUTES_PATH: &str = "../../services/dm-service/src/http/routes.rs";

#[test]
fn bff_dm_send_route_exists() {
    let content = fs::read_to_string(BFF_ROUTES_PATH)
        .unwrap_or_else(|e| panic!("cannot read {BFF_ROUTES_PATH}: {e}"));
    assert!(
        content.contains("\"/api/v1/bff/dm/send\""),
        "BFF must have POST /api/v1/bff/dm/send route"
    );
    assert!(content.contains("dm_send"), "BFF must have dm_send handler");
}

#[test]
fn bff_safety_report_route_exists() {
    let content = fs::read_to_string(BFF_ROUTES_PATH)
        .unwrap_or_else(|e| panic!("cannot read {BFF_ROUTES_PATH}: {e}"));
    assert!(
        content.contains("\"/api/v1/bff/safety/report\""),
        "BFF must have POST /api/v1/bff/safety/report route"
    );
    assert!(
        content.contains("safety_report"),
        "BFF must have safety_report handler"
    );
}

#[test]
fn bff_safety_block_route_exists() {
    let content = fs::read_to_string(BFF_ROUTES_PATH)
        .unwrap_or_else(|e| panic!("cannot read {BFF_ROUTES_PATH}: {e}"));
    assert!(
        content.contains("\"/api/v1/bff/safety/block\""),
        "BFF must have POST /api/v1/bff/safety/block route"
    );
    assert!(
        content.contains("safety_block"),
        "BFF must have safety_block handler"
    );
}

#[test]
fn bff_dm_send_calls_safety_review_for_new_threads() {
    let content = fs::read_to_string(BFF_ROUTES_PATH)
        .unwrap_or_else(|e| panic!("cannot read {BFF_ROUTES_PATH}: {e}"));
    assert!(
        content.contains("dm-first-message-review") || content.contains("dm_first_message_review"),
        "BFF dm_send must call safety-service dm-first-message-review for new threads"
    );
    assert!(
        content.contains("is_new_thread") || content.contains("created"),
        "BFF dm_send must check whether thread is new before safety review"
    );
}

#[test]
fn bff_dm_send_requires_bearer_auth() {
    let content = fs::read_to_string(BFF_ROUTES_PATH)
        .unwrap_or_else(|e| panic!("cannot read {BFF_ROUTES_PATH}: {e}"));
    let dm_send_start = content
        .find("async fn dm_send")
        .expect("dm_send handler must exist");
    let after_sig = &content[dm_send_start + "async fn dm_send".len()..];
    let dm_send_end = after_sig
        .find("\nasync fn ")
        .map(|i| dm_send_start + "async fn dm_send".len() + i)
        .unwrap_or(content.len());
    let dm_send_body = &content[dm_send_start..dm_send_end];
    assert!(
        dm_send_body.contains("extract_auth"),
        "dm_send handler must call extract_auth to verify Bearer token"
    );
}

#[test]
fn bff_safety_report_injects_reporter_from_identity() {
    let content = fs::read_to_string(BFF_ROUTES_PATH)
        .unwrap_or_else(|e| panic!("cannot read {BFF_ROUTES_PATH}: {e}"));
    let handler_start = content
        .find("async fn safety_report")
        .expect("safety_report handler must exist");
    let after_sig = &content[handler_start + "async fn safety_report".len()..];
    let handler_end = after_sig
        .find("\nasync fn ")
        .map(|i| handler_start + "async fn safety_report".len() + i)
        .unwrap_or(content.len());
    let handler_body = &content[handler_start..handler_end];
    assert!(
        handler_body.contains("identity_user_json"),
        "safety_report must validate identity before creating report"
    );
    assert!(
        handler_body.contains("reporter_user_id"),
        "safety_report must inject reporter_user_id from identity, not from client payload"
    );
}

#[test]
fn bff_safety_block_injects_blocker_from_identity() {
    let content = fs::read_to_string(BFF_ROUTES_PATH)
        .unwrap_or_else(|e| panic!("cannot read {BFF_ROUTES_PATH}: {e}"));
    let handler_start = content
        .find("async fn safety_block")
        .expect("safety_block handler must exist");
    let after_sig = &content[handler_start + "async fn safety_block".len()..];
    let handler_end = after_sig
        .find("\nasync fn ")
        .map(|i| handler_start + "async fn safety_block".len() + i)
        .unwrap_or(content.len());
    let handler_body = &content[handler_start..handler_end];
    assert!(
        handler_body.contains("identity_user_json"),
        "safety_block must validate identity before creating block"
    );
    assert!(
        handler_body.contains("user_id"),
        "safety_block must inject user_id from identity, not from client payload"
    );
}

#[test]
fn match_service_has_find_request_and_candidate_routes() {
    let content = fs::read_to_string(MATCH_ROUTES_PATH)
        .unwrap_or_else(|e| panic!("cannot read {MATCH_ROUTES_PATH}: {e}"));
    assert!(
        content.contains("/api/v1/match/find-requests"),
        "match-service must have /api/v1/match/find-requests route"
    );
    assert!(
        content.contains("/candidates"),
        "match-service must have candidates route"
    );
    assert!(
        content.contains("/feedback"),
        "match-service must have feedback route"
    );
}

#[test]
fn safety_service_has_screen_report_block_and_dm_review_routes() {
    let content = fs::read_to_string(SAFETY_ROUTES_PATH)
        .unwrap_or_else(|e| panic!("cannot read {SAFETY_ROUTES_PATH}: {e}"));
    assert!(
        content.contains("/api/v1/safety/screen-message"),
        "safety-service must have screen-message route"
    );
    assert!(
        content.contains("/api/v1/safety/reports"),
        "safety-service must have reports route"
    );
    assert!(
        content.contains("/api/v1/safety/blocks"),
        "safety-service must have blocks route"
    );
    assert!(
        content.contains("/api/v1/safety/dm-first-message-review"),
        "safety-service must have dm-first-message-review route"
    );
}

#[test]
fn dm_service_has_thread_and_message_routes() {
    let content = fs::read_to_string(DM_ROUTES_PATH)
        .unwrap_or_else(|e| panic!("cannot read {DM_ROUTES_PATH}: {e}"));
    assert!(
        content.contains("/api/v1/dm/threads"),
        "dm-service must have threads route"
    );
    assert!(
        content.contains("/messages"),
        "dm-service must have messages route"
    );
    assert!(
        content.contains("/read"),
        "dm-service must have mark-read route"
    );
}

#[test]
fn safety_service_risk_scoring_blocks_high_risk_first_messages() {
    let content = fs::read_to_string(SAFETY_ROUTES_PATH)
        .unwrap_or_else(|e| panic!("cannot read {SAFETY_ROUTES_PATH}: {e}"));
    assert!(
        content.contains("compute_risk_score"),
        "safety-service must have compute_risk_score function"
    );
    assert!(
        content.contains("is_first_message"),
        "safety-service screen_message must consider is_first_message flag"
    );
    assert!(
        content.contains("ContentVerdict"),
        "safety-service must have ContentVerdict enum for screening results"
    );
}

#[test]
fn openapi_phase3_schemas_are_complete() {
    let content = fs::read_to_string(BFF_OPENAPI_PATH)
        .unwrap_or_else(|e| panic!("cannot read {BFF_OPENAPI_PATH}: {e}"));
    let yaml: serde_json::Value =
        serde_yaml::from_str(&content).expect("bff.yaml must be valid YAML");

    let schemas = yaml["components"]["schemas"]
        .as_object()
        .expect("components.schemas must exist");

    let phase3_schemas = [
        "DmSendRequest",
        "DmSendResponse",
        "SafetyReportRequest",
        "SafetyReportResponse",
        "SafetyBlockRequest",
        "SafetyBlockResponse",
    ];

    for name in &phase3_schemas {
        assert!(
            schemas.contains_key(*name),
            "Phase 3 BFF OpenAPI schema missing: {name}"
        );
    }
}

#[test]
fn match_service_has_match_routes() {
    let content = fs::read_to_string(MATCH_ROUTES_PATH)
        .unwrap_or_else(|e| panic!("cannot read {MATCH_ROUTES_PATH}: {e}"));
    assert!(
        content.contains("/api/v1/match/matches"),
        "match-service must have /api/v1/match/matches route"
    );
    assert!(
        content.contains("MatchRecord"),
        "match-service must have MatchRecord struct"
    );
    assert!(
        content.contains("MutualLike"),
        "match-service must support MutualLike match type"
    );
    assert!(
        content.contains("like_index"),
        "match-service must have like_index for mutual-like detection"
    );
}

#[test]
fn match_service_feedback_creates_mutual_match() {
    let content = fs::read_to_string(MATCH_ROUTES_PATH)
        .unwrap_or_else(|e| panic!("cannot read {MATCH_ROUTES_PATH}: {e}"));
    assert!(
        content.contains("match_created"),
        "match-service submit_feedback response must include match_created field"
    );
    assert!(
        content.contains("match_id"),
        "match-service submit_feedback response must include match_id field"
    );
    assert!(
        content.contains("is_mutual"),
        "match-service must detect mutual likes via like_index"
    );
}

#[test]
fn safety_service_has_risk_control_routes() {
    let content = fs::read_to_string(SAFETY_ROUTES_PATH)
        .unwrap_or_else(|e| panic!("cannot read {SAFETY_ROUTES_PATH}: {e}"));
    assert!(
        content.contains("/api/v1/safety/risk-flags"),
        "safety-service must have /api/v1/safety/risk-flags route"
    );
    assert!(
        content.contains("RiskFlagRecord"),
        "safety-service must have RiskFlagRecord struct"
    );
    assert!(
        content.contains("RiskSeverity"),
        "safety-service must have RiskSeverity enum"
    );
}

#[test]
fn safety_service_report_action_closed_loop() {
    let content = fs::read_to_string(SAFETY_ROUTES_PATH)
        .unwrap_or_else(|e| panic!("cannot read {SAFETY_ROUTES_PATH}: {e}"));
    assert!(
        content.contains("/actions"),
        "safety-service must have report actions route"
    );
    assert!(
        content.contains("ReportActionRecord"),
        "safety-service must have ReportActionRecord struct"
    );
    assert!(
        content.contains("ReportActionType"),
        "safety-service must have ReportActionType enum"
    );
    assert!(
        content.contains("EscalatedToHuman"),
        "safety-service must support EscalatedToHuman action type"
    );
}

#[test]
fn dm_service_has_thread_lifecycle_routes() {
    let content = fs::read_to_string(DM_ROUTES_PATH)
        .unwrap_or_else(|e| panic!("cannot read {DM_ROUTES_PATH}: {e}"));
    assert!(
        content.contains("/archive"),
        "dm-service must have thread archive route"
    );
    assert!(
        content.contains("DmThreadStatus"),
        "dm-service must have DmThreadStatus enum"
    );
    assert!(
        content.contains("Archived"),
        "dm-service DmThreadStatus must include Archived variant"
    );
    assert!(
        content.contains("Blocked"),
        "dm-service DmThreadStatus must include Blocked variant"
    );
}

#[test]
fn dm_service_has_safety_screening_log() {
    let content = fs::read_to_string(DM_ROUTES_PATH)
        .unwrap_or_else(|e| panic!("cannot read {DM_ROUTES_PATH}: {e}"));
    assert!(
        content.contains("/screening-log"),
        "dm-service must have screening-log route"
    );
    assert!(
        content.contains("SafetyScreeningRecord"),
        "dm-service must have SafetyScreeningRecord struct"
    );
    assert!(
        content.contains("safety_screened"),
        "dm-service DmMessage must have safety_screened field"
    );
}

#[test]
fn openapi_phase3_endpoints_have_security() {
    let content = fs::read_to_string(BFF_OPENAPI_PATH)
        .unwrap_or_else(|e| panic!("cannot read {BFF_OPENAPI_PATH}: {e}"));
    let yaml: serde_json::Value =
        serde_yaml::from_str(&content).expect("bff.yaml must be valid YAML");

    let paths = yaml["paths"].as_object().expect("paths must be an object");

    let phase3_endpoints = [
        "/api/v1/bff/dm/send",
        "/api/v1/bff/safety/report",
        "/api/v1/bff/safety/block",
        "/api/v1/bff/dm/list",
        "/api/v1/bff/find/results",
    ];

    for path in &phase3_endpoints {
        if let Some(path_obj) = paths.get(*path) {
            for (method, method_obj) in path_obj.as_object().unwrap() {
                if method == "get" || method == "post" || method == "patch" {
                    let has_security = method_obj
                        .get("security")
                        .and_then(|s| s.as_array())
                        .map(|arr| !arr.is_empty())
                        .unwrap_or(false);
                    assert!(
                        has_security,
                        "Phase 3 endpoint {path} {method} must have security requirement (bearerAuth)"
                    );
                }
            }
        }
    }
}
