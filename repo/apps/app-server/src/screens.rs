use serde::{Deserialize, Serialize};

use crate::i18n::I18nRegistry;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ScreenId {
    Splash,
    Login,
    Register,
    ConversationList,
    Chat,
    ProfileConfirmation,
    Profile,
    Settings,
    FindInput,
    RecommendationList,
    RecommendationDetail,
    DmFirstMessage,
    DmDetail,
    Report,
    Block,
    AppealStatus,
    LocaleSettings,
    DataRights,
}

impl ScreenId {
    pub fn route(&self) -> &str {
        match self {
            ScreenId::Splash => "/",
            ScreenId::Login => "/login",
            ScreenId::Register => "/register",
            ScreenId::ConversationList => "/conversations",
            ScreenId::Chat => "/chat/{id}",
            ScreenId::ProfileConfirmation => "/profile/confirmations",
            ScreenId::Profile => "/profile",
            ScreenId::Settings => "/settings",
            ScreenId::FindInput => "/find",
            ScreenId::RecommendationList => "/recommendations",
            ScreenId::RecommendationDetail => "/recommendations/{id}",
            ScreenId::DmFirstMessage => "/dm/new",
            ScreenId::DmDetail => "/dm/{id}",
            ScreenId::Report => "/report",
            ScreenId::Block => "/block",
            ScreenId::AppealStatus => "/appeal/{id}",
            ScreenId::LocaleSettings => "/settings/locale",
            ScreenId::DataRights => "/settings/data-rights",
        }
    }

    pub fn title_key(&self) -> &str {
        match self {
            ScreenId::Splash => "screen.splash.title",
            ScreenId::Login => "screen.login.title",
            ScreenId::Register => "screen.register.title",
            ScreenId::ConversationList => "screen.conversations.title",
            ScreenId::Chat => "screen.chat.title",
            ScreenId::ProfileConfirmation => "screen.profile_confirmation.title",
            ScreenId::Profile => "screen.profile.title",
            ScreenId::Settings => "screen.settings.title",
            ScreenId::FindInput => "screen.find.title",
            ScreenId::RecommendationList => "screen.recommendations.title",
            ScreenId::RecommendationDetail => "screen.recommendation_detail.title",
            ScreenId::DmFirstMessage => "screen.dm_new.title",
            ScreenId::DmDetail => "screen.dm_detail.title",
            ScreenId::Report => "screen.report.title",
            ScreenId::Block => "screen.block.title",
            ScreenId::AppealStatus => "screen.appeal.title",
            ScreenId::LocaleSettings => "screen.locale_settings.title",
            ScreenId::DataRights => "screen.data_rights.title",
        }
    }

    pub fn title(&self) -> &str {
        match self {
            ScreenId::Splash => "OneLink",
            ScreenId::Login => "Login",
            ScreenId::Register => "Register",
            ScreenId::ConversationList => "Conversations",
            ScreenId::Chat => "Chat",
            ScreenId::ProfileConfirmation => "Profile Confirmation",
            ScreenId::Profile => "Profile",
            ScreenId::Settings => "Settings",
            ScreenId::FindInput => "Find People",
            ScreenId::RecommendationList => "Recommendations",
            ScreenId::RecommendationDetail => "Recommendation Detail",
            ScreenId::DmFirstMessage => "New Message",
            ScreenId::DmDetail => "Message",
            ScreenId::Report => "Report",
            ScreenId::Block => "Block",
            ScreenId::AppealStatus => "Appeal Status",
            ScreenId::LocaleSettings => "Language & Region",
            ScreenId::DataRights => "Data Rights",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenSpec {
    pub screen_id: ScreenId,
    pub route: String,
    pub title: String,
    pub title_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub localized_title: Option<String>,
    pub bff_contracts: Vec<String>,
    pub owner_services: Vec<String>,
    pub priority: ScreenPriority,
}

impl ScreenSpec {
    pub fn with_locale(&self, locale: &str) -> Self {
        let reg = I18nRegistry::new();
        let localized = reg.translate(&self.title_key, locale);
        Self {
            localized_title: Some(localized),
            ..self.clone()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ScreenPriority {
    A0,
    A1,
    A2,
    A3,
    A4,
}

pub fn a0_a1_screens() -> Vec<ScreenSpec> {
    vec![
        ScreenSpec {
            screen_id: ScreenId::Splash,
            route: ScreenId::Splash.route().to_string(),
            title: ScreenId::Splash.title().to_string(),
            title_key: ScreenId::Splash.title_key().to_string(),
            localized_title: None,
            bff_contracts: vec!["auth/session/refresh".into(), "me/summary".into()],
            owner_services: vec!["identity-service".into()],
            priority: ScreenPriority::A0,
        },
        ScreenSpec {
            screen_id: ScreenId::Login,
            route: ScreenId::Login.route().to_string(),
            title: ScreenId::Login.title().to_string(),
            title_key: ScreenId::Login.title_key().to_string(),
            localized_title: None,
            bff_contracts: vec!["auth/login".into()],
            owner_services: vec!["identity-service".into()],
            priority: ScreenPriority::A1,
        },
        ScreenSpec {
            screen_id: ScreenId::Register,
            route: ScreenId::Register.route().to_string(),
            title: ScreenId::Register.title().to_string(),
            title_key: ScreenId::Register.title_key().to_string(),
            localized_title: None,
            bff_contracts: vec!["auth/register".into()],
            owner_services: vec!["identity-service".into()],
            priority: ScreenPriority::A1,
        },
        ScreenSpec {
            screen_id: ScreenId::ConversationList,
            route: ScreenId::ConversationList.route().to_string(),
            title: ScreenId::ConversationList.title().to_string(),
            title_key: ScreenId::ConversationList.title_key().to_string(),
            localized_title: None,
            bff_contracts: vec!["chat/conversations".into()],
            owner_services: vec!["ai-chat-service".into()],
            priority: ScreenPriority::A1,
        },
        ScreenSpec {
            screen_id: ScreenId::Chat,
            route: ScreenId::Chat.route().to_string(),
            title: ScreenId::Chat.title().to_string(),
            title_key: ScreenId::Chat.title_key().to_string(),
            localized_title: None,
            bff_contracts: vec!["chat/messages".into(), "chat/conversations/{id}".into()],
            owner_services: vec!["ai-chat-service".into(), "model-gateway".into()],
            priority: ScreenPriority::A1,
        },
        ScreenSpec {
            screen_id: ScreenId::ProfileConfirmation,
            route: ScreenId::ProfileConfirmation.route().to_string(),
            title: ScreenId::ProfileConfirmation.title().to_string(),
            title_key: ScreenId::ProfileConfirmation.title_key().to_string(),
            localized_title: None,
            bff_contracts: vec![
                "profile/confirmations".into(),
                "profile/confirmations/{id}/actions".into(),
            ],
            owner_services: vec!["profile-service".into(), "context-service".into()],
            priority: ScreenPriority::A1,
        },
        ScreenSpec {
            screen_id: ScreenId::Profile,
            route: ScreenId::Profile.route().to_string(),
            title: ScreenId::Profile.title().to_string(),
            title_key: ScreenId::Profile.title_key().to_string(),
            localized_title: None,
            bff_contracts: vec!["profile/me".into()],
            owner_services: vec!["profile-service".into()],
            priority: ScreenPriority::A1,
        },
        ScreenSpec {
            screen_id: ScreenId::Settings,
            route: ScreenId::Settings.route().to_string(),
            title: ScreenId::Settings.title().to_string(),
            title_key: ScreenId::Settings.title_key().to_string(),
            localized_title: None,
            bff_contracts: vec!["settings/summary".into()],
            owner_services: vec!["profile-service".into(), "bff".into()],
            priority: ScreenPriority::A1,
        },
    ]
}

pub fn a2_screens() -> Vec<ScreenSpec> {
    vec![
        ScreenSpec {
            screen_id: ScreenId::FindInput,
            route: ScreenId::FindInput.route().to_string(),
            title: ScreenId::FindInput.title().to_string(),
            title_key: ScreenId::FindInput.title_key().to_string(),
            localized_title: None,
            bff_contracts: vec!["find/requests".into(), "find/requests/{id}".into()],
            owner_services: vec!["match-service".into(), "bff".into()],
            priority: ScreenPriority::A2,
        },
        ScreenSpec {
            screen_id: ScreenId::RecommendationList,
            route: ScreenId::RecommendationList.route().to_string(),
            title: ScreenId::RecommendationList.title().to_string(),
            title_key: ScreenId::RecommendationList.title_key().to_string(),
            localized_title: None,
            bff_contracts: vec!["recommendations".into()],
            owner_services: vec!["match-service".into()],
            priority: ScreenPriority::A2,
        },
        ScreenSpec {
            screen_id: ScreenId::RecommendationDetail,
            route: ScreenId::RecommendationDetail.route().to_string(),
            title: ScreenId::RecommendationDetail.title().to_string(),
            title_key: ScreenId::RecommendationDetail.title_key().to_string(),
            localized_title: None,
            bff_contracts: vec![
                "recommendations/{id}".into(),
                "recommendations/{id}/feedback".into(),
            ],
            owner_services: vec!["match-service".into(), "profile-service".into()],
            priority: ScreenPriority::A2,
        },
    ]
}

pub fn a3_screens() -> Vec<ScreenSpec> {
    vec![
        ScreenSpec {
            screen_id: ScreenId::DmFirstMessage,
            route: ScreenId::DmFirstMessage.route().to_string(),
            title: ScreenId::DmFirstMessage.title().to_string(),
            title_key: ScreenId::DmFirstMessage.title_key().to_string(),
            localized_title: None,
            bff_contracts: vec!["dm/threads/draft".into(), "dm/threads/first-message".into()],
            owner_services: vec!["dm-service".into(), "safety-service".into()],
            priority: ScreenPriority::A3,
        },
        ScreenSpec {
            screen_id: ScreenId::DmDetail,
            route: ScreenId::DmDetail.route().to_string(),
            title: ScreenId::DmDetail.title().to_string(),
            title_key: ScreenId::DmDetail.title_key().to_string(),
            localized_title: None,
            bff_contracts: vec!["dm/threads/{id}".into()],
            owner_services: vec!["dm-service".into()],
            priority: ScreenPriority::A3,
        },
        ScreenSpec {
            screen_id: ScreenId::Report,
            route: ScreenId::Report.route().to_string(),
            title: ScreenId::Report.title().to_string(),
            title_key: ScreenId::Report.title_key().to_string(),
            localized_title: None,
            bff_contracts: vec!["safety/reports".into()],
            owner_services: vec!["safety-service".into()],
            priority: ScreenPriority::A3,
        },
        ScreenSpec {
            screen_id: ScreenId::Block,
            route: ScreenId::Block.route().to_string(),
            title: ScreenId::Block.title().to_string(),
            title_key: ScreenId::Block.title_key().to_string(),
            localized_title: None,
            bff_contracts: vec!["safety/blocks".into()],
            owner_services: vec!["safety-service".into()],
            priority: ScreenPriority::A3,
        },
        ScreenSpec {
            screen_id: ScreenId::AppealStatus,
            route: ScreenId::AppealStatus.route().to_string(),
            title: ScreenId::AppealStatus.title().to_string(),
            title_key: ScreenId::AppealStatus.title_key().to_string(),
            localized_title: None,
            bff_contracts: vec!["safety/appeals/{id}".into()],
            owner_services: vec!["safety-service".into()],
            priority: ScreenPriority::A3,
        },
    ]
}

pub fn a4_screens() -> Vec<ScreenSpec> {
    vec![
        ScreenSpec {
            screen_id: ScreenId::LocaleSettings,
            route: ScreenId::LocaleSettings.route().to_string(),
            title: ScreenId::LocaleSettings.title().to_string(),
            title_key: ScreenId::LocaleSettings.title_key().to_string(),
            localized_title: None,
            bff_contracts: vec![
                "settings/locale".into(),
                "settings/update".into(),
                "locale/registry".into(),
            ],
            owner_services: vec!["profile-service".into(), "bff".into()],
            priority: ScreenPriority::A4,
        },
        ScreenSpec {
            screen_id: ScreenId::DataRights,
            route: ScreenId::DataRights.route().to_string(),
            title: ScreenId::DataRights.title().to_string(),
            title_key: ScreenId::DataRights.title_key().to_string(),
            localized_title: None,
            bff_contracts: vec![
                "compliance/summary".into(),
                "compliance/export".into(),
                "compliance/delete".into(),
                "compliance/correction".into(),
            ],
            owner_services: vec![
                "profile-service".into(),
                "context-service".into(),
                "bff".into(),
            ],
            priority: ScreenPriority::A4,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a0_a1_screens_count() {
        let screens = a0_a1_screens();
        assert_eq!(screens.len(), 8);
    }

    #[test]
    fn test_splash_is_a0() {
        let screens = a0_a1_screens();
        let splash = screens
            .iter()
            .find(|s| s.screen_id == ScreenId::Splash)
            .unwrap();
        assert_eq!(splash.priority, ScreenPriority::A0);
    }

    #[test]
    fn test_login_is_a1() {
        let screens = a0_a1_screens();
        let login = screens
            .iter()
            .find(|s| s.screen_id == ScreenId::Login)
            .unwrap();
        assert_eq!(login.priority, ScreenPriority::A1);
    }

    #[test]
    fn test_screen_routes_are_unique() {
        let screens = a0_a1_screens();
        let routes: Vec<&str> = screens.iter().map(|s| s.route.as_str()).collect();
        for i in 0..routes.len() {
            for j in (i + 1)..routes.len() {
                assert_ne!(routes[i], routes[j], "duplicate route: {}", routes[i]);
            }
        }
    }

    #[test]
    fn test_screen_id_route_roundtrip() {
        assert_eq!(ScreenId::Splash.route(), "/");
        assert_eq!(ScreenId::Login.route(), "/login");
        assert_eq!(ScreenId::Chat.route(), "/chat/{id}");
    }

    #[test]
    fn test_a2_screens_count() {
        let screens = a2_screens();
        assert_eq!(screens.len(), 3);
    }

    #[test]
    fn test_a2_screens_have_a2_priority() {
        let screens = a2_screens();
        for s in &screens {
            assert_eq!(
                s.priority,
                ScreenPriority::A2,
                "{} should be A2",
                s.screen_id.route()
            );
        }
    }

    #[test]
    fn test_a2_screens_routes_unique() {
        let a01 = a0_a1_screens();
        let a2 = a2_screens();
        let all_routes: Vec<&str> = a01
            .iter()
            .chain(a2.iter())
            .map(|s| s.route.as_str())
            .collect();
        for i in 0..all_routes.len() {
            for j in (i + 1)..all_routes.len() {
                assert_ne!(
                    all_routes[i], all_routes[j],
                    "duplicate route: {}",
                    all_routes[i]
                );
            }
        }
    }

    #[test]
    fn test_a3_screens_count() {
        let screens = a3_screens();
        assert_eq!(screens.len(), 5);
    }

    #[test]
    fn test_a3_screens_have_a3_priority() {
        let screens = a3_screens();
        for s in &screens {
            assert_eq!(
                s.priority,
                ScreenPriority::A3,
                "{} should be A3",
                s.screen_id.route()
            );
        }
    }

    #[test]
    fn test_a4_screens_count() {
        let screens = a4_screens();
        assert_eq!(screens.len(), 2);
    }

    #[test]
    fn test_a4_screens_have_a4_priority() {
        let screens = a4_screens();
        for s in &screens {
            assert_eq!(
                s.priority,
                ScreenPriority::A4,
                "{} should be A4",
                s.screen_id.route()
            );
        }
    }

    #[test]
    fn test_all_screens_routes_unique() {
        let a0_a1 = a0_a1_screens();
        let a2 = a2_screens();
        let a3 = a3_screens();
        let a4 = a4_screens();
        let all_routes: Vec<&str> = a0_a1
            .iter()
            .chain(a2.iter())
            .chain(a3.iter())
            .chain(a4.iter())
            .map(|s| s.route.as_str())
            .collect();
        for i in 0..all_routes.len() {
            for j in (i + 1)..all_routes.len() {
                assert_ne!(
                    all_routes[i], all_routes[j],
                    "duplicate route: {}",
                    all_routes[i]
                );
            }
        }
    }

    #[test]
    fn test_dm_first_message_route() {
        assert_eq!(ScreenId::DmFirstMessage.route(), "/dm/new");
    }

    #[test]
    fn test_report_route() {
        assert_eq!(ScreenId::Report.route(), "/report");
    }

    #[test]
    fn test_block_route() {
        assert_eq!(ScreenId::Block.route(), "/block");
    }

    #[test]
    fn test_locale_settings_route() {
        assert_eq!(ScreenId::LocaleSettings.route(), "/settings/locale");
    }

    #[test]
    fn test_data_rights_route() {
        assert_eq!(ScreenId::DataRights.route(), "/settings/data-rights");
    }

    #[test]
    fn test_find_input_route() {
        assert_eq!(ScreenId::FindInput.route(), "/find");
    }

    #[test]
    fn test_recommendation_list_route() {
        assert_eq!(ScreenId::RecommendationList.route(), "/recommendations");
    }

    #[test]
    fn test_recommendation_detail_route() {
        assert_eq!(
            ScreenId::RecommendationDetail.route(),
            "/recommendations/{id}"
        );
    }
}
