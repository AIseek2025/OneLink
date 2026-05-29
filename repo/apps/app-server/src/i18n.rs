//! Phase 10 I18N resource layer for the App.
//!
//! Provides:
//! - Message key registry with zh-CN and en translations
//! - Locale resolution (resolve or fallback to zh-CN)
//! - Region/timezone/content_language/notification_language defaults
//! - Data rights message keys (export, delete, correction, privacy)
//! - High-risk content review keys (rejection, appeal, safety, privacy)
//!
//! Rules: rules/20 §3 (文案资源化), §4 (字段模型), §5 (数据权利), §6 (高风险文案审核)

use std::collections::HashMap;

const DEFAULT_LOCALE: &str = "zh-CN";
const SUPPORTED_LOCALES: &[&str] = &["zh-CN", "en"];
const SUPPORTED_REGIONS: &[&str] = &["CN", "US", "EU", "SEA"];
const SUPPORTED_TIMEZONES: &[&str] = &[
    "Asia/Shanghai",
    "America/New_York",
    "Europe/Berlin",
    "Asia/Singapore",
];
const SUPPORTED_CONTENT_LANGUAGES: &[&str] = &["zh-CN", "en"];
const SUPPORTED_NOTIFICATION_LANGUAGES: &[&str] = &["zh-CN", "en"];

#[derive(Debug, Clone)]
pub struct MessageEntry {
    key: String,
    translations: HashMap<String, String>,
    owner: String,
    last_reviewed: String,
}

impl MessageEntry {
    pub fn new(key: &str, zh: &str, en: &str, owner: &str, last_reviewed: &str) -> Self {
        Self {
            key: key.to_string(),
            translations: HashMap::from([
                ("zh-CN".to_string(), zh.to_string()),
                ("en".to_string(), en.to_string()),
            ]),
            owner: owner.to_string(),
            last_reviewed: last_reviewed.to_string(),
        }
    }

    pub fn translate(&self, locale: &str) -> &str {
        if let Some(v) = self.translations.get(locale) {
            return v.as_str();
        }
        self.translations
            .get(DEFAULT_LOCALE)
            .map(|v| v.as_str())
            .unwrap_or(&self.key)
    }

    pub fn key(&self) -> &str {
        &self.key
    }
    pub fn owner(&self) -> &str {
        &self.owner
    }
    pub fn last_reviewed(&self) -> &str {
        &self.last_reviewed
    }
}

#[derive(Debug, Clone, Default)]
pub struct I18nRegistry {
    entries: HashMap<String, MessageEntry>,
}

impl I18nRegistry {
    pub fn new() -> Self {
        let mut reg = Self::default();
        reg.register_all();
        reg
    }

    fn register_all(&mut self) {
        // Phase 10 §3: 文案资源化 — 用户可见文案
        self.register(MessageEntry::new(
            "app.boot.welcome",
            "欢迎使用 OneLink",
            "Welcome to OneLink",
            "app-team",
            "2026-05-25",
        ));
        self.register(MessageEntry::new(
            "app.auth.login.success",
            "登录成功",
            "Login successful",
            "app-team",
            "2026-05-25",
        ));
        self.register(MessageEntry::new(
            "app.auth.register.success",
            "注册成功",
            "Registration successful",
            "app-team",
            "2026-05-25",
        ));
        self.register(MessageEntry::new(
            "app.auth.session.expired",
            "会话已过期，请重新登录",
            "Session expired, please log in again",
            "app-team",
            "2026-05-25",
        ));
        self.register(MessageEntry::new(
            "app.auth.error.invalid_credentials",
            "用户名或密码错误",
            "Invalid credentials",
            "app-team",
            "2026-05-25",
        ));

        // Phase 10 §3: 安全提示文案 (高风险)
        self.register(MessageEntry::new(
            "safety.report.confirmation",
            "举报已提交，我们将尽快处理",
            "Report submitted. We will review it shortly.",
            "safety-team",
            "2026-05-25",
        ));
        self.register(MessageEntry::new(
            "safety.block.applied",
            "该用户已被拉黑",
            "This user has been blocked",
            "safety-team",
            "2026-05-25",
        ));
        self.register(MessageEntry::new(
            "safety.reject.harmful",
            "您的消息因安全原因被拒绝发送",
            "Your message was blocked for safety reasons",
            "safety-team",
            "2026-05-25",
        ));
        self.register(MessageEntry::new(
            "safety.appeal.submitted",
            "申诉已提交，请等待审核结果",
            "Appeal submitted, awaiting review",
            "safety-team",
            "2026-05-25",
        ));
        self.register(MessageEntry::new(
            "safety.appeal.rejected",
            "申诉被驳回",
            "Appeal rejected",
            "safety-team",
            "2026-05-25",
        ));
        self.register(MessageEntry::new(
            "safety.appeal.approved",
            "申诉已通过",
            "Appeal approved",
            "safety-team",
            "2026-05-25",
        ));

        // Phase 10 §3: 拒绝与劝导文案 (高风险)
        self.register(MessageEntry::new(
            "rejection.no_match.title",
            "暂时未找到合适人选",
            "No suitable matches found at this time",
            "match-team",
            "2026-05-25",
        ));
        self.register(MessageEntry::new(
            "rejection.no_match.encouragement",
            "请稍后再试，我们会持续为您寻找",
            "Please try again later, we'll keep searching for you",
            "match-team",
            "2026-05-25",
        ));
        self.register(MessageEntry::new(
            "dm.first_message.under_review",
            "首条消息正在安全审查中",
            "First message is under safety review",
            "safety-team",
            "2026-05-25",
        ));
        self.register(MessageEntry::new(
            "dm.first_message.blocked",
            "首条消息未通过安全审查，无法发送",
            "First message did not pass safety review and cannot be sent",
            "safety-team",
            "2026-05-25",
        ));

        // Phase 10 §5: 用户数据权利文案 (高风险)
        self.register(MessageEntry::new(
            "privacy.data_export.title",
            "个人数据导出",
            "Personal Data Export",
            "compliance-team",
            "2026-05-25",
        ));
        self.register(MessageEntry::new(
            "privacy.data_export.description",
            "您可以导出您的个人数据，包括画像、记忆和设置",
            "You can export your personal data, including profile, memories, and settings",
            "compliance-team",
            "2026-05-25",
        ));
        self.register(MessageEntry::new(
            "privacy.data_export.requested",
            "导出请求已提交，数据将在规定期限内提供",
            "Export request submitted. Data will be provided within the required period.",
            "compliance-team",
            "2026-05-25",
        ));
        self.register(MessageEntry::new(
            "privacy.data_delete.title",
            "个人数据删除",
            "Personal Data Deletion",
            "compliance-team",
            "2026-05-25",
        ));
        self.register(MessageEntry::new(
            "privacy.data_delete.description",
            "您可以请求删除您的个人数据",
            "You can request deletion of your personal data",
            "compliance-team",
            "2026-05-25",
        ));
        self.register(MessageEntry::new(
            "privacy.data_delete.confirmation",
            "删除请求已提交，数据将在规定期限内清除",
            "Deletion request submitted. Data will be removed within the required period.",
            "compliance-team",
            "2026-05-25",
        ));
        self.register(MessageEntry::new(
            "privacy.data_delete.warning",
            "删除操作不可逆，请确认后再提交",
            "Deletion is irreversible. Please confirm before submitting.",
            "compliance-team",
            "2026-05-25",
        ));
        self.register(MessageEntry::new(
            "privacy.data_correction.title",
            "个人数据纠正",
            "Personal Data Correction",
            "compliance-team",
            "2026-05-25",
        ));
        self.register(MessageEntry::new(
            "privacy.data_correction.description",
            "您可以纠正不准确的个人数据",
            "You can correct inaccurate personal data",
            "compliance-team",
            "2026-05-25",
        ));
        self.register(MessageEntry::new(
            "privacy.data_correction.submitted",
            "纠正请求已提交",
            "Correction request submitted",
            "compliance-team",
            "2026-05-25",
        ));
        self.register(MessageEntry::new(
            "privacy.view_data.title",
            "查看我的数据",
            "View My Data",
            "compliance-team",
            "2026-05-25",
        ));

        // Phase 10 §6: 跨境说明文案 (高风险)
        self.register(MessageEntry::new(
            "compliance.crossborder.notice",
            "您的数据可能在不同区域间传输，详见隐私政策",
            "Your data may be transferred between regions. See privacy policy for details.",
            "compliance-team",
            "2026-05-25",
        ));
        self.register(MessageEntry::new(
            "compliance.underage.warning",
            "未成年人保护提示：本功能对未成年人有特殊限制",
            "Minor Protection Notice: This feature has special restrictions for minors",
            "compliance-team",
            "2026-05-25",
        ));
        self.register(MessageEntry::new(
            "compliance.region.policy",
            "数据驻留策略：您的数据存储在您所属区域",
            "Data residency policy: Your data is stored in your assigned region",
            "compliance-team",
            "2026-05-25",
        ));

        // Phase 10 §3: 隐私文案 (高风险)
        self.register(MessageEntry::new(
            "privacy.policy.title",
            "隐私政策",
            "Privacy Policy",
            "compliance-team",
            "2026-05-25",
        ));
        self.register(MessageEntry::new(
            "privacy.consent.required",
            "使用本功能需要您同意相关隐私条款",
            "Using this feature requires your consent to relevant privacy terms",
            "compliance-team",
            "2026-05-25",
        ));

        // Phase 10 §3: Screen title i18n keys (Web/BFF page entry evidence)
        self.register(MessageEntry::new(
            "screen.splash.title",
            "OneLink",
            "OneLink",
            "app-team",
            "2026-05-26",
        ));
        self.register(MessageEntry::new(
            "screen.login.title",
            "登录",
            "Login",
            "app-team",
            "2026-05-26",
        ));
        self.register(MessageEntry::new(
            "screen.register.title",
            "注册",
            "Register",
            "app-team",
            "2026-05-26",
        ));
        self.register(MessageEntry::new(
            "screen.conversations.title",
            "会话列表",
            "Conversations",
            "app-team",
            "2026-05-26",
        ));
        self.register(MessageEntry::new(
            "screen.chat.title",
            "聊天",
            "Chat",
            "app-team",
            "2026-05-26",
        ));
        self.register(MessageEntry::new(
            "screen.profile_confirmation.title",
            "画像确认",
            "Profile Confirmation",
            "app-team",
            "2026-05-26",
        ));
        self.register(MessageEntry::new(
            "screen.profile.title",
            "个人资料",
            "Profile",
            "app-team",
            "2026-05-26",
        ));
        self.register(MessageEntry::new(
            "screen.settings.title",
            "设置",
            "Settings",
            "app-team",
            "2026-05-26",
        ));
        self.register(MessageEntry::new(
            "screen.find.title",
            "发现",
            "Find People",
            "app-team",
            "2026-05-26",
        ));
        self.register(MessageEntry::new(
            "screen.recommendations.title",
            "推荐",
            "Recommendations",
            "app-team",
            "2026-05-26",
        ));
        self.register(MessageEntry::new(
            "screen.recommendation_detail.title",
            "推荐详情",
            "Recommendation Detail",
            "app-team",
            "2026-05-26",
        ));
        self.register(MessageEntry::new(
            "screen.dm_new.title",
            "新消息",
            "New Message",
            "app-team",
            "2026-05-26",
        ));
        self.register(MessageEntry::new(
            "screen.dm_detail.title",
            "消息详情",
            "Message",
            "app-team",
            "2026-05-26",
        ));
        self.register(MessageEntry::new(
            "screen.report.title",
            "举报",
            "Report",
            "app-team",
            "2026-05-26",
        ));
        self.register(MessageEntry::new(
            "screen.block.title",
            "拉黑",
            "Block",
            "app-team",
            "2026-05-26",
        ));
        self.register(MessageEntry::new(
            "screen.appeal.title",
            "申诉状态",
            "Appeal Status",
            "app-team",
            "2026-05-26",
        ));
        self.register(MessageEntry::new(
            "screen.locale_settings.title",
            "语言与地区",
            "Language & Region",
            "app-team",
            "2026-05-26",
        ));
        self.register(MessageEntry::new(
            "screen.data_rights.title",
            "数据权利",
            "Data Rights",
            "app-team",
            "2026-05-26",
        ));

        // Phase 10 §5: Compliance action labels (Web/BFF page entry evidence)
        self.register(MessageEntry::new(
            "compliance.action.view_data",
            "查看我的数据",
            "View My Data",
            "compliance-team",
            "2026-05-26",
        ));
        self.register(MessageEntry::new(
            "compliance.action.export_data",
            "导出我的数据",
            "Export My Data",
            "compliance-team",
            "2026-05-26",
        ));
        self.register(MessageEntry::new(
            "compliance.action.correct_data",
            "纠正我的数据",
            "Correct My Data",
            "compliance-team",
            "2026-05-26",
        ));
        self.register(MessageEntry::new(
            "compliance.action.delete_data",
            "删除我的数据",
            "Delete My Data",
            "compliance-team",
            "2026-05-26",
        ));
    }

    pub fn register(&mut self, entry: MessageEntry) {
        self.entries.insert(entry.key.clone(), entry);
    }

    pub fn translate(&self, key: &str, locale: &str) -> String {
        self.entries
            .get(key)
            .map(|e| e.translate(locale).to_string())
            .unwrap_or_else(|| format!("[{key}]"))
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    pub fn all_keys(&self) -> Vec<&str> {
        self.entries.keys().map(|k| k.as_str()).collect()
    }

    pub fn entry_review_info(&self, key: &str) -> Option<(&str, &str)> {
        self.entries
            .get(key)
            .map(|e| (e.owner(), e.last_reviewed()))
    }
}

pub fn resolve_locale(preferred: &str) -> &'static str {
    if SUPPORTED_LOCALES.contains(&preferred) {
        return match preferred {
            "zh-CN" => "zh-CN",
            "en" => "en",
            _ => DEFAULT_LOCALE,
        };
    }
    DEFAULT_LOCALE
}

pub fn supported_locales() -> Vec<String> {
    SUPPORTED_LOCALES.iter().map(|s| s.to_string()).collect()
}
pub fn supported_regions() -> Vec<String> {
    SUPPORTED_REGIONS.iter().map(|s| s.to_string()).collect()
}
pub fn supported_timezones() -> Vec<String> {
    SUPPORTED_TIMEZONES.iter().map(|s| s.to_string()).collect()
}
pub fn supported_content_languages() -> Vec<String> {
    SUPPORTED_CONTENT_LANGUAGES
        .iter()
        .map(|s| s.to_string())
        .collect()
}
pub fn supported_notification_languages() -> Vec<String> {
    SUPPORTED_NOTIFICATION_LANGUAGES
        .iter()
        .map(|s| s.to_string())
        .collect()
}
pub fn default_locale() -> &'static str {
    DEFAULT_LOCALE
}
pub fn default_region() -> &'static str {
    "CN"
}
pub fn default_timezone() -> &'static str {
    "Asia/Shanghai"
}
pub fn default_content_language() -> &'static str {
    "zh-CN"
}
pub fn default_notification_language() -> &'static str {
    "zh-CN"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_has_minimum_entries() {
        let reg = I18nRegistry::new();
        assert!(reg.entry_count() >= 50, "must have at least 50 message keys covering all high-risk categories + screen titles + compliance actions");
        println!("[i18n] entry_count={}", reg.entry_count());
    }

    #[test]
    fn test_translate_zh_cn() {
        let reg = I18nRegistry::new();
        let text = reg.translate("safety.block.applied", "zh-CN");
        assert_eq!(text, "该用户已被拉黑");
        println!("[i18n-zh] safety.block.applied => {}", text);
    }

    #[test]
    fn test_translate_en() {
        let reg = I18nRegistry::new();
        let text = reg.translate("safety.block.applied", "en");
        assert_eq!(text, "This user has been blocked");
        println!("[i18n-en] safety.block.applied => {}", text);
    }

    #[test]
    fn test_translate_fallback() {
        let reg = I18nRegistry::new();
        let text = reg.translate("safety.block.applied", "ja-JP");
        assert_eq!(text, "该用户已被拉黑");
        println!("[i18n-fallback] safety.block.applied (ja-JP) => {}", text);
    }

    #[test]
    fn test_missing_key() {
        let reg = I18nRegistry::new();
        let text = reg.translate("nonexistent.key", "zh-CN");
        assert!(text.contains("nonexistent.key"));
        println!("[i18n-missing] nonexistent.key => {}", text);
    }

    #[test]
    fn test_data_rights_keys() {
        let reg = I18nRegistry::new();
        assert!(reg.contains_key("privacy.data_export.title"));
        assert!(reg.contains_key("privacy.data_delete.title"));
        assert!(reg.contains_key("privacy.data_correction.title"));
        assert!(reg.contains_key("privacy.view_data.title"));
        println!(
            "[i18n-data-rights] export={}, delete={}, correction={}, view={}",
            reg.contains_key("privacy.data_export.title"),
            reg.contains_key("privacy.data_delete.title"),
            reg.contains_key("privacy.data_correction.title"),
            reg.contains_key("privacy.view_data.title")
        );
    }

    #[test]
    fn test_high_risk_review_keys() {
        let reg = I18nRegistry::new();
        for key in &[
            "safety.report.confirmation",
            "safety.block.applied",
            "rejection.no_match.title",
            "safety.appeal.submitted",
            "privacy.data_delete.confirmation",
            "compliance.crossborder.notice",
            "privacy.consent.required",
        ] {
            let (owner, reviewed) = reg.entry_review_info(key).unwrap();
            assert!(!owner.is_empty());
            assert!(!reviewed.is_empty());
            println!(
                "[i18n-review] {} owner={} reviewed={}",
                key, owner, reviewed
            );
        }
    }

    #[test]
    fn test_locale_resolution() {
        assert_eq!(resolve_locale("zh-CN"), "zh-CN");
        assert_eq!(resolve_locale("en"), "en");
        assert_eq!(resolve_locale("ja"), DEFAULT_LOCALE);
        println!(
            "[locale] zh-CN={}, en={}, ja=>{}",
            resolve_locale("zh-CN"),
            resolve_locale("en"),
            resolve_locale("ja")
        );
    }

    #[test]
    fn test_supported_lists() {
        let locales = supported_locales();
        assert!(locales.contains(&"zh-CN".to_string()));
        assert!(locales.contains(&"en".to_string()));
        let regions = supported_regions();
        assert!(regions.len() >= 4);
        let timezones = supported_timezones();
        assert!(timezones.len() >= 4);
        println!(
            "[locale-lists] locales={}, regions={}, timezones={}",
            locales.len(),
            regions.len(),
            timezones.len()
        );
    }

    #[test]
    fn test_defaults() {
        assert_eq!(default_locale(), "zh-CN");
        assert_eq!(default_region(), "CN");
        assert_eq!(default_timezone(), "Asia/Shanghai");
        assert_eq!(default_content_language(), "zh-CN");
        assert_eq!(default_notification_language(), "zh-CN");
        println!(
            "[defaults] locale={}, region={}, tz={}, content_lang={}, notif_lang={}",
            default_locale(),
            default_region(),
            default_timezone(),
            default_content_language(),
            default_notification_language()
        );
    }
}
