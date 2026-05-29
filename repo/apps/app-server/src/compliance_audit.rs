use crate::i18n::I18nRegistry;
use crate::region_gate::{check_region_gate, RegionDegradationMode};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceAuditAction {
    Export,
    Delete,
    Correction,
    View,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceAuditOutcome {
    Allowed,
    BlockedReadOnly,
    BlockedGate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceAuditEntry {
    pub timestamp_ms: u64,
    pub user_region: String,
    pub data_zone: String,
    pub degradation: String,
    pub action: ComplianceAuditAction,
    pub outcome: ComplianceAuditOutcome,
    pub i18n_notice_key: Option<String>,
    pub localized_message_en: Option<String>,
    pub localized_message_zh: Option<String>,
}

fn degradation_to_string(d: &RegionDegradationMode) -> &'static str {
    match d {
        RegionDegradationMode::Normal => "normal",
        RegionDegradationMode::FallbackToDefault => "fallback_to_default",
        RegionDegradationMode::ReadOnly => "read_only",
        RegionDegradationMode::Blocked => "blocked",
    }
}

fn data_zone_to_string(z: &crate::region_gate::DataZone) -> &'static str {
    match z {
        crate::region_gate::DataZone::Domestic => "domestic",
        crate::region_gate::DataZone::CrossBorder => "cross_border",
        crate::region_gate::DataZone::Restricted => "restricted",
    }
}

pub fn audit_compliance_action(
    user_region: &str,
    action: ComplianceAuditAction,
    _locale: &str,
) -> ComplianceAuditEntry {
    let gate = check_region_gate(user_region);
    let reg = I18nRegistry::new();

    let restricted = gate.degradation == RegionDegradationMode::ReadOnly
        || gate.degradation == RegionDegradationMode::Blocked;

    let is_write_action = action != ComplianceAuditAction::View;
    let outcome = if restricted && is_write_action {
        ComplianceAuditOutcome::BlockedReadOnly
    } else if !gate.allowed {
        ComplianceAuditOutcome::BlockedGate
    } else {
        ComplianceAuditOutcome::Allowed
    };

    let notice_key = gate.notice_key.clone();
    let localized_en = notice_key.as_ref().map(|k| reg.translate(k, "en"));
    let localized_zh = notice_key.as_ref().map(|k| reg.translate(k, "zh-CN"));

    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    ComplianceAuditEntry {
        timestamp_ms,
        user_region: user_region.to_string(),
        data_zone: data_zone_to_string(&gate.data_zone).to_string(),
        degradation: degradation_to_string(&gate.degradation).to_string(),
        action,
        outcome,
        i18n_notice_key: notice_key,
        localized_message_en: localized_en,
        localized_message_zh: localized_zh,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_cn_export_allowed() {
        let entry = audit_compliance_action("CN", ComplianceAuditAction::Export, "zh-CN");
        assert_eq!(entry.outcome, ComplianceAuditOutcome::Allowed);
        assert_eq!(entry.data_zone, "domestic");
        assert_eq!(entry.degradation, "normal");
        assert!(entry.i18n_notice_key.is_none());
        println!("[compliance-audit] CN export => {:?}", entry.outcome);
    }

    #[test]
    fn test_audit_eu_export_blocked() {
        let entry = audit_compliance_action("EU", ComplianceAuditAction::Export, "en");
        assert_eq!(entry.outcome, ComplianceAuditOutcome::BlockedReadOnly);
        assert_eq!(entry.data_zone, "restricted");
        assert_eq!(entry.degradation, "read_only");
        assert_eq!(
            entry.i18n_notice_key.as_deref(),
            Some("compliance.region.policy")
        );
        assert!(entry.localized_message_en.is_some());
        assert!(entry
            .localized_message_en
            .as_ref()
            .unwrap()
            .contains("residency"));
        println!("[compliance-audit] EU export => {:?}", entry.outcome);
    }

    #[test]
    fn test_audit_eu_delete_blocked() {
        let entry = audit_compliance_action("EU", ComplianceAuditAction::Delete, "zh-CN");
        assert_eq!(entry.outcome, ComplianceAuditOutcome::BlockedReadOnly);
        assert!(entry.localized_message_zh.is_some());
        assert!(entry
            .localized_message_zh
            .as_ref()
            .unwrap()
            .contains("驻留"));
        println!("[compliance-audit] EU delete => {:?}", entry.outcome);
    }

    #[test]
    fn test_audit_eu_correction_blocked() {
        let entry = audit_compliance_action("EU", ComplianceAuditAction::Correction, "en");
        assert_eq!(entry.outcome, ComplianceAuditOutcome::BlockedReadOnly);
        println!("[compliance-audit] EU correction => {:?}", entry.outcome);
    }

    #[test]
    fn test_audit_us_crossborder_notice() {
        let entry = audit_compliance_action("US", ComplianceAuditAction::Export, "en");
        assert_eq!(entry.outcome, ComplianceAuditOutcome::Allowed);
        assert_eq!(entry.data_zone, "cross_border");
        assert_eq!(
            entry.i18n_notice_key.as_deref(),
            Some("compliance.crossborder.notice")
        );
        assert!(entry
            .localized_message_en
            .as_ref()
            .unwrap()
            .contains("transferred"));
        println!(
            "[compliance-audit] US export => {:?} notice={:?}",
            entry.outcome, entry.i18n_notice_key
        );
    }

    #[test]
    fn test_audit_cn_all_actions_allowed() {
        for action in &[
            ComplianceAuditAction::Export,
            ComplianceAuditAction::Delete,
            ComplianceAuditAction::Correction,
            ComplianceAuditAction::View,
        ] {
            let entry = audit_compliance_action("CN", action.clone(), "zh-CN");
            assert_eq!(entry.outcome, ComplianceAuditOutcome::Allowed);
        }
        println!("[compliance-audit] CN all actions => Allowed");
    }

    #[test]
    fn test_audit_eu_all_write_actions_blocked() {
        for action in &[
            ComplianceAuditAction::Export,
            ComplianceAuditAction::Delete,
            ComplianceAuditAction::Correction,
        ] {
            let entry = audit_compliance_action("EU", action.clone(), "en");
            assert_eq!(entry.outcome, ComplianceAuditOutcome::BlockedReadOnly);
        }
        println!("[compliance-audit] EU write actions => BlockedReadOnly");
    }

    #[test]
    fn test_audit_entry_serializable() {
        let entry = audit_compliance_action("EU", ComplianceAuditAction::Export, "en");
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("blocked_read_only"));
        assert!(json.contains("restricted"));
        let parsed: ComplianceAuditEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.outcome, ComplianceAuditOutcome::BlockedReadOnly);
        println!("[compliance-audit] serializable round-trip ok");
    }

    #[test]
    fn test_audit_evidence_all_regions_all_actions() {
        let regions = crate::i18n::supported_regions();
        let actions = [
            ComplianceAuditAction::Export,
            ComplianceAuditAction::Delete,
            ComplianceAuditAction::Correction,
            ComplianceAuditAction::View,
        ];
        let mut results = Vec::new();
        for region in &regions {
            for action in &actions {
                let entry = audit_compliance_action(region, action.clone(), "en");
                results.push(serde_json::to_value(&entry).unwrap());
            }
        }
        println!(
            "[evidence:compliance_audit_log] {} entries, sample: {}",
            results.len(),
            serde_json::to_string_pretty(&results[0]).unwrap()
        );
        assert_eq!(results.len(), regions.len() * actions.len());
        let eu_blocked: Vec<_> = results
            .iter()
            .filter(|r| {
                r["user_region"].as_str() == Some("EU")
                    && r["outcome"].as_str() == Some("blocked_read_only")
            })
            .collect();
        assert_eq!(
            eu_blocked.len(),
            3,
            "EU should block 3 write actions (export/delete/correction)"
        );
    }
}
