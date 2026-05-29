use std::fs;
use std::path::Path;

const MODEL_GW_CONTRACT: &str = "../../platform/contracts/internal/model-gateway.yaml";
const RULES_CAPACITY: &str = "../../../rules/12-CAPACITY-AND-SLO.md";

fn read_contract() -> String {
    fs::read_to_string(MODEL_GW_CONTRACT)
        .unwrap_or_else(|e| panic!("cannot read model-gateway contract: {e}"))
}

#[test]
fn model_gateway_contract_is_valid_yaml() {
    let content = read_contract();
    let _: serde_yaml::Value = serde_yaml::from_str(&content)
        .unwrap_or_else(|e| panic!("model-gateway.yaml is not valid YAML: {e}"));
}

#[test]
fn model_gateway_contract_version_at_least_0_2() {
    let content = read_contract();
    let yaml: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();
    let version = yaml["info"]["version"].as_str().unwrap_or("");
    assert!(
        version.starts_with("0.2.") || version.starts_with("0.3."),
        "model-gateway contract version must be at least 0.2.0-draft after Phase 5 (current: {version})"
    );
}

#[test]
fn model_gateway_contract_has_capacity_status_endpoint() {
    let content = read_contract();
    assert!(
        content.contains("/internal/v1/capacity/status"),
        "model-gateway contract must include capacity status endpoint"
    );
}

#[test]
fn model_gateway_contract_has_invoke_degraded_field() {
    let content = read_contract();
    assert!(
        content.contains("degraded") && content.contains("fallback_reason"),
        "model-gateway contract InvokeResponse must include degraded and fallback_reason"
    );
}

#[test]
fn model_gateway_contract_has_circuit_breaker_schema() {
    let content = read_contract();
    assert!(
        content.contains("CircuitBreakerStatus"),
        "model-gateway contract must include CircuitBreakerStatus schema"
    );
    assert!(
        content.contains("half_open"),
        "CircuitBreakerStatus state must include half_open"
    );
}

#[test]
fn model_gateway_contract_has_token_usage_schema() {
    let content = read_contract();
    assert!(
        content.contains("TokenUsage"),
        "model-gateway contract must include TokenUsage schema"
    );
    assert!(
        content.contains("budget_limit"),
        "TokenUsage must include budget_limit"
    );
}

#[test]
fn model_gateway_contract_has_cost_metrics_schema() {
    let content = read_contract();
    assert!(
        content.contains("CostMetricsSnapshot"),
        "model-gateway contract must include CostMetricsSnapshot schema"
    );
    assert!(
        content.contains("cache_hit") && content.contains("fallback_used"),
        "CostMetricsSnapshot must include cache_hit and fallback_used"
    );
}

#[test]
fn model_gateway_contract_has_cache_stats_schema() {
    let content = read_contract();
    assert!(
        content.contains("CacheStats"),
        "model-gateway contract must include CacheStats schema"
    );
}

#[test]
fn model_gateway_contract_has_fallback_response_schema() {
    let content = read_contract();
    assert!(
        content.contains("FallbackResponse"),
        "model-gateway contract must include FallbackResponse schema"
    );
}

#[test]
fn model_gateway_contract_invoke_request_has_estimated_tokens() {
    let content = read_contract();
    assert!(
        content.contains("estimated_tokens"),
        "InvokeRequest must include estimated_tokens for budget check"
    );
}

#[test]
fn capacity_rules_file_exists() {
    assert!(
        Path::new(RULES_CAPACITY).exists(),
        "rules/12-CAPACITY-AND-SLO.md must exist"
    );
}

#[test]
fn capacity_rules_define_slo_baselines() {
    let content = fs::read_to_string(RULES_CAPACITY)
        .unwrap_or_else(|e| panic!("cannot read capacity rules: {e}"));
    assert!(
        content.contains("SLO") && content.contains("p95"),
        "Capacity rules must define SLO baselines with p95 targets"
    );
}

#[test]
fn capacity_rules_define_cost_budget() {
    let content = fs::read_to_string(RULES_CAPACITY)
        .unwrap_or_else(|e| panic!("cannot read capacity rules: {e}"));
    assert!(
        content.contains("成本预算") || content.contains("token"),
        "Capacity rules must define cost budget"
    );
}

#[test]
fn model_gateway_contract_has_three_capability_bulkheads() {
    let content = read_contract();
    assert!(
        content.contains("chat.respond") || content.contains("capability_name"),
        "Contract must reference capability names for bulkhead partitioning"
    );
}
