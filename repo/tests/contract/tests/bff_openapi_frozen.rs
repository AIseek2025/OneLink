use std::fs;

#[test]
fn bff_openapi_frozen_endpoints_unchanged() {
    let content = fs::read_to_string("../../platform/contracts/openapi/bff.yaml")
        .expect("bff.yaml must be readable");

    let yaml: serde_json::Value =
        serde_yaml::from_str(&content).expect("bff.yaml must be valid YAML");

    let version = yaml["info"]["version"]
        .as_str()
        .expect("bff.yaml must have info.version");
    assert!(
        version >= "1.1.0",
        "BFF OpenAPI version must be >= 1.1.0 (frozen), got {version}"
    );

    let description = yaml["info"]["description"].as_str().unwrap_or("");
    assert!(
        description.contains("冻结") || description.contains("freeze"),
        "BFF OpenAPI must contain freeze declaration in description"
    );

    let paths = yaml["paths"].as_object().expect("paths must be an object");

    let required_frozen = [
        ("/api/v1/bff/chat/init", "get"),
        ("/api/v1/bff/onboarding", "get"),
        ("/api/v1/bff/home", "get"),
        ("/api/v1/bff/profile/{userId}", "get"),
        ("/api/v1/bff/profile/me", "patch"),
        ("/api/v1/bff/find/intent", "post"),
        ("/api/v1/bff/analytics/events", "post"),
        ("/api/v1/bff/find/results", "get"),
        ("/api/v1/bff/dm/list", "get"),
        ("/api/v1/bff/dm/send", "post"),
        ("/api/v1/bff/safety/report", "post"),
        ("/api/v1/bff/safety/block", "post"),
    ];

    for (path, method) in &required_frozen {
        assert!(
            paths.contains_key(*path),
            "BFF OpenAPI frozen endpoint missing: {path}"
        );
        let path_obj = paths[*path].as_object().unwrap();
        assert!(
            path_obj.contains_key(*method),
            "BFF OpenAPI frozen endpoint {path} missing method {method}"
        );
    }

    assert!(
        paths.len() >= required_frozen.len(),
        "BFF OpenAPI must have at least {} paths, found {}",
        required_frozen.len(),
        paths.len()
    );

    let find_intent = &paths["/api/v1/bff/find/intent"]["post"];
    let req_body = find_intent["requestBody"].as_object();
    assert!(
        req_body.is_some(),
        "POST /api/v1/bff/find/intent must have requestBody"
    );
    let req_schema = req_body.unwrap()["content"]["application/json"]["schema"].as_object();
    assert!(
        req_schema.is_some(),
        "POST /api/v1/bff/find/intent must have JSON request schema"
    );

    let find_resp = find_intent["responses"]["202"].as_object();
    assert!(
        find_resp.is_some(),
        "POST /api/v1/bff/find/intent must have 202 response"
    );
}

#[test]
fn bff_openapi_no_breaking_schema_changes() {
    let content = fs::read_to_string("../../platform/contracts/openapi/bff.yaml")
        .expect("bff.yaml must be readable");
    let yaml: serde_json::Value =
        serde_yaml::from_str(&content).expect("bff.yaml must be valid YAML");

    let schemas = yaml["components"]["schemas"]
        .as_object()
        .expect("components.schemas must exist");

    let required_schemas = [
        "ChatInitResponse",
        "OnboardingResponse",
        "HomeResponse",
        "ProfileResponse",
        "FindIntentRequest",
        "FindIntentResponse",
        "UserSummary",
        "ProfileSummary",
        "ProfileMePatchRequest",
        "AnalyticsEvent",
        "DmSendRequest",
        "DmSendResponse",
        "SafetyReportRequest",
        "SafetyReportResponse",
        "SafetyBlockRequest",
        "SafetyBlockResponse",
    ];

    for name in &required_schemas {
        assert!(
            schemas.contains_key(*name),
            "BFF OpenAPI frozen schema missing: {name}"
        );
    }

    let find_req = &schemas["FindIntentRequest"];
    let req_props = find_req["properties"].as_object().unwrap();
    assert!(
        req_props.contains_key("raw_query"),
        "FindIntentRequest must have raw_query property"
    );
    let required_fields = find_req["required"]
        .as_array()
        .expect("FindIntentRequest must have required fields");
    assert!(
        required_fields
            .iter()
            .any(|v| v.as_str() == Some("raw_query")),
        "raw_query must be required in FindIntentRequest"
    );

    let find_resp = &schemas["FindIntentResponse"];
    let resp_props = find_resp["properties"].as_object().unwrap();
    assert!(
        resp_props.contains_key("find_request_id"),
        "FindIntentResponse must have find_request_id"
    );
    assert!(
        resp_props.contains_key("status"),
        "FindIntentResponse must have status"
    );
}

#[test]
fn bff_openapi_analytics_event_enum_frozen() {
    let content = fs::read_to_string("../../platform/contracts/openapi/bff.yaml")
        .expect("bff.yaml must be readable");
    let yaml: serde_json::Value =
        serde_yaml::from_str(&content).expect("bff.yaml must be valid YAML");

    let schemas = yaml["components"]["schemas"]
        .as_object()
        .expect("components.schemas must exist");

    let analytics_event = &schemas["AnalyticsEvent"];
    assert!(
        analytics_event.is_object(),
        "AnalyticsEvent schema must exist"
    );

    let event_name_prop = &analytics_event["properties"]["event_name"];
    assert!(
        event_name_prop.is_object(),
        "AnalyticsEvent must have event_name property"
    );

    let enum_values = event_name_prop["enum"]
        .as_array()
        .expect("event_name must have enum values");

    let frozen_event_names = [
        "app_boot_started",
        "app_boot_finished",
        "app_boot_failed",
        "login_submit",
        "login_success",
        "login_failed",
        "register_submit",
        "register_success",
        "register_failed",
        "conversation_list_view",
        "conversation_open",
        "conversation_create",
        "chat_view",
        "chat_send",
        "chat_reply_received",
        "chat_retry",
        "profile_fact_exposed",
        "profile_fact_accept",
        "profile_fact_reject",
        "profile_fact_snooze",
        "settings_view",
        "settings_section_open",
        "find_request_started",
        "find_request_submitted",
        "find_request_failed",
        "clarification_view",
        "clarification_submit",
        "recommendation_list_view",
        "recommendation_card_exposed",
        "recommendation_detail_view",
        "recommendation_explanation_view",
        "recommendation_connect_start",
        "recommendation_feedback_open",
        "recommendation_feedback_submit",
        "dm_first_message_submit",
        "dm_first_message_approved",
        "dm_first_message_blocked",
        "report_open",
        "report_submit",
        "locale_setting_view",
        "locale_setting_save",
        "data_rights_view",
        "data_export_request",
        "data_delete_request",
        "data_correction_request",
    ];

    let enum_strs: Vec<&str> = enum_values.iter().filter_map(|v| v.as_str()).collect();

    for name in &frozen_event_names {
        assert!(
            enum_strs.contains(name),
            "AnalyticsEvent.event_name enum missing frozen value: {name}"
        );
    }

    assert!(
        enum_strs.len() >= frozen_event_names.len(),
        "AnalyticsEvent.event_name enum must have at least {} values, found {}",
        frozen_event_names.len(),
        enum_strs.len()
    );

    let required_fields = analytics_event["required"]
        .as_array()
        .expect("AnalyticsEvent must have required fields");
    assert!(
        required_fields
            .iter()
            .any(|v| v.as_str() == Some("event_name")),
        "event_name must be required in AnalyticsEvent"
    );
    assert!(
        required_fields
            .iter()
            .any(|v| v.as_str() == Some("occurred_at")),
        "occurred_at must be required in AnalyticsEvent"
    );
}
