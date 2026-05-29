use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BffContractEndpoint {
    pub method: String,
    pub path: String,
    pub request_dto: String,
    pub response_dto: String,
    pub app_route: String,
    pub phase: String,
    pub analytics_events: Vec<AnalyticsEventMapping>,
    pub tracking_fields_frozen: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalyticsEventMapping {
    pub event_name: String,
    pub trigger: String,
    pub supplemental_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContractFreezeManifest {
    pub version: String,
    pub frozen_at: String,
    pub endpoints: Vec<BffContractEndpoint>,
}

pub fn frozen_bff_contract_manifest() -> ContractFreezeManifest {
    ContractFreezeManifest {
        version: "1.0.0".into(),
        frozen_at: "2026-05-24T00:00:00Z".into(),
        endpoints: vec![
            BffContractEndpoint {
                method: "POST".into(),
                path: "/api/v1/bff/auth/login".into(),
                request_dto: "AuthRequest".into(),
                response_dto: "AuthResponse".into(),
                app_route: "/api/v1/bff/auth/login".into(),
                phase: "A1".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
            BffContractEndpoint {
                method: "POST".into(),
                path: "/api/v1/bff/auth/register".into(),
                request_dto: "RegisterRequest".into(),
                response_dto: "RegisterResponse".into(),
                app_route: "/api/v1/bff/auth/register".into(),
                phase: "A1".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
            BffContractEndpoint {
                method: "POST".into(),
                path: "/api/v1/bff/auth/session/refresh".into(),
                request_dto: "SessionRefreshRequest".into(),
                response_dto: "SessionRefreshResponse".into(),
                app_route: "/api/v1/bff/auth/session/refresh".into(),
                phase: "A1".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
            BffContractEndpoint {
                method: "GET".into(),
                path: "/api/v1/bff/home".into(),
                request_dto: "()".into(),
                response_dto: "MeSummaryResponse".into(),
                app_route: "/api/v1/bff/me/summary".into(),
                phase: "A1".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
            BffContractEndpoint {
                method: "GET".into(),
                path: "/api/v1/bff/chat/init".into(),
                request_dto: "()".into(),
                response_dto: "ConversationListResponse".into(),
                app_route: "/api/v1/bff/conversations".into(),
                phase: "A1".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
            BffContractEndpoint {
                method: "POST".into(),
                path: "/api/v1/bff/chat/messages".into(),
                request_dto: "ChatRequest".into(),
                response_dto: "ChatResponse".into(),
                app_route: "/api/v1/bff/conversations/{id}/messages".into(),
                phase: "A1".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
            BffContractEndpoint {
                method: "GET".into(),
                path: "/api/v1/bff/profile/confirmations".into(),
                request_dto: "()".into(),
                response_dto: "ProfileConfirmationListResponse".into(),
                app_route: "/api/v1/bff/profile/confirmations".into(),
                phase: "A1".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
            BffContractEndpoint {
                method: "POST".into(),
                path: "/api/v1/bff/profile/confirmations/{id}/actions".into(),
                request_dto: "ProfileConfirmationActionRequest".into(),
                response_dto: "ProfileConfirmationActionResponse".into(),
                app_route: "/api/v1/bff/profile/confirmations/{id}/actions".into(),
                phase: "A1".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
            BffContractEndpoint {
                method: "GET".into(),
                path: "/api/v1/bff/profile/me".into(),
                request_dto: "()".into(),
                response_dto: "ProfileDto".into(),
                app_route: "/api/v1/bff/profile/me".into(),
                phase: "A1".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
            BffContractEndpoint {
                method: "GET".into(),
                path: "/api/v1/bff/settings/summary".into(),
                request_dto: "()".into(),
                response_dto: "SettingsSummaryResponse".into(),
                app_route: "/api/v1/bff/settings/summary".into(),
                phase: "A1".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
            BffContractEndpoint {
                method: "PATCH".into(),
                path: "/api/v1/bff/settings/locale".into(),
                request_dto: "SettingsUpdateRequest".into(),
                response_dto: "Value".into(),
                app_route: "/api/v1/bff/settings/locale/update".into(),
                phase: "A1".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
            BffContractEndpoint {
                method: "POST".into(),
                path: "/api/v1/bff/analytics/events".into(),
                request_dto: "AnalyticsBatchRequest".into(),
                response_dto: "Value".into(),
                app_route: "/api/v1/bff/analytics/events".into(),
                phase: "A1".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
            BffContractEndpoint {
                method: "POST".into(),
                path: "/api/v1/bff/find/requests".into(),
                request_dto: "FindRequestCreateRequest".into(),
                response_dto: "FindRequestResponse".into(),
                app_route: "/api/v1/bff/find/requests".into(),
                phase: "A2".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
            BffContractEndpoint {
                method: "GET".into(),
                path: "/api/v1/bff/find/requests/{id}".into(),
                request_dto: "()".into(),
                response_dto: "FindRequestDetailResponse".into(),
                app_route: "/api/v1/bff/find/requests/{id}".into(),
                phase: "A2".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
            BffContractEndpoint {
                method: "POST".into(),
                path: "/api/v1/bff/find/requests/{id}/clarifications".into(),
                request_dto: "ClarificationAnswerRequest".into(),
                response_dto: "ClarificationAnswerResponse".into(),
                app_route: "/api/v1/bff/find/requests/{id}/clarifications".into(),
                phase: "A2".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
            BffContractEndpoint {
                method: "GET".into(),
                path: "/api/v1/bff/recommendations".into(),
                request_dto: "()".into(),
                response_dto: "RecommendationListResponse".into(),
                app_route: "/api/v1/bff/recommendations".into(),
                phase: "A2".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
            BffContractEndpoint {
                method: "GET".into(),
                path: "/api/v1/bff/recommendations/{id}".into(),
                request_dto: "()".into(),
                response_dto: "RecommendationDetailResponse".into(),
                app_route: "/api/v1/bff/recommendations/{id}".into(),
                phase: "A2".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
            BffContractEndpoint {
                method: "POST".into(),
                path: "/api/v1/bff/recommendations/{id}/feedback".into(),
                request_dto: "RecommendationFeedbackRequest".into(),
                response_dto: "RecommendationFeedbackResponse".into(),
                app_route: "/api/v1/bff/recommendations/{id}/feedback".into(),
                phase: "A2".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
            BffContractEndpoint {
                method: "POST".into(),
                path: "/api/v1/bff/dm/threads/draft".into(),
                request_dto: "DmDraftRequest".into(),
                response_dto: "DmDraftResponse".into(),
                app_route: "/api/v1/bff/dm/threads/draft".into(),
                phase: "A3".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
            BffContractEndpoint {
                method: "POST".into(),
                path: "/api/v1/bff/dm/threads/first-message".into(),
                request_dto: "DmFirstMessageSubmitRequest".into(),
                response_dto: "DmFirstMessageSubmitResponse".into(),
                app_route: "/api/v1/bff/dm/threads/first-message".into(),
                phase: "A3".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
            BffContractEndpoint {
                method: "GET".into(),
                path: "/api/v1/bff/dm/threads/{id}".into(),
                request_dto: "()".into(),
                response_dto: "DmThreadDetailResponse".into(),
                app_route: "/api/v1/bff/dm/threads/{id}".into(),
                phase: "A3".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
            BffContractEndpoint {
                method: "POST".into(),
                path: "/api/v1/bff/safety/reports".into(),
                request_dto: "ReportSubmitRequest".into(),
                response_dto: "ReportSubmitResponse".into(),
                app_route: "/api/v1/bff/safety/reports".into(),
                phase: "A3".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
            BffContractEndpoint {
                method: "POST".into(),
                path: "/api/v1/bff/safety/blocks".into(),
                request_dto: "BlockApplyRequest".into(),
                response_dto: "BlockApplyResponse".into(),
                app_route: "/api/v1/bff/safety/blocks".into(),
                phase: "A3".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
            BffContractEndpoint {
                method: "GET".into(),
                path: "/api/v1/bff/safety/appeals/{id}".into(),
                request_dto: "()".into(),
                response_dto: "AppealStatusResponse".into(),
                app_route: "/api/v1/bff/safety/appeals/{id}".into(),
                phase: "A3".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
            BffContractEndpoint {
                method: "GET".into(),
                path: "/api/v1/bff/settings/locale".into(),
                request_dto: "()".into(),
                response_dto: "LocaleRegistryResponse".into(),
                app_route: "/api/v1/bff/settings/locale".into(),
                phase: "A4".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
            BffContractEndpoint {
                method: "GET".into(),
                path: "/api/v1/bff/compliance/summary".into(),
                request_dto: "()".into(),
                response_dto: "ComplianceSummaryResponse".into(),
                app_route: "/api/v1/bff/compliance/summary".into(),
                phase: "A4".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
            BffContractEndpoint {
                method: "POST".into(),
                path: "/api/v1/bff/compliance/export".into(),
                request_dto: "ComplianceActionRequest".into(),
                response_dto: "ComplianceActionResponse".into(),
                app_route: "/api/v1/bff/compliance/export".into(),
                phase: "A4".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
            BffContractEndpoint {
                method: "POST".into(),
                path: "/api/v1/bff/compliance/delete".into(),
                request_dto: "ComplianceActionRequest".into(),
                response_dto: "ComplianceActionResponse".into(),
                app_route: "/api/v1/bff/compliance/delete".into(),
                phase: "A4".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
            BffContractEndpoint {
                method: "POST".into(),
                path: "/api/v1/bff/compliance/correction".into(),
                request_dto: "ComplianceActionRequest".into(),
                response_dto: "ComplianceActionResponse".into(),
                app_route: "/api/v1/bff/compliance/correction".into(),
                phase: "A4".into(),
                analytics_events: vec![],
                tracking_fields_frozen: true,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_manifest_version() {
        let manifest = frozen_bff_contract_manifest();
        assert_eq!(manifest.version, "1.0.0");
    }

    #[test]
    fn test_contract_manifest_has_a1_endpoints() {
        let manifest = frozen_bff_contract_manifest();
        let a1_count = manifest
            .endpoints
            .iter()
            .filter(|e| e.phase == "A1")
            .count();
        assert!(
            a1_count >= 10,
            "A1 should have at least 10 endpoints, got {}",
            a1_count
        );
    }

    #[test]
    fn test_contract_manifest_has_a2_endpoints() {
        let manifest = frozen_bff_contract_manifest();
        let a2_count = manifest
            .endpoints
            .iter()
            .filter(|e| e.phase == "A2")
            .count();
        assert!(
            a2_count >= 5,
            "A2 should have at least 5 endpoints, got {}",
            a2_count
        );
    }

    #[test]
    fn test_contract_manifest_has_a3_endpoints() {
        let manifest = frozen_bff_contract_manifest();
        let a3_count = manifest
            .endpoints
            .iter()
            .filter(|e| e.phase == "A3")
            .count();
        assert!(
            a3_count >= 4,
            "A3 should have at least 4 endpoints, got {}",
            a3_count
        );
    }

    #[test]
    fn test_contract_manifest_has_a4_endpoints() {
        let manifest = frozen_bff_contract_manifest();
        let a4_count = manifest
            .endpoints
            .iter()
            .filter(|e| e.phase == "A4")
            .count();
        assert!(
            a4_count >= 3,
            "A4 should have at least 3 endpoints, got {}",
            a4_count
        );
    }

    #[test]
    fn test_contract_paths_unique() {
        let manifest = frozen_bff_contract_manifest();
        let keys: Vec<String> = manifest
            .endpoints
            .iter()
            .map(|e| format!("{} {}", e.method, e.path))
            .collect();
        for i in 0..keys.len() {
            for j in (i + 1)..keys.len() {
                assert_ne!(keys[i], keys[j], "duplicate contract: {}", keys[i]);
            }
        }
    }

    #[test]
    fn test_contract_app_routes_unique() {
        let manifest = frozen_bff_contract_manifest();
        let routes: Vec<&str> = manifest
            .endpoints
            .iter()
            .map(|e| e.app_route.as_str())
            .collect();
        for i in 0..routes.len() {
            for j in (i + 1)..routes.len() {
                assert_ne!(routes[i], routes[j], "duplicate app route: {}", routes[i]);
            }
        }
    }

    #[test]
    fn test_manifest_serializable() {
        let manifest = frozen_bff_contract_manifest();
        let json = serde_json::to_string(&manifest).unwrap();
        assert!(!json.is_empty());
        let parsed: ContractFreezeManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, manifest);
    }

    #[test]
    fn test_contract_methods_valid() {
        let manifest = frozen_bff_contract_manifest();
        for ep in &manifest.endpoints {
            assert!(
                matches!(
                    ep.method.as_str(),
                    "GET" | "POST" | "PUT" | "PATCH" | "DELETE"
                ),
                "invalid method: {}",
                ep.method
            );
        }
    }
}
