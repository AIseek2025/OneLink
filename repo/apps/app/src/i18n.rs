use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const DEFAULT_LOCALE: &str = "zh-CN";
const SUPPORTED_LOCALES: &[&str] = &["zh-CN", "en"];

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Locale {
    language: String,
    region: Option<String>,
}

impl Locale {
    pub fn new(language: &str, region: Option<&str>) -> Self {
        Self {
            language: language.to_lowercase(),
            region: region.map(|r| r.to_uppercase()),
        }
    }

    pub fn parse(tag: &str) -> Result<Self, LocaleError> {
        let tag = tag.trim();
        if tag.is_empty() {
            return Err(LocaleError::EmptyTag);
        }
        let parts: Vec<&str> = tag.splitn(2, '-').collect();
        let language = parts[0].to_lowercase();
        if language.is_empty() || !language.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err(LocaleError::InvalidLanguage(language));
        }
        let region = if parts.len() > 1 {
            let r = parts[1].to_uppercase();
            if r.is_empty() {
                None
            } else {
                Some(r)
            }
        } else {
            None
        };
        Ok(Self { language, region })
    }

    pub fn tag(&self) -> String {
        match &self.region {
            Some(r) => format!("{}-{}", self.language, r),
            None => self.language.clone(),
        }
    }

    pub fn language(&self) -> &str {
        &self.language
    }

    pub fn region(&self) -> Option<&str> {
        self.region.as_deref()
    }

    pub fn is_supported(&self) -> bool {
        let tag = self.tag();
        SUPPORTED_LOCALES.contains(&tag.as_str())
    }

    pub fn resolve_or_default(preferred: &str) -> Self {
        let parsed = Self::parse(preferred);
        if let Ok(loc) = &parsed {
            if loc.is_supported() {
                return loc.clone();
            }
        }
        Self::parse(DEFAULT_LOCALE).unwrap()
    }
}

impl std::fmt::Display for Locale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.tag())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocaleError {
    EmptyTag,
    InvalidLanguage(String),
}

impl std::fmt::Display for LocaleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LocaleError::EmptyTag => write!(f, "locale tag is empty"),
            LocaleError::InvalidLanguage(lang) => write!(f, "invalid language code: {lang}"),
        }
    }
}

impl std::error::Error for LocaleError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct I18nEntry {
    key: String,
    translations: HashMap<String, String>,
}

impl I18nEntry {
    pub fn new(key: &str, translations: HashMap<String, String>) -> Self {
        Self {
            key: key.to_string(),
            translations,
        }
    }

    pub fn translate(&self, locale: &Locale) -> Option<&str> {
        let tag = locale.tag();
        if let Some(v) = self.translations.get(&tag) {
            return Some(v.as_str());
        }
        self.translations.get(DEFAULT_LOCALE).map(|v| v.as_str())
    }
}

#[derive(Debug, Clone, Default)]
pub struct I18nRegistry {
    entries: HashMap<String, I18nEntry>,
}

impl I18nRegistry {
    pub fn new() -> Self {
        let mut registry = Self::default();
        registry.register_defaults();
        registry
    }

    fn register_defaults(&mut self) {
        self.register(I18nEntry::new(
            "safety.block.applied",
            HashMap::from([
                ("zh-CN".to_string(), "该用户已被拉黑".to_string()),
                ("en".to_string(), "This user has been blocked".to_string()),
            ]),
        ));
        self.register(I18nEntry::new(
            "safety.report.confirmation",
            HashMap::from([
                (
                    "zh-CN".to_string(),
                    "举报已提交，我们将尽快处理".to_string(),
                ),
                (
                    "en".to_string(),
                    "Report submitted. We will review it shortly.".to_string(),
                ),
            ]),
        ));
        self.register(I18nEntry::new(
            "safety.reject.harmful",
            HashMap::from([
                (
                    "zh-CN".to_string(),
                    "您的消息因安全原因被拒绝发送".to_string(),
                ),
                (
                    "en".to_string(),
                    "Your message was blocked for safety reasons".to_string(),
                ),
            ]),
        ));
        self.register(I18nEntry::new(
            "privacy.data_export.title",
            HashMap::from([
                ("zh-CN".to_string(), "个人数据导出".to_string()),
                ("en".to_string(), "Personal Data Export".to_string()),
            ]),
        ));
        self.register(I18nEntry::new(
            "privacy.data_delete.confirmation",
            HashMap::from([
                (
                    "zh-CN".to_string(),
                    "删除请求已提交，数据将在规定期限内清除".to_string(),
                ),
                (
                    "en".to_string(),
                    "Deletion request submitted. Data will be removed within the required period."
                        .to_string(),
                ),
            ]),
        ));
        self.register(I18nEntry::new(
            "privacy.data_correction.title",
            HashMap::from([
                ("zh-CN".to_string(), "个人数据纠正".to_string()),
                ("en".to_string(), "Personal Data Correction".to_string()),
            ]),
        ));
        self.register(I18nEntry::new(
            "compliance.underage.warning",
            HashMap::from([
                (
                    "zh-CN".to_string(),
                    "未成年人保护提示：本功能对未成年人有特殊限制".to_string(),
                ),
                (
                    "en".to_string(),
                    "Minor Protection Notice: This feature has special restrictions for minors"
                        .to_string(),
                ),
            ]),
        ));
        self.register(I18nEntry::new(
            "compliance.crossborder.notice",
            HashMap::from([
                (
                    "zh-CN".to_string(),
                    "您的数据可能在不同区域间传输，详见隐私政策".to_string(),
                ),
                (
                    "en".to_string(),
                    "Your data may be transferred between regions. See privacy policy for details."
                        .to_string(),
                ),
            ]),
        ));
        self.register(I18nEntry::new(
            "compliance.region.policy",
            HashMap::from([
                (
                    "zh-CN".to_string(),
                    "数据驻留策略：您的数据存储在您所属区域".to_string(),
                ),
                (
                    "en".to_string(),
                    "Data residency policy: Your data is stored in your assigned region"
                        .to_string(),
                ),
            ]),
        ));
        self.register(I18nEntry::new(
            "privacy.consent.required",
            HashMap::from([
                (
                    "zh-CN".to_string(),
                    "使用本功能需要您同意相关隐私条款".to_string(),
                ),
                (
                    "en".to_string(),
                    "Using this feature requires your consent to relevant privacy terms"
                        .to_string(),
                ),
            ]),
        ));
    }

    pub fn register(&mut self, entry: I18nEntry) {
        self.entries.insert(entry.key.clone(), entry);
    }

    pub fn translate(&self, key: &str, locale: &Locale) -> Option<&str> {
        self.entries.get(key).and_then(|e| e.translate(locale))
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

pub fn supported_locales() -> &'static [&'static str] {
    SUPPORTED_LOCALES
}

pub fn default_locale() -> &'static str {
    DEFAULT_LOCALE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locale_parse_zh_cn() {
        let loc = Locale::parse("zh-CN").unwrap();
        assert_eq!(loc.language(), "zh");
        assert_eq!(loc.region(), Some("CN"));
        assert_eq!(loc.tag(), "zh-CN");
    }

    #[test]
    fn test_locale_parse_en() {
        let loc = Locale::parse("en").unwrap();
        assert_eq!(loc.language(), "en");
        assert_eq!(loc.region(), None);
    }

    #[test]
    fn test_locale_parse_empty_rejected() {
        assert!(matches!(Locale::parse(""), Err(LocaleError::EmptyTag)));
    }

    #[test]
    fn test_locale_is_supported() {
        assert!(Locale::parse("zh-CN").unwrap().is_supported());
        assert!(Locale::parse("en").unwrap().is_supported());
        assert!(!Locale::parse("fr").unwrap().is_supported());
    }

    #[test]
    fn test_resolve_or_default_falls_back() {
        let loc = Locale::resolve_or_default("fr");
        assert_eq!(loc.tag(), "zh-CN");
    }

    #[test]
    fn test_resolve_or_default_keeps_supported() {
        let loc = Locale::resolve_or_default("en");
        assert_eq!(loc.tag(), "en");
    }

    #[test]
    fn test_i18n_registry_translate_zh() {
        let registry = I18nRegistry::new();
        let loc = Locale::parse("zh-CN").unwrap();
        let text = registry.translate("safety.block.applied", &loc).unwrap();
        assert_eq!(text, "该用户已被拉黑");
    }

    #[test]
    fn test_i18n_registry_translate_en() {
        let registry = I18nRegistry::new();
        let loc = Locale::parse("en").unwrap();
        let text = registry.translate("safety.block.applied", &loc).unwrap();
        assert_eq!(text, "This user has been blocked");
    }

    #[test]
    fn test_i18n_registry_falls_to_default_locale() {
        let registry = I18nRegistry::new();
        let loc = Locale::parse("fr").unwrap();
        let text = registry.translate("safety.block.applied", &loc).unwrap();
        assert_eq!(text, "该用户已被拉黑");
    }

    #[test]
    fn test_i18n_registry_missing_key_returns_none() {
        let registry = I18nRegistry::new();
        let loc = Locale::parse("zh-CN").unwrap();
        assert!(registry.translate("nonexistent.key", &loc).is_none());
    }

    #[test]
    fn test_i18n_registry_entry_count() {
        let registry = I18nRegistry::new();
        assert!(registry.entry_count() > 0);
    }

    #[test]
    fn test_privacy_keys_present() {
        let registry = I18nRegistry::new();
        assert!(registry.contains_key("privacy.data_export.title"));
        assert!(registry.contains_key("privacy.data_delete.confirmation"));
        assert!(registry.contains_key("privacy.data_correction.title"));
        assert!(registry.contains_key("privacy.consent.required"));
    }

    #[test]
    fn test_compliance_keys_present() {
        let registry = I18nRegistry::new();
        assert!(registry.contains_key("compliance.underage.warning"));
        assert!(registry.contains_key("compliance.crossborder.notice"));
        assert!(registry.contains_key("compliance.region.policy"));
    }
}
