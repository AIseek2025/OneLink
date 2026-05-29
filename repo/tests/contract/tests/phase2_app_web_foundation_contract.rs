use std::fs;

const BFF_YAML: &str = "../../platform/contracts/openapi/bff.yaml";
const TOKENS_DTCG: &str = "../../platform/design-system/tokens.json";
const TOKENS_WEB: &str = "../../apps/web/src/design-tokens/tokens.json";
const ANALYTICS_EVENTS_TS: &str = "../../apps/web/src/analytics/events.ts";
const API_CLIENT_TS: &str = "../../apps/web/src/api/client.ts";
const VITE_CONFIG_TS: &str = "../../apps/web/vite.config.ts";
const MOCK_VERIFY_MJS: &str = "../../apps/web/scripts/verify-openapi-mock.mjs";

#[test]
fn phase2_bff_openapi_frozen_version() {
    let content = fs::read_to_string(BFF_YAML).expect("bff.yaml must be readable");
    let yaml: serde_json::Value =
        serde_yaml::from_str(&content).expect("bff.yaml must be valid YAML");

    let version = yaml["info"]["version"]
        .as_str()
        .expect("bff.yaml must have info.version");
    assert!(
        version >= "1.1.0",
        "BFF OpenAPI must be frozen (>= 1.1.0), got {version}"
    );

    let description = yaml["info"]["description"].as_str().unwrap_or("");
    assert!(
        description.contains("冻结") || description.contains("freeze"),
        "BFF OpenAPI must contain freeze declaration"
    );
}

#[test]
fn phase2_bff_phase2_endpoints_present() {
    let content = fs::read_to_string(BFF_YAML).expect("bff.yaml must be readable");
    let yaml: serde_json::Value =
        serde_yaml::from_str(&content).expect("bff.yaml must be valid YAML");

    let paths = yaml["paths"].as_object().expect("paths must be an object");

    let phase2_endpoints = [
        ("/api/v1/bff/chat/init", "get"),
        ("/api/v1/bff/onboarding", "get"),
        ("/api/v1/bff/home", "get"),
        ("/api/v1/bff/profile/{userId}", "get"),
        ("/api/v1/bff/profile/me", "patch"),
        ("/api/v1/bff/find/intent", "post"),
        ("/api/v1/bff/analytics/events", "post"),
    ];

    for (path, method) in &phase2_endpoints {
        assert!(
            paths.contains_key(*path),
            "Phase 2 BFF endpoint missing: {path}"
        );
        let path_obj = paths[*path].as_object().unwrap();
        assert!(
            path_obj.contains_key(*method),
            "Phase 2 BFF endpoint {path} missing method {method}"
        );
    }
}

#[test]
fn phase2_bff_schemas_cover_app_web_needs() {
    let content = fs::read_to_string(BFF_YAML).expect("bff.yaml must be readable");
    let yaml: serde_json::Value =
        serde_yaml::from_str(&content).expect("bff.yaml must be valid YAML");

    let schemas = yaml["components"]["schemas"]
        .as_object()
        .expect("components.schemas must exist");

    let required_phase2_schemas = [
        "ChatInitResponse",
        "OnboardingResponse",
        "HomeResponse",
        "ProfileResponse",
        "ProfileSummary",
        "ProfileMePatchRequest",
        "FindIntentRequest",
        "FindIntentResponse",
        "AnalyticsEvent",
        "UserSummary",
        "PendingQuestionItem",
        "QuestionnaireProgress",
        "ProfileCompletion",
    ];

    for name in &required_phase2_schemas {
        assert!(
            schemas.contains_key(*name),
            "Phase 2 required schema missing: {name}"
        );
    }
}

#[test]
fn phase2_design_tokens_dtcg_exists() {
    let content =
        fs::read_to_string(TOKENS_DTCG).expect("platform/design-system/tokens.json must exist");
    let tokens: serde_json::Value =
        serde_json::from_str(&content).expect("tokens.json must be valid JSON");

    assert!(
        tokens["color"].is_object(),
        "Design tokens must have color section"
    );
    assert!(
        tokens["color"]["brand"]["primary"].is_object()
            || tokens["color"]["brand"]["primary"]["$value"].is_string(),
        "Design tokens must have color.brand.primary"
    );
    assert!(
        tokens["spacing"].is_object(),
        "Design tokens must have spacing section"
    );
    assert!(
        tokens["typography"].is_object(),
        "Design tokens must have typography section"
    );
    assert!(
        tokens["radius"].is_object(),
        "Design tokens must have radius section"
    );
    assert!(
        tokens["shadow"].is_object(),
        "Design tokens must have shadow section"
    );
    assert!(
        tokens["breakpoint"].is_object(),
        "Design tokens must have breakpoint section"
    );
    assert!(
        tokens["z-index"].is_object(),
        "Design tokens must have z-index section"
    );
}

#[test]
fn phase2_design_tokens_web_export_exists() {
    let content =
        fs::read_to_string(TOKENS_WEB).expect("apps/web/src/design-tokens/tokens.json must exist");
    let web_tokens: serde_json::Value =
        serde_json::from_str(&content).expect("web tokens.json must be valid JSON");

    assert!(
        web_tokens["tokens"]["color"].is_object(),
        "Web tokens must have color section"
    );
    assert!(
        web_tokens["tokens"]["color"]["brand"]["primary"].is_string(),
        "Web tokens must have color.brand.primary as string"
    );
    assert!(
        web_tokens["tokens"]["typography"].is_object(),
        "Web tokens must have typography section"
    );
    assert!(
        web_tokens["tokens"]["spacing"].is_object(),
        "Web tokens must have spacing section"
    );
}

#[test]
fn phase2_design_tokens_dtcg_and_web_consistent() {
    let dtcg_content = fs::read_to_string(TOKENS_DTCG).expect("DTCG tokens must be readable");
    let dtcg: serde_json::Value =
        serde_json::from_str(&dtcg_content).expect("DTCG tokens must be valid JSON");

    let web_content = fs::read_to_string(TOKENS_WEB).expect("Web tokens must be readable");
    let web: serde_json::Value =
        serde_json::from_str(&web_content).expect("Web tokens must be valid JSON");

    let dtcg_primary = dtcg["color"]["brand"]["primary"]["$value"]
        .as_str()
        .unwrap_or("");
    let web_primary = web["tokens"]["color"]["brand"]["primary"]
        .as_str()
        .unwrap_or("");
    assert_eq!(
        dtcg_primary, web_primary,
        "DTCG and web tokens must agree on color.brand.primary"
    );

    let dtcg_secondary = dtcg["color"]["brand"]["secondary"]["$value"]
        .as_str()
        .unwrap_or("");
    let web_secondary = web["tokens"]["color"]["brand"]["secondary"]
        .as_str()
        .unwrap_or("");
    assert_eq!(
        dtcg_secondary, web_secondary,
        "DTCG and web tokens must agree on color.brand.secondary"
    );

    let dtcg_error = dtcg["color"]["semantic"]["error"]["$value"]
        .as_str()
        .unwrap_or("");
    let web_error = web["tokens"]["color"]["semantic"]["error"]
        .as_str()
        .unwrap_or("");
    assert_eq!(
        dtcg_error, web_error,
        "DTCG and web tokens must agree on color.semantic.error"
    );
}

#[test]
fn phase2_analytics_event_catalog_exists() {
    let content = fs::read_to_string(ANALYTICS_EVENTS_TS).expect("analytics/events.ts must exist");

    let required_events = [
        "page.view",
        "registration.started",
        "registration.completed",
        "login.started",
        "login.completed",
        "chat.message.sent",
        "chat.message.received",
        "profile.confirmation.viewed",
        "profile.fact.confirmed",
        "profile.fact.dismissed",
        "find.intent.submitted",
        "error.occurred",
    ];

    for event_name in &required_events {
        assert!(
            content.contains(event_name),
            "Analytics event catalog must define: {event_name}"
        );
    }

    assert!(
        content.contains("trackEvent"),
        "Analytics module must export trackEvent function"
    );
    assert!(
        content.contains("AnalyticsContext"),
        "Analytics module must define AnalyticsContext type"
    );
    assert!(
        content.contains("platform") && content.contains("web"),
        "Analytics events must include platform field with web value"
    );
}

#[test]
fn phase2_analytics_events_match_bff_schema() {
    let bff_content = fs::read_to_string(BFF_YAML).expect("bff.yaml must be readable");
    let yaml: serde_json::Value =
        serde_yaml::from_str(&bff_content).expect("bff.yaml must be valid YAML");

    let analytics_schema = &yaml["components"]["schemas"]["AnalyticsEvent"];
    assert!(
        analytics_schema.is_object(),
        "BFF OpenAPI must define AnalyticsEvent schema"
    );

    let required_fields = analytics_schema["required"]
        .as_array()
        .expect("AnalyticsEvent must have required fields");
    assert!(
        required_fields
            .iter()
            .any(|v| v.as_str() == Some("event_name")),
        "AnalyticsEvent must require event_name"
    );
    assert!(
        required_fields
            .iter()
            .any(|v| v.as_str() == Some("occurred_at")),
        "AnalyticsEvent must require occurred_at"
    );

    let props = analytics_schema["properties"]
        .as_object()
        .expect("AnalyticsEvent must have properties");
    assert!(
        props.contains_key("platform"),
        "AnalyticsEvent must have platform property"
    );
    assert!(
        props.contains_key("user_id"),
        "AnalyticsEvent must have user_id property"
    );
}

#[test]
fn phase2_api_client_covers_phase2_endpoints() {
    let content = fs::read_to_string(API_CLIENT_TS).expect("api/client.ts must exist");

    let required_client_functions = [
        "register",
        "login",
        "fetchHome",
        "fetchChatInit",
        "fetchProfile",
        "submitFindIntent",
        "patchProfile",
    ];

    for func_name in &required_client_functions {
        assert!(
            content.contains(&format!("export async function {func_name}")),
            "API client must export function: {func_name}"
        );
    }

    assert!(
        content.contains("localStorage") && content.contains("onelink_token"),
        "API client must read auth token from localStorage"
    );
    assert!(
        content.contains("Authorization") && content.contains("Bearer"),
        "API client must send Bearer auth header"
    );
}

#[test]
fn phase2_vite_proxy_configured_for_bff() {
    let content = fs::read_to_string(VITE_CONFIG_TS).expect("vite.config.ts must exist");

    assert!(
        content.contains("proxy"),
        "Vite config must have proxy configuration"
    );
    assert!(
        content.contains("/api/v1"),
        "Vite proxy must cover /api/v1 paths"
    );
    assert!(
        content.contains("8083"),
        "Vite proxy must target BFF default port 8083"
    );
}

#[test]
fn phase2_mock_verification_script_exists() {
    let content = fs::read_to_string(MOCK_VERIFY_MJS).expect("verify-openapi-mock.mjs must exist");

    assert!(
        content.contains("bff.yaml"),
        "Mock verification must reference bff.yaml"
    );
    assert!(
        content.contains("extractBffEndpoints"),
        "Mock verification must extract BFF endpoints"
    );
    assert!(
        content.contains("PASS"),
        "Mock verification must produce PASS result"
    );
}

#[test]
fn phase2_web_pages_cover_phase2_flows() {
    let pages_dir =
        fs::read_dir("../../apps/web/src/pages").expect("apps/web/src/pages directory must exist");
    let page_names: Vec<String> = pages_dir
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
        .collect();

    let required_pages = [
        "LoginPage.tsx",
        "ChatPage.tsx",
        "ProfilePage.tsx",
        "FindPage.tsx",
        "HomePage.tsx",
    ];

    for page in &required_pages {
        assert!(
            page_names.iter().any(|p| p == page),
            "Web app must have page: {page}"
        );
    }
}

#[test]
fn phase2_web_app_uses_design_tokens() {
    let pages = [
        ("LoginPage.tsx", "../../apps/web/src/pages/LoginPage.tsx"),
        ("ChatPage.tsx", "../../apps/web/src/pages/ChatPage.tsx"),
        ("HomePage.tsx", "../../apps/web/src/pages/HomePage.tsx"),
        (
            "ProfilePage.tsx",
            "../../apps/web/src/pages/ProfilePage.tsx",
        ),
        ("FindPage.tsx", "../../apps/web/src/pages/FindPage.tsx"),
    ];

    for (name, path) in &pages {
        let content =
            fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {name}: {e}"));
        assert!(
            content.contains("design-tokens") || content.contains("tokens"),
            "{name} must import and use design tokens"
        );
    }
}

#[test]
fn phase2_web_app_tracks_analytics() {
    let pages = [
        ("LoginPage.tsx", "../../apps/web/src/pages/LoginPage.tsx"),
        ("ChatPage.tsx", "../../apps/web/src/pages/ChatPage.tsx"),
        ("HomePage.tsx", "../../apps/web/src/pages/HomePage.tsx"),
        (
            "ProfilePage.tsx",
            "../../apps/web/src/pages/ProfilePage.tsx",
        ),
        ("FindPage.tsx", "../../apps/web/src/pages/FindPage.tsx"),
    ];

    for (name, path) in &pages {
        let content =
            fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {name}: {e}"));
        assert!(
            content.contains("trackEvent"),
            "{name} must call trackEvent for analytics"
        );
    }
}
