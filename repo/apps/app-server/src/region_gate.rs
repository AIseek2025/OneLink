//! Region gate policy — two-level compliance boundary.
//!
//! # Policy architecture
//!
//! The region gate implements a **two-level** access policy:
//!
//! 1. **Gate level** (`check_region_gate`): determines whether a user from a
//!    given region may access the application at all. For restricted regions
//!    (currently EU), the gate returns `allowed = true` with `degradation =
//!    ReadOnly`, meaning the user can reach the app shell (view settings,
//!    view summaries, view compliance summary) but is placed in a degraded
//!    read-only mode.
//!
//! 2. **Handler level** (in `router.rs`): each data-rights action handler
//!    (export, delete, correction) independently checks `degradation ==
//!    ReadOnly || Blocked`. When the degradation is restrictive, the handler
//!    returns `403 safety_blocked` with a localized message keyed on
//!    `compliance.region.policy`.
//!
//! This means: **EU users see the app shell (gate `allowed=true`, ReadOnly),
//! but data-rights write operations (export/delete/correction) are rejected
//! with `403 safety_blocked` at the handler level.** The compliance-summary
//! endpoint also reflects this by setting `data_export_available = false`,
//! `data_delete_available = false`, and `data_correction_available = false`
//! for restricted regions.
//!
//! This two-level design separates "can the user see the app" from "can the
//! user perform data-rights write operations", which is required by GDPR
//! data-residency constraints.

use crate::i18n::I18nRegistry;
use serde::{Deserialize, Serialize};

const RESTRICTED_REGIONS: &[&str] = &["EU"];
const DEFAULT_REGION: &str = "CN";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DataZone {
    Domestic,
    CrossBorder,
    Restricted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RegionDegradationMode {
    Normal,
    FallbackToDefault,
    ReadOnly,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionGateResult {
    pub user_region: String,
    pub data_zone: DataZone,
    pub degradation: RegionDegradationMode,
    pub allowed: bool,
    pub reason: Option<String>,
    pub fallback_region: Option<String>,
    pub notice_key: Option<String>,
}

pub fn check_region_gate(user_region: &str) -> RegionGateResult {
    let region = if user_region.is_empty() {
        DEFAULT_REGION
    } else {
        user_region
    };

    if RESTRICTED_REGIONS.contains(&region) {
        RegionGateResult {
            user_region: region.to_string(),
            data_zone: DataZone::Restricted,
            degradation: RegionDegradationMode::ReadOnly,
            allowed: true,
            reason: Some("restricted_region_read_only".to_string()),
            fallback_region: None,
            notice_key: Some("compliance.region.policy".to_string()),
        }
    } else if region == DEFAULT_REGION {
        RegionGateResult {
            user_region: region.to_string(),
            data_zone: DataZone::Domestic,
            degradation: RegionDegradationMode::Normal,
            allowed: true,
            reason: None,
            fallback_region: None,
            notice_key: None,
        }
    } else {
        RegionGateResult {
            user_region: region.to_string(),
            data_zone: DataZone::CrossBorder,
            degradation: RegionDegradationMode::Normal,
            allowed: true,
            reason: Some("crossborder_data_notice".to_string()),
            fallback_region: Some(DEFAULT_REGION.to_string()),
            notice_key: Some("compliance.crossborder.notice".to_string()),
        }
    }
}

pub fn localize_gate_notice(gate: &RegionGateResult, locale: &str) -> String {
    let reg = I18nRegistry::new();
    match &gate.notice_key {
        Some(key) => reg.translate(key, locale),
        None => String::new(),
    }
}

use crate::dto::{RegionPolicyDetail, RegionPolicySection, RegionResidencyPolicy};

pub fn region_residency_policy_document() -> RegionResidencyPolicy {
    RegionResidencyPolicy {
        version: "1.0.0".to_string(),
        updated_at: "2026-05-25".to_string(),
        user_region_determination: RegionPolicySection {
            summary_en: "User region is determined by registration country and can be updated in settings. Default: CN (domestic).".to_string(),
            summary_zh: "用户区域由注册国家决定，可在设置中更新。默认：CN（国内）。".to_string(),
            details: vec![
                RegionPolicyDetail {
                    region: "CN".to_string(),
                    policy_en: "Domestic zone. User data stored in mainland infrastructure.".to_string(),
                    policy_zh: "国内区域。用户数据存储在大陆基础设施。".to_string(),
                },
                RegionPolicyDetail {
                    region: "US".to_string(),
                    policy_en: "Cross-border zone. User data may transfer between CN and US with explicit notice.".to_string(),
                    policy_zh: "跨境区域。用户数据可能在 CN 与 US 间传输，附带明确通知。".to_string(),
                },
                RegionPolicyDetail {
                    region: "EU".to_string(),
                    policy_en: "Restricted zone. User data stays in assigned region; write operations blocked per GDPR residency.".to_string(),
                    policy_zh: "受限区域。用户数据驻留在指定区域；写入操作按 GDPR 驻留要求阻断。".to_string(),
                },
                RegionPolicyDetail {
                    region: "SEA".to_string(),
                    policy_en: "Cross-border zone. User data may transfer between CN and SEA with explicit notice.".to_string(),
                    policy_zh: "跨境区域。用户数据可能在 CN 与 SEA 间传输，附带明确通知。".to_string(),
                },
            ],
        },
        data_residency: RegionPolicySection {
            summary_en: "Data residency enforced per region. Domestic data stays local; restricted data never leaves the assigned region.".to_string(),
            summary_zh: "数据驻留按区域执行。国内数据留在本地；受限数据不离开指定区域。".to_string(),
            details: vec![
                RegionPolicyDetail {
                    region: "CN".to_string(),
                    policy_en: "All user data (profile, memory, settings, consent) stored in mainland data centers.".to_string(),
                    policy_zh: "所有用户数据（画像、记忆、设置、同意）存储在大陆数据中心。".to_string(),
                },
                RegionPolicyDetail {
                    region: "US".to_string(),
                    policy_en: "Profile and settings replicated across CN and US; memory data stays in origin region.".to_string(),
                    policy_zh: "画像和设置在 CN 和 US 间复制；记忆数据留在原区域。".to_string(),
                },
                RegionPolicyDetail {
                    region: "EU".to_string(),
                    policy_en: "All data stored strictly within EU data centers. No cross-border replication. Read-only degradation applied.".to_string(),
                    policy_zh: "所有数据严格存储在 EU 数据中心内。不做跨境复制。只读降级适用。".to_string(),
                },
                RegionPolicyDetail {
                    region: "SEA".to_string(),
                    policy_en: "Profile and settings replicated across CN and SEA; memory data stays in origin region.".to_string(),
                    policy_zh: "画像和设置在 CN 和 SEA 间复制；记忆数据留在原区域。".to_string(),
                },
            ],
        },
        model_call_region: RegionPolicySection {
            summary_en: "AI model calls routed to the nearest compliant region. EU calls processed within EU.".to_string(),
            summary_zh: "AI 模型调用路由至最近的合规区域。EU 调用在 EU 内处理。".to_string(),
            details: vec![
                RegionPolicyDetail {
                    region: "CN".to_string(),
                    policy_en: "Model calls processed in domestic inference cluster.".to_string(),
                    policy_zh: "模型调用在国产推理集群中处理。".to_string(),
                },
                RegionPolicyDetail {
                    region: "US".to_string(),
                    policy_en: "Model calls processed in US inference cluster; fallback to CN if unavailable.".to_string(),
                    policy_zh: "模型调用在美国推理集群处理；如不可用则回退至 CN。".to_string(),
                },
                RegionPolicyDetail {
                    region: "EU".to_string(),
                    policy_en: "Model calls processed in EU inference cluster. No cross-border model routing.".to_string(),
                    policy_zh: "模型调用在 EU 推理集群中处理。不做跨境模型路由。".to_string(),
                },
                RegionPolicyDetail {
                    region: "SEA".to_string(),
                    policy_en: "Model calls processed in SEA inference cluster; fallback to CN if unavailable.".to_string(),
                    policy_zh: "模型调用在 SEA 推理集群处理；如不可用则回退至 CN。".to_string(),
                },
            ],
        },
        log_storage_region: RegionPolicySection {
            summary_en: "Operational logs stored in the user's assigned region. EU logs never exported.".to_string(),
            summary_zh: "运营日志存储在用户指定区域。EU 日志不做导出。".to_string(),
            details: vec![
                RegionPolicyDetail {
                    region: "CN".to_string(),
                    policy_en: "All operational logs stored in mainland logging infrastructure.".to_string(),
                    policy_zh: "所有运营日志存储在大陆日志基础设施。".to_string(),
                },
                RegionPolicyDetail {
                    region: "US".to_string(),
                    policy_en: "Logs stored in US logging infrastructure; audit logs may replicate to CN for compliance review.".to_string(),
                    policy_zh: "日志存储在 US 日志基础设施；审计日志可复制至 CN 用于合规审查。".to_string(),
                },
                RegionPolicyDetail {
                    region: "EU".to_string(),
                    policy_en: "All logs stored strictly within EU. No log export outside EU boundary.".to_string(),
                    policy_zh: "所有日志严格存储在 EU 内。不做 EU 边界外的日志导出。".to_string(),
                },
                RegionPolicyDetail {
                    region: "SEA".to_string(),
                    policy_en: "Logs stored in SEA logging infrastructure; audit logs may replicate to CN for compliance review.".to_string(),
                    policy_zh: "日志存储在 SEA 日志基础设施；审计日志可复制至 CN 用于合规审查。".to_string(),
                },
            ],
        },
        cross_border_basis: RegionPolicySection {
            summary_en: "Cross-border data transfers require explicit user consent and comply with applicable regulations.".to_string(),
            summary_zh: "跨境数据传输需要用户明确同意，并遵守适用法规。".to_string(),
            details: vec![
                RegionPolicyDetail {
                    region: "CN".to_string(),
                    policy_en: "No cross-border transfer for domestic users. All data stays within mainland boundary.".to_string(),
                    policy_zh: "国内用户不做跨境传输。所有数据留在大陆边界内。".to_string(),
                },
                RegionPolicyDetail {
                    region: "US".to_string(),
                    policy_en: "Cross-border transfer based on user consent via crossborder_data_notice. PII encrypted in transit.".to_string(),
                    policy_zh: "跨境传输基于用户同意（crossborder_data_notice）。PII 在传输中加密。".to_string(),
                },
                RegionPolicyDetail {
                    region: "EU".to_string(),
                    policy_en: "No cross-border transfer. GDPR Article 44-49 compliance enforced. Read-only degradation ensures data stays in EU.".to_string(),
                    policy_zh: "不做跨境传输。执行 GDPR 第 44-49 条合规。只读降级确保数据留在 EU。".to_string(),
                },
                RegionPolicyDetail {
                    region: "SEA".to_string(),
                    policy_en: "Cross-border transfer based on user consent via crossborder_data_notice. PII encrypted in transit.".to_string(),
                    policy_zh: "跨境传输基于用户同意（crossborder_data_notice）。PII 在传输中加密。".to_string(),
                },
            ],
        },
        region_failure_degradation: RegionPolicySection {
            summary_en: "If a region's infrastructure fails, users are degraded to read-only with localized notice. No data migration across regions.".to_string(),
            summary_zh: "如果某区域基础设施故障，用户降级为只读并附带本地化通知。不做跨区域数据迁移。".to_string(),
            details: vec![
                RegionPolicyDetail {
                    region: "CN".to_string(),
                    policy_en: "Service unavailable: users see maintenance notice. No degradation to other regions.".to_string(),
                    policy_zh: "服务不可用：用户看到维护通知。不降级到其他区域。".to_string(),
                },
                RegionPolicyDetail {
                    region: "US".to_string(),
                    policy_en: "US cluster failure: fallback to CN for read-only profile view. Write operations queued until US recovery.".to_string(),
                    policy_zh: "US 集群故障：回退至 CN 进行只读画像查看。写入操作排队直至 US 恢复。".to_string(),
                },
                RegionPolicyDetail {
                    region: "EU".to_string(),
                    policy_en: "EU cluster failure: strictly read-only mode within EU. No fallback to other regions per GDPR residency.".to_string(),
                    policy_zh: "EU 集群故障：在 EU 内严格只读模式。按 GDPR 驻留不回退至其他区域。".to_string(),
                },
                RegionPolicyDetail {
                    region: "SEA".to_string(),
                    policy_en: "SEA cluster failure: fallback to CN for read-only profile view. Write operations queued until SEA recovery.".to_string(),
                    policy_zh: "SEA 集群故障：回退至 CN 进行只读画像查看。写入操作排队直至 SEA 恢复。".to_string(),
                },
            ],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domestic_region() {
        let gate = check_region_gate("CN");
        assert_eq!(gate.data_zone, DataZone::Domestic);
        assert_eq!(gate.degradation, RegionDegradationMode::Normal);
        assert!(gate.allowed);
        assert!(gate.reason.is_none());
        println!(
            "[region-gate] CN => zone={:?}, allowed={}",
            gate.data_zone, gate.allowed
        );
    }

    #[test]
    fn test_crossborder_region() {
        let gate = check_region_gate("US");
        assert_eq!(gate.data_zone, DataZone::CrossBorder);
        assert!(gate.allowed);
        assert!(gate.fallback_region.is_some());
        assert_eq!(
            gate.notice_key.as_deref(),
            Some("compliance.crossborder.notice")
        );
        println!(
            "[region-gate] US => zone={:?}, notice={:?}",
            gate.data_zone, gate.notice_key
        );
    }

    #[test]
    fn test_restricted_region() {
        let gate = check_region_gate("EU");
        assert_eq!(gate.data_zone, DataZone::Restricted);
        assert_eq!(gate.degradation, RegionDegradationMode::ReadOnly);
        assert!(gate.allowed);
        assert_eq!(gate.reason.as_deref(), Some("restricted_region_read_only"));
        println!(
            "[region-gate] EU => zone={:?}, degradation={:?}",
            gate.data_zone, gate.degradation
        );
    }

    #[test]
    fn test_empty_region_defaults() {
        let gate = check_region_gate("");
        assert_eq!(gate.user_region, "CN");
        assert_eq!(gate.data_zone, DataZone::Domestic);
        println!("[region-gate] '' => default=CN, zone={:?}", gate.data_zone);
    }

    #[test]
    fn test_localize_gate_notice_crossborder() {
        let gate = check_region_gate("US");
        let notice_zh = localize_gate_notice(&gate, "zh-CN");
        assert!(notice_zh.contains("传输"));
        let notice_en = localize_gate_notice(&gate, "en");
        assert!(notice_en.contains("transferred"));
        println!("[region-gate-notice] US zh={}, en={}", notice_zh, notice_en);
    }

    #[test]
    fn test_localize_gate_notice_restricted() {
        let gate = check_region_gate("EU");
        let notice_zh = localize_gate_notice(&gate, "zh-CN");
        assert!(notice_zh.contains("驻留"));
        let notice_en = localize_gate_notice(&gate, "en");
        assert!(notice_en.contains("residency"));
        println!("[region-gate-notice] EU zh={}, en={}", notice_zh, notice_en);
    }
}
