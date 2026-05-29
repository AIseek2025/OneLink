//! Locale and i18n registry provider for BFF.
//!
//! Provides a config-driven abstraction layer for locale/region/timezone data.
//! This decouples the route handlers from direct config access and establishes
//! a clear boundary where external config sources (config center, file, database)
//! can be plugged in.
//!
//! The `source`, `loaded_at`, and `generation` fields identify where the data
//! came from, when it was loaded, and which reload generation it belongs to,
//! making the registry auditable and hot-swap ready.

use crate::config::Config;
use std::collections::HashMap;
use std::time::SystemTime;

type TranslationStore = HashMap<String, HashMap<String, String>>;

#[derive(Debug, Clone)]
pub struct LocaleRegistry {
    default_locale: String,
    default_region: String,
    supported_locales: Vec<String>,
    supported_regions: Vec<String>,
    supported_timezones: Vec<String>,
    source: String,
    loaded_at: u64,
    generation: u64,
    translations: TranslationStore,
    translations_source: String,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn parse_translations(raw: &str) -> TranslationStore {
    serde_json::from_str::<TranslationStore>(raw).unwrap_or_else(|_| TranslationStore::new())
}

impl LocaleRegistry {
    pub fn from_config(config: &Config) -> Self {
        Self {
            default_locale: config.default_locale.clone(),
            default_region: config.default_region.clone(),
            supported_locales: config.supported_locales.clone(),
            supported_regions: config.supported_regions.clone(),
            supported_timezones: config.supported_timezones.clone(),
            source: "env_config".to_string(),
            loaded_at: now_secs(),
            generation: 0,
            translations: parse_translations(&config.translations_inline),
            translations_source: config.translations_source.clone(),
        }
    }

    pub fn reload(&mut self, config: &Config) {
        self.default_locale = config.default_locale.clone();
        self.default_region = config.default_region.clone();
        self.supported_locales = config.supported_locales.clone();
        self.supported_regions = config.supported_regions.clone();
        self.supported_timezones = config.supported_timezones.clone();
        self.translations = parse_translations(&config.translations_inline);
        self.translations_source = config.translations_source.clone();
        self.loaded_at = now_secs();
        self.generation += 1;
    }

    pub fn translate(&self, locale: &str, key: &str) -> Option<&str> {
        self.translations
            .get(locale)
            .and_then(|m| m.get(key))
            .map(|s| s.as_str())
    }

    pub fn translation_count(&self) -> usize {
        self.translations.values().map(|m| m.len()).sum()
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn default_locale(&self) -> &str {
        &self.default_locale
    }

    pub fn default_region(&self) -> &str {
        &self.default_region
    }

    pub fn supported_locales(&self) -> &[String] {
        &self.supported_locales
    }

    pub fn supported_regions(&self) -> &[String] {
        &self.supported_regions
    }

    pub fn supported_timezones(&self) -> &[String] {
        &self.supported_timezones
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn loaded_at(&self) -> u64 {
        self.loaded_at
    }

    pub fn locale_settings_snapshot(&self, env_mode: &str) -> serde_json::Value {
        serde_json::json!({
            "default_locale": self.default_locale,
            "default_region": self.default_region,
            "available_locales": self.supported_locales,
            "available_regions": self.supported_regions,
            "available_timezones": self.supported_timezones,
            "env_mode": env_mode,
            "source": self.source,
            "loaded_at": self.loaded_at,
            "generation": self.generation,
            "translations_source": self.translations_source,
            "translation_count": self.translation_count()
        })
    }

    pub fn region_gate_snapshot(&self, env_mode: &str) -> serde_json::Value {
        serde_json::json!({
            "region": self.default_region,
            "data_residency": "local",
            "i18n_enabled": true,
            "env_mode": env_mode,
            "default_locale": self.default_locale,
            "supported_locales": self.supported_locales,
            "supported_regions": self.supported_regions,
            "source": self.source,
            "loaded_at": self.loaded_at,
            "generation": self.generation
        })
    }

    pub fn i18n_registry_snapshot(&self, env_mode: &str) -> serde_json::Value {
        serde_json::json!({
            "default_locale": self.default_locale,
            "default_region": self.default_region,
            "supported_locales": self.supported_locales,
            "available_regions": self.supported_regions,
            "available_timezones": self.supported_timezones,
            "env_mode": env_mode,
            "source": self.source,
            "loaded_at": self.loaded_at,
            "generation": self.generation,
            "translations_source": self.translations_source,
            "translation_count": self.translation_count()
        })
    }

    pub fn i18n_translate_lookup(
        &self,
        key: &str,
        locale: Option<&str>,
        env_mode: &str,
    ) -> serde_json::Value {
        let target_locale = locale.unwrap_or(&self.default_locale);
        let (resolved, found_locale, fallback_used) =
            if let Some(v) = self.translate(target_locale, key) {
                (v.to_string(), target_locale.to_string(), false)
            } else if let Some(v) = self.translate(&self.default_locale, key) {
                (
                    v.to_string(),
                    self.default_locale.clone(),
                    target_locale != self.default_locale,
                )
            } else {
                (key.to_string(), self.default_locale.clone(), true)
            };

        serde_json::json!({
            "key": key,
            "requested_locale": target_locale,
            "resolved_locale": found_locale,
            "value": resolved,
            "fallback_used": fallback_used,
            "default_locale": self.default_locale,
            "default_region": self.default_region,
            "env_mode": env_mode,
            "source": self.source,
            "loaded_at": self.loaded_at,
            "generation": self.generation,
            "translations_source": self.translations_source
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Config {
        Config {
            port: 8083,
            identity_service_base_url: "http://127.0.0.1:8081".to_string(),
            ai_chat_service_base_url: "http://127.0.0.1:8085".to_string(),
            question_service_base_url: "http://127.0.0.1:8086".to_string(),
            profile_service_base_url: "http://127.0.0.1:8082".to_string(),
            match_service_base_url: "http://127.0.0.1:8087".to_string(),
            safety_service_base_url: "http://127.0.0.1:8088".to_string(),
            dm_service_base_url: "http://127.0.0.1:8089".to_string(),
            admin_service_base_url: "http://127.0.0.1:8090".to_string(),
            internal_shared_secret: "test-secret".to_string(),
            env_mode: "dev".to_string(),
            cors_allowed_origins: "http://localhost:3000".to_string(),
            default_locale: "zh-CN".to_string(),
            default_region: "CN".to_string(),
            supported_locales: vec!["zh-CN".to_string(), "en-US".to_string()],
            supported_regions: vec!["CN".to_string(), "US".to_string()],
            supported_timezones: vec!["Asia/Shanghai".to_string(), "America/New_York".to_string()],
            translations_source: "env_inline".to_string(),
            translations_inline: r#"{"zh-CN":{"app.title":"OneLink","btn.submit":"提交"},"en-US":{"app.title":"OneLink","btn.submit":"Submit"}}"#.to_string(),
        }
    }

    #[test]
    fn test_registry_from_config() {
        let config = test_config();
        let registry = LocaleRegistry::from_config(&config);

        assert_eq!(registry.default_locale(), "zh-CN");
        assert_eq!(registry.default_region(), "CN");
        assert_eq!(registry.supported_locales().len(), 2);
        assert_eq!(registry.supported_regions().len(), 2);
        assert_eq!(registry.supported_timezones().len(), 2);
        assert_eq!(registry.generation(), 0);
        assert!(registry.translation_count() > 0);
    }

    #[test]
    fn test_translate_exact_locale() {
        let config = test_config();
        let registry = LocaleRegistry::from_config(&config);

        assert_eq!(registry.translate("zh-CN", "btn.submit"), Some("提交"));
        assert_eq!(registry.translate("en-US", "btn.submit"), Some("Submit"));
        assert_eq!(registry.translate("en-US", "app.title"), Some("OneLink"));
    }

    #[test]
    fn test_translate_fallback_to_default() {
        let config = test_config();
        let registry = LocaleRegistry::from_config(&config);

        assert_eq!(registry.translate("ja-JP", "btn.submit"), None);
    }

    #[test]
    fn test_translate_missing_key() {
        let config = test_config();
        let registry = LocaleRegistry::from_config(&config);

        assert_eq!(registry.translate("zh-CN", "nonexistent.key"), None);
    }

    #[test]
    fn test_i18n_translate_lookup_hit() {
        let config = test_config();
        let registry = LocaleRegistry::from_config(&config);
        let result = registry.i18n_translate_lookup("btn.submit", Some("zh-CN"), "dev");

        assert_eq!(result["key"], "btn.submit");
        assert_eq!(result["value"], "提交");
        assert_eq!(result["resolved_locale"], "zh-CN");
        assert_eq!(result["fallback_used"], false);
    }

    #[test]
    fn test_i18n_translate_lookup_fallback() {
        let config = test_config();
        let registry = LocaleRegistry::from_config(&config);
        let result = registry.i18n_translate_lookup("btn.submit", Some("ja-JP"), "dev");

        assert_eq!(result["key"], "btn.submit");
        assert_eq!(result["value"], "提交");
        assert_eq!(result["resolved_locale"], "zh-CN");
        assert_eq!(result["fallback_used"], true);
    }

    #[test]
    fn test_i18n_translate_lookup_missing_key() {
        let config = test_config();
        let registry = LocaleRegistry::from_config(&config);
        let result = registry.i18n_translate_lookup("nonexistent", Some("en-US"), "dev");

        assert_eq!(result["key"], "nonexistent");
        assert_eq!(result["value"], "nonexistent");
        assert_eq!(result["fallback_used"], true);
    }

    #[test]
    fn test_i18n_translate_lookup_default_locale() {
        let config = test_config();
        let registry = LocaleRegistry::from_config(&config);
        let result = registry.i18n_translate_lookup("app.title", None, "production");

        assert_eq!(result["value"], "OneLink");
        assert_eq!(result["resolved_locale"], "zh-CN");
        assert_eq!(result["env_mode"], "production");
    }

    #[test]
    fn test_reload_increments_generation() {
        let config = test_config();
        let mut registry = LocaleRegistry::from_config(&config);
        let gen0 = registry.generation();
        let loaded_at_0 = registry.loaded_at();

        assert_eq!(gen0, 0);

        let new_config = test_config();
        registry.reload(&new_config);

        assert_eq!(registry.generation(), 1);
        assert!(registry.loaded_at() >= loaded_at_0);
    }

    #[test]
    fn test_locale_settings_snapshot() {
        let config = test_config();
        let registry = LocaleRegistry::from_config(&config);
        let snapshot = registry.locale_settings_snapshot("dev");

        assert_eq!(snapshot["default_locale"], "zh-CN");
        assert_eq!(snapshot["default_region"], "CN");
        assert_eq!(snapshot["env_mode"], "dev");
        assert_eq!(snapshot["generation"], 0);
        assert!(snapshot["available_locales"].is_array());
        assert!(snapshot["available_regions"].is_array());
        assert!(snapshot["available_timezones"].is_array());
        assert!(snapshot["translation_count"].as_u64().unwrap() > 0);
    }

    #[test]
    fn test_region_gate_snapshot() {
        let config = test_config();
        let registry = LocaleRegistry::from_config(&config);
        let snapshot = registry.region_gate_snapshot("dev");

        assert_eq!(snapshot["region"], "CN");
        assert_eq!(snapshot["data_residency"], "local");
        assert_eq!(snapshot["i18n_enabled"], true);
        assert_eq!(snapshot["default_locale"], "zh-CN");
        assert_eq!(snapshot["generation"], 0);
    }

    #[test]
    fn test_i18n_registry_snapshot() {
        let config = test_config();
        let registry = LocaleRegistry::from_config(&config);
        let snapshot = registry.i18n_registry_snapshot("staging");

        assert_eq!(snapshot["default_locale"], "zh-CN");
        assert_eq!(snapshot["default_region"], "CN");
        assert_eq!(snapshot["env_mode"], "staging");
        assert_eq!(snapshot["generation"], 0);
        assert!(snapshot["translation_count"].as_u64().unwrap() > 0);
    }
}
