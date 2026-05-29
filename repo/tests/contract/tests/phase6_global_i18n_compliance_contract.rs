use std::fs;
use std::path::Path;

const BFF_CONTRACT: &str = "../../platform/contracts/openapi/bff.yaml";
const MODEL_GW_CONTRACT: &str = "../../platform/contracts/internal/model-gateway.yaml";
const RULES_I18N: &str = "../../../rules/13-GLOBAL-I18N-AND-COMPLIANCE.md";

fn read_bff_contract() -> String {
    fs::read_to_string(BFF_CONTRACT).unwrap_or_else(|e| panic!("cannot read bff contract: {e}"))
}

fn read_model_gw_contract() -> String {
    fs::read_to_string(MODEL_GW_CONTRACT)
        .unwrap_or_else(|e| panic!("cannot read model-gateway contract: {e}"))
}

fn read_i18n_rules() -> String {
    fs::read_to_string(RULES_I18N).unwrap_or_else(|e| panic!("cannot read i18n rules: {e}"))
}

#[test]
fn bff_contract_is_valid_yaml() {
    let content = read_bff_contract();
    let _: serde_yaml::Value = serde_yaml::from_str(&content)
        .unwrap_or_else(|e| panic!("bff.yaml is not valid YAML: {e}"));
}

#[test]
fn bff_contract_version_is_at_least_1_6() {
    let content = read_bff_contract();
    let version_line = content
        .lines()
        .find(|l| l.trim().starts_with("version:"))
        .expect("bff contract must have a version field");
    let version_str = version_line.trim().strip_prefix("version:").unwrap().trim();
    let parts: Vec<u32> = version_str
        .split('.')
        .filter_map(|p| p.parse().ok())
        .collect();
    assert!(
        parts.len() >= 2 && (parts[0] > 1 || (parts[0] == 1 && parts[1] >= 6)),
        "bff contract version must be >= 1.6.0 after Phase 6, got {version_str}"
    );
}

#[test]
fn bff_contract_user_summary_has_content_language() {
    let content = read_bff_contract();
    assert!(
        content.contains("content_language"),
        "UserSummary must include content_language field"
    );
}

#[test]
fn bff_contract_user_summary_has_notification_language() {
    let content = read_bff_contract();
    assert!(
        content.contains("notification_language"),
        "UserSummary must include notification_language field"
    );
}

#[test]
fn bff_contract_has_i18n_config_schema() {
    let content = read_bff_contract();
    assert!(
        content.contains("I18NConfig"),
        "bff contract must include I18NConfig schema"
    );
}

#[test]
fn bff_contract_has_data_residency_decision_schema() {
    let content = read_bff_contract();
    assert!(
        content.contains("DataResidencyDecision"),
        "bff contract must include DataResidencyDecision schema"
    );
}

#[test]
fn bff_contract_has_compliance_policy_summary_schema() {
    let content = read_bff_contract();
    assert!(
        content.contains("CompliancePolicySummary"),
        "bff contract must include CompliancePolicySummary schema"
    );
}

#[test]
fn bff_contract_i18n_config_has_supported_locales() {
    let content = read_bff_contract();
    assert!(
        content.contains("supported_locales") && content.contains("default_locale"),
        "I18NConfig must include supported_locales and default_locale"
    );
}

#[test]
fn bff_contract_compliance_policy_has_user_rights() {
    let content = read_bff_contract();
    assert!(
        content.contains("view_data")
            && content.contains("export_data")
            && content.contains("correct_data")
            && content.contains("delete_data"),
        "CompliancePolicySummary must enumerate all four user rights"
    );
}

#[test]
fn bff_contract_compliance_policy_blocks_unauthorized_find_person() {
    let content = read_bff_contract();
    assert!(
        content.contains("unauthorized_find_person_blocked"),
        "CompliancePolicySummary must include unauthorized_find_person_blocked"
    );
}

#[test]
fn bff_contract_compliance_policy_blocks_unauthorized_data_collection() {
    let content = read_bff_contract();
    assert!(
        content.contains("unauthorized_data_collection_blocked"),
        "CompliancePolicySummary must include unauthorized_data_collection_blocked"
    );
}

#[test]
fn model_gateway_contract_version_bumped_to_0_3() {
    let content = read_model_gw_contract();
    assert!(
        content.contains("version: 0.3.0-draft"),
        "model-gateway contract must be version 0.3.0-draft after Phase 6"
    );
}

#[test]
fn model_gateway_contract_invoke_request_has_locale_hint() {
    let content = read_model_gw_contract();
    assert!(
        content.contains("locale_hint"),
        "InvokeRequest must include locale_hint for i18n"
    );
}

#[test]
fn model_gateway_contract_has_i18n_policy_schema() {
    let content = read_model_gw_contract();
    assert!(
        content.contains("I18NPolicy"),
        "model-gateway contract must include I18NPolicy schema"
    );
}

#[test]
fn model_gateway_contract_has_residency_config_schema() {
    let content = read_model_gw_contract();
    assert!(
        content.contains("ResidencyConfig"),
        "model-gateway contract must include ResidencyConfig schema"
    );
}

#[test]
fn model_gateway_contract_residency_config_has_degradation_mode() {
    let content = read_model_gw_contract();
    assert!(
        content.contains("read_only")
            && content.contains("delayed_sync")
            && content.contains("standby_region"),
        "ResidencyConfig degradation_mode must include read_only, delayed_sync, standby_region"
    );
}

#[test]
fn i18n_rules_file_exists() {
    assert!(
        Path::new(RULES_I18N).exists(),
        "rules/13-GLOBAL-I18N-AND-COMPLIANCE.md must exist"
    );
}

#[test]
fn i18n_rules_define_multi_language_principles() {
    let content = read_i18n_rules();
    assert!(
        content.contains("多语言") || content.contains("multi-language"),
        "i18n rules must define multi-language principles"
    );
}

#[test]
fn i18n_rules_define_data_residency() {
    let content = read_i18n_rules();
    assert!(
        content.contains("数据驻留") || content.contains("data residency"),
        "i18n rules must define data residency requirements"
    );
}

#[test]
fn i18n_rules_define_compliance_baseline() {
    let content = read_i18n_rules();
    assert!(
        content.contains("合规") || content.contains("compliance"),
        "i18n rules must define compliance baseline"
    );
}

#[test]
fn i18n_rules_define_user_rights() {
    let content = read_i18n_rules();
    assert!(
        content.contains("删除") || content.contains("导出") || content.contains("纠正"),
        "i18n rules must define user data rights (view, export, correct, delete)"
    );
}

#[test]
fn i18n_rules_require_independent_language_fields() {
    let content = read_i18n_rules();
    assert!(
        content.contains("独立字段"),
        "i18n rules must require independent language/region/timezone fields"
    );
}

#[test]
fn i18n_rules_require_human_review_for_high_risk() {
    let content = read_i18n_rules();
    assert!(
        content.contains("人工审核"),
        "i18n rules must require human review for high-risk safety scenarios"
    );
}

const APP_I18N_SRC: &str = "../../apps/app/src/i18n.rs";
const MODEL_GW_LOCALE_SRC: &str = "../../services/model-gateway/src/locale.rs";

fn read_app_i18n() -> String {
    fs::read_to_string(APP_I18N_SRC).unwrap_or_else(|e| panic!("cannot read app i18n: {e}"))
}

fn read_model_gw_locale() -> String {
    fs::read_to_string(MODEL_GW_LOCALE_SRC)
        .unwrap_or_else(|e| panic!("cannot read model-gw locale: {e}"))
}

#[test]
fn app_i18n_and_model_gateway_locale_share_core_keys() {
    let app_src = read_app_i18n();
    let gw_src = read_model_gw_locale();
    let core_keys = [
        "safety.block.applied",
        "safety.report.confirmation",
        "safety.reject.harmful",
        "privacy.data_export.title",
        "privacy.data_delete.confirmation",
        "privacy.data_correction.title",
        "compliance.underage.warning",
        "compliance.crossborder.notice",
        "compliance.region.policy",
        "privacy.consent.required",
    ];
    for key in &core_keys {
        assert!(
            app_src.contains(&format!("\"{}\"", key)),
            "App I18nRegistry missing key: {key}"
        );
        assert!(
            gw_src.contains(&format!("\"{}\"", key)),
            "Model-Gateway TerminologyRegistry missing key: {key}"
        );
    }
}

#[test]
fn no_legacy_key_safety_block_message_remains() {
    let gw_src = read_model_gw_locale();
    assert!(
        !gw_src.contains("\"safety.block.message\""),
        "legacy key safety.block.message must be replaced with safety.block.applied"
    );
}
