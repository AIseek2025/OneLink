//! Compliance policy engine for Phase 6: Global I18N And Compliance.
//!
//! Enforces the minimum baseline from rules/13-GLOBAL-I18N-AND-COMPLIANCE.md:
//! - No unauthorized personal data collection
//! - No unauthorized real-world find-person
//! - User rights: view, export, correct, delete key memory/profile facts
//! - Safety-first for minors, harassment, scams, self-harm, illegal, privacy violation
//! - Auditable safety enforcement, appeals, blocks

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserRight {
    ViewData,
    ExportData,
    CorrectData,
    DeleteData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SafetyCategory {
    MinorProtection,
    Harassment,
    Scam,
    SelfHarm,
    Illegal,
    PrivacyViolation,
}

impl SafetyCategory {
    pub fn all() -> Vec<SafetyCategory> {
        vec![
            SafetyCategory::MinorProtection,
            SafetyCategory::Harassment,
            SafetyCategory::Scam,
            SafetyCategory::SelfHarm,
            SafetyCategory::Illegal,
            SafetyCategory::PrivacyViolation,
        ]
    }

    pub fn is_priority(&self) -> bool {
        true
    }

    pub fn label(&self) -> &str {
        match self {
            SafetyCategory::MinorProtection => "minor_protection",
            SafetyCategory::Harassment => "harassment",
            SafetyCategory::Scam => "scam",
            SafetyCategory::SelfHarm => "self_harm",
            SafetyCategory::Illegal => "illegal",
            SafetyCategory::PrivacyViolation => "privacy_violation",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComplianceAction {
    Block,
    Warn,
    RequireReview,
    AllowWithAudit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompliancePolicy {
    supported_rights: Vec<UserRight>,
    safety_categories: Vec<SafetyCategory>,
    requires_human_review_for_safety: bool,
    allows_unauthorized_find_person: bool,
    allows_unauthorized_data_collection: bool,
}

impl Default for CompliancePolicy {
    fn default() -> Self {
        Self {
            supported_rights: vec![
                UserRight::ViewData,
                UserRight::ExportData,
                UserRight::CorrectData,
                UserRight::DeleteData,
            ],
            safety_categories: SafetyCategory::all(),
            requires_human_review_for_safety: true,
            allows_unauthorized_find_person: false,
            allows_unauthorized_data_collection: false,
        }
    }
}

impl CompliancePolicy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn supported_rights(&self) -> &[UserRight] {
        &self.supported_rights
    }

    pub fn safety_categories(&self) -> &[SafetyCategory] {
        &self.safety_categories
    }

    pub fn requires_human_review_for_safety(&self) -> bool {
        self.requires_human_review_for_safety
    }

    pub fn check_find_person(&self, is_authorized: bool) -> ComplianceAction {
        if is_authorized {
            ComplianceAction::AllowWithAudit
        } else {
            ComplianceAction::Block
        }
    }

    pub fn check_data_collection(&self, is_authorized: bool) -> ComplianceAction {
        if is_authorized {
            ComplianceAction::AllowWithAudit
        } else {
            ComplianceAction::Block
        }
    }

    pub fn check_safety_category(&self, category: &SafetyCategory) -> ComplianceAction {
        if category.is_priority() {
            ComplianceAction::RequireReview
        } else {
            ComplianceAction::Warn
        }
    }

    pub fn user_can(&self, right: &UserRight) -> bool {
        self.supported_rights.contains(right)
    }

    pub fn validate_baseline(&self) -> Vec<ComplianceViolation> {
        let mut violations = Vec::new();
        if self.allows_unauthorized_find_person {
            violations.push(ComplianceViolation {
                rule: "no_unauthorized_find_person".to_string(),
                description: "Policy must not allow unauthorized real-world find-person"
                    .to_string(),
            });
        }
        if self.allows_unauthorized_data_collection {
            violations.push(ComplianceViolation {
                rule: "no_unauthorized_data_collection".to_string(),
                description: "Policy must not allow unauthorized personal data collection"
                    .to_string(),
            });
        }
        if !self.supported_rights.contains(&UserRight::ViewData) {
            violations.push(ComplianceViolation {
                rule: "user_right_view".to_string(),
                description: "Policy must support user right to view data".to_string(),
            });
        }
        if !self.supported_rights.contains(&UserRight::ExportData) {
            violations.push(ComplianceViolation {
                rule: "user_right_export".to_string(),
                description: "Policy must support user right to export data".to_string(),
            });
        }
        if !self.supported_rights.contains(&UserRight::CorrectData) {
            violations.push(ComplianceViolation {
                rule: "user_right_correct".to_string(),
                description: "Policy must support user right to correct data".to_string(),
            });
        }
        if !self.supported_rights.contains(&UserRight::DeleteData) {
            violations.push(ComplianceViolation {
                rule: "user_right_delete".to_string(),
                description: "Policy must support user right to delete data".to_string(),
            });
        }
        violations
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceViolation {
    pub rule: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub action: ComplianceAction,
    pub category: Option<SafetyCategory>,
    pub authorized: bool,
    pub details: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_has_all_user_rights() {
        let policy = CompliancePolicy::new();
        assert!(policy.user_can(&UserRight::ViewData));
        assert!(policy.user_can(&UserRight::ExportData));
        assert!(policy.user_can(&UserRight::CorrectData));
        assert!(policy.user_can(&UserRight::DeleteData));
    }

    #[test]
    fn default_policy_has_all_safety_categories() {
        let policy = CompliancePolicy::new();
        assert_eq!(policy.safety_categories().len(), 6);
    }

    #[test]
    fn default_policy_requires_human_review() {
        let policy = CompliancePolicy::new();
        assert!(policy.requires_human_review_for_safety());
    }

    #[test]
    fn unauthorized_find_person_is_blocked() {
        let policy = CompliancePolicy::new();
        assert_eq!(policy.check_find_person(false), ComplianceAction::Block);
    }

    #[test]
    fn authorized_find_person_is_audited() {
        let policy = CompliancePolicy::new();
        assert_eq!(
            policy.check_find_person(true),
            ComplianceAction::AllowWithAudit
        );
    }

    #[test]
    fn unauthorized_data_collection_is_blocked() {
        let policy = CompliancePolicy::new();
        assert_eq!(policy.check_data_collection(false), ComplianceAction::Block);
    }

    #[test]
    fn safety_category_requires_review() {
        let policy = CompliancePolicy::new();
        assert_eq!(
            policy.check_safety_category(&SafetyCategory::MinorProtection),
            ComplianceAction::RequireReview
        );
    }

    #[test]
    fn default_policy_passes_baseline() {
        let policy = CompliancePolicy::new();
        let violations = policy.validate_baseline();
        assert!(
            violations.is_empty(),
            "default policy should have no violations"
        );
    }

    #[test]
    fn safety_category_labels() {
        assert_eq!(SafetyCategory::MinorProtection.label(), "minor_protection");
        assert_eq!(SafetyCategory::Harassment.label(), "harassment");
        assert_eq!(SafetyCategory::Scam.label(), "scam");
        assert_eq!(SafetyCategory::SelfHarm.label(), "self_harm");
        assert_eq!(SafetyCategory::Illegal.label(), "illegal");
        assert_eq!(
            SafetyCategory::PrivacyViolation.label(),
            "privacy_violation"
        );
    }

    #[test]
    fn safety_category_all_count() {
        assert_eq!(SafetyCategory::all().len(), 6);
    }
}
