use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AnalyticsEventName {
    AppBootStarted,
    AppBootFinished,
    AppBootFailed,
    LoginSubmit,
    LoginSuccess,
    LoginFailed,
    RegisterSubmit,
    RegisterSuccess,
    RegisterFailed,
    ConversationListView,
    ConversationOpen,
    ConversationCreate,
    ChatView,
    ChatSend,
    ChatReplyReceived,
    ChatRetry,
    ProfileFactExposed,
    ProfileFactAccept,
    ProfileFactReject,
    ProfileFactSnooze,
    SettingsView,
    SettingsSectionOpen,
    FindRequestStarted,
    FindRequestSubmitted,
    FindRequestFailed,
    ClarificationView,
    ClarificationSubmit,
    RecommendationListView,
    RecommendationCardExposed,
    RecommendationDetailView,
    RecommendationExplanationView,
    RecommendationConnectStart,
    RecommendationFeedbackOpen,
    RecommendationFeedbackSubmit,
    DmFirstMessageSubmit,
    DmFirstMessageApproved,
    DmFirstMessageBlocked,
    ReportOpen,
    ReportSubmit,
    LocaleSettingView,
    LocaleSettingSave,
    DataRightsView,
    DataExportRequest,
    DataDeleteRequest,
    DataCorrectionRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsEventCatalogEntry {
    pub event_name: AnalyticsEventName,
    pub screen_id: String,
    pub description: String,
    pub payload_schema: Vec<String>,
    pub phase: String,
}

pub fn frozen_analytics_event_catalog() -> Vec<AnalyticsEventCatalogEntry> {
    vec![
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::AppBootStarted,
            screen_id: "splash".into(),
            description: "App boot process started".into(),
            payload_schema: vec!["boot_phase".into()],
            phase: "A0".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::AppBootFinished,
            screen_id: "splash".into(),
            description: "App boot completed successfully".into(),
            payload_schema: vec!["boot_phase".into(), "has_session".into()],
            phase: "A0".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::AppBootFailed,
            screen_id: "splash".into(),
            description: "App boot failed".into(),
            payload_schema: vec!["boot_phase".into(), "error_code".into()],
            phase: "A0".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::LoginSubmit,
            screen_id: "login".into(),
            description: "User submitted login form".into(),
            payload_schema: vec!["phone_hash".into()],
            phase: "A1".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::LoginSuccess,
            screen_id: "login".into(),
            description: "Login succeeded".into(),
            payload_schema: vec!["user_id".into(), "first_run".into()],
            phase: "A1".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::LoginFailed,
            screen_id: "login".into(),
            description: "Login failed".into(),
            payload_schema: vec!["error_code".into()],
            phase: "A1".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::RegisterSubmit,
            screen_id: "register".into(),
            description: "User submitted registration form".into(),
            payload_schema: vec!["phone_hash".into()],
            phase: "A1".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::RegisterSuccess,
            screen_id: "register".into(),
            description: "Registration succeeded".into(),
            payload_schema: vec!["user_id".into()],
            phase: "A1".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::RegisterFailed,
            screen_id: "register".into(),
            description: "Registration failed".into(),
            payload_schema: vec!["error_code".into()],
            phase: "A1".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::ConversationListView,
            screen_id: "conversations".into(),
            description: "Conversation list viewed".into(),
            payload_schema: vec!["count".into()],
            phase: "A1".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::ConversationOpen,
            screen_id: "chat".into(),
            description: "Opened an existing conversation".into(),
            payload_schema: vec!["conversation_id".into()],
            phase: "A1".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::ConversationCreate,
            screen_id: "conversations".into(),
            description: "Created a new conversation".into(),
            payload_schema: vec!["conversation_id".into()],
            phase: "A1".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::ChatView,
            screen_id: "chat".into(),
            description: "Chat screen viewed".into(),
            payload_schema: vec!["conversation_id".into()],
            phase: "A1".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::ChatSend,
            screen_id: "chat".into(),
            description: "User sent a chat message".into(),
            payload_schema: vec!["conversation_id".into(), "content_type".into()],
            phase: "A1".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::ChatReplyReceived,
            screen_id: "chat".into(),
            description: "Received a reply in chat".into(),
            payload_schema: vec!["conversation_id".into(), "reply_source".into()],
            phase: "A1".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::ChatRetry,
            screen_id: "chat".into(),
            description: "User retried a failed message".into(),
            payload_schema: vec!["conversation_id".into(), "original_message_id".into()],
            phase: "A1".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::ProfileFactExposed,
            screen_id: "profile_confirmation".into(),
            description: "A profile fact was shown to user".into(),
            payload_schema: vec!["fact_id".into(), "source".into(), "confidence".into()],
            phase: "A1".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::ProfileFactAccept,
            screen_id: "profile_confirmation".into(),
            description: "User accepted a profile fact".into(),
            payload_schema: vec!["fact_id".into()],
            phase: "A1".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::ProfileFactReject,
            screen_id: "profile_confirmation".into(),
            description: "User rejected a profile fact".into(),
            payload_schema: vec!["fact_id".into()],
            phase: "A1".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::ProfileFactSnooze,
            screen_id: "profile_confirmation".into(),
            description: "User snoozed a profile fact".into(),
            payload_schema: vec!["fact_id".into()],
            phase: "A1".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::SettingsView,
            screen_id: "settings".into(),
            description: "Settings screen viewed".into(),
            payload_schema: vec![],
            phase: "A1".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::SettingsSectionOpen,
            screen_id: "settings".into(),
            description: "User opened a settings section".into(),
            payload_schema: vec!["section_name".into()],
            phase: "A1".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::FindRequestStarted,
            screen_id: "find_input".into(),
            description: "User started a find request".into(),
            payload_schema: vec![],
            phase: "A2".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::FindRequestSubmitted,
            screen_id: "find_input".into(),
            description: "User submitted a find request".into(),
            payload_schema: vec!["request_id".into(), "intent_text_length".into()],
            phase: "A2".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::FindRequestFailed,
            screen_id: "find_input".into(),
            description: "Find request submission failed".into(),
            payload_schema: vec!["error_code".into()],
            phase: "A2".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::ClarificationView,
            screen_id: "find_input".into(),
            description: "Clarification questions shown".into(),
            payload_schema: vec!["request_id".into(), "question_count".into()],
            phase: "A2".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::ClarificationSubmit,
            screen_id: "find_input".into(),
            description: "User submitted clarification answers".into(),
            payload_schema: vec!["request_id".into(), "answer_count".into()],
            phase: "A2".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::RecommendationListView,
            screen_id: "recommendation_list".into(),
            description: "Recommendation list viewed".into(),
            payload_schema: vec!["request_id".into(), "result_count".into()],
            phase: "A2".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::RecommendationCardExposed,
            screen_id: "recommendation_list".into(),
            description: "A recommendation card was shown".into(),
            payload_schema: vec!["recommendation_id".into(), "match_score_bucket".into()],
            phase: "A2".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::RecommendationDetailView,
            screen_id: "recommendation_detail".into(),
            description: "Recommendation detail viewed".into(),
            payload_schema: vec!["recommendation_id".into()],
            phase: "A2".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::RecommendationExplanationView,
            screen_id: "recommendation_detail".into(),
            description: "User viewed recommendation explanation".into(),
            payload_schema: vec!["recommendation_id".into(), "explanation_version".into()],
            phase: "A2".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::RecommendationConnectStart,
            screen_id: "recommendation_detail".into(),
            description: "User initiated connection from recommendation".into(),
            payload_schema: vec!["recommendation_id".into()],
            phase: "A2".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::RecommendationFeedbackOpen,
            screen_id: "recommendation_detail".into(),
            description: "Feedback dialog opened".into(),
            payload_schema: vec!["recommendation_id".into()],
            phase: "A2".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::RecommendationFeedbackSubmit,
            screen_id: "recommendation_detail".into(),
            description: "User submitted recommendation feedback".into(),
            payload_schema: vec!["recommendation_id".into(), "feedback_type".into()],
            phase: "A2".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::DmFirstMessageSubmit,
            screen_id: "dm_first_message".into(),
            description: "User submitted first DM message".into(),
            payload_schema: vec!["recommendation_id".into(), "message_length".into()],
            phase: "A3".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::DmFirstMessageApproved,
            screen_id: "dm_first_message".into(),
            description: "First DM message approved by safety".into(),
            payload_schema: vec!["dm_thread_id".into()],
            phase: "A3".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::DmFirstMessageBlocked,
            screen_id: "dm_first_message".into(),
            description: "First DM message blocked by safety".into(),
            payload_schema: vec!["safety_decision".into()],
            phase: "A3".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::ReportOpen,
            screen_id: "report".into(),
            description: "Report dialog opened".into(),
            payload_schema: vec!["target_user_id".into()],
            phase: "A3".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::ReportSubmit,
            screen_id: "report".into(),
            description: "User submitted a report".into(),
            payload_schema: vec!["report_category".into()],
            phase: "A3".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::LocaleSettingView,
            screen_id: "locale_settings".into(),
            description: "Locale settings viewed".into(),
            payload_schema: vec!["current_locale".into(), "current_region".into()],
            phase: "A4".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::LocaleSettingSave,
            screen_id: "locale_settings".into(),
            description: "Locale settings saved".into(),
            payload_schema: vec!["locale".into(), "region".into(), "timezone".into()],
            phase: "A4".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::DataRightsView,
            screen_id: "data_rights".into(),
            description: "Data rights page viewed".into(),
            payload_schema: vec![],
            phase: "A4".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::DataExportRequest,
            screen_id: "data_rights".into(),
            description: "User requested data export".into(),
            payload_schema: vec!["export_format".into()],
            phase: "A4".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::DataDeleteRequest,
            screen_id: "data_rights".into(),
            description: "User requested data deletion".into(),
            payload_schema: vec!["scope".into()],
            phase: "A4".into(),
        },
        AnalyticsEventCatalogEntry {
            event_name: AnalyticsEventName::DataCorrectionRequest,
            screen_id: "data_rights".into(),
            description: "User requested data correction".into(),
            payload_schema: vec!["field_name".into()],
            phase: "A4".into(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_is_frozen() {
        let catalog = frozen_analytics_event_catalog();
        assert_eq!(catalog.len(), 45);
    }

    #[test]
    fn test_catalog_has_a0_events() {
        let catalog = frozen_analytics_event_catalog();
        let a0 = catalog.iter().filter(|e| e.phase == "A0").count();
        assert_eq!(a0, 3);
    }

    #[test]
    fn test_catalog_has_a1_events() {
        let catalog = frozen_analytics_event_catalog();
        let a1 = catalog.iter().filter(|e| e.phase == "A1").count();
        assert_eq!(a1, 19);
    }

    #[test]
    fn test_catalog_has_a2_events() {
        let catalog = frozen_analytics_event_catalog();
        let a2 = catalog.iter().filter(|e| e.phase == "A2").count();
        assert_eq!(a2, 12);
    }

    #[test]
    fn test_catalog_has_a3_events() {
        let catalog = frozen_analytics_event_catalog();
        let a3 = catalog.iter().filter(|e| e.phase == "A3").count();
        assert_eq!(a3, 5);
    }

    #[test]
    fn test_catalog_has_a4_events() {
        let catalog = frozen_analytics_event_catalog();
        let a4 = catalog.iter().filter(|e| e.phase == "A4").count();
        assert_eq!(a4, 6);
    }

    #[test]
    fn test_catalog_event_names_unique() {
        let catalog = frozen_analytics_event_catalog();
        let names: Vec<&AnalyticsEventName> = catalog.iter().map(|e| &e.event_name).collect();
        for i in 0..names.len() {
            for j in (i + 1)..names.len() {
                assert_ne!(names[i], names[j], "duplicate event name");
            }
        }
    }

    #[test]
    fn test_event_name_serialization() {
        assert_eq!(
            serde_json::to_string(&AnalyticsEventName::FindRequestStarted).unwrap(),
            "\"find_request_started\""
        );
        assert_eq!(
            serde_json::to_string(&AnalyticsEventName::RecommendationListView).unwrap(),
            "\"recommendation_list_view\""
        );
        assert_eq!(
            serde_json::to_string(&AnalyticsEventName::ChatSend).unwrap(),
            "\"chat_send\""
        );
    }

    #[test]
    fn test_catalog_serializable() {
        let catalog = frozen_analytics_event_catalog();
        let json = serde_json::to_string(&catalog).unwrap();
        assert!(!json.is_empty());
        let parsed: Vec<AnalyticsEventCatalogEntry> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), catalog.len());
    }
}
