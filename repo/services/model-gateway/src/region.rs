//! Data residency and region isolation for Phase 6: Global I18N And Compliance.
//!
//! Implements:
//! - Region mapping for user location, data归属, model call, log storage
//! - Isolation strategy per region (DB, object storage, event log, vector index, model log)
//! - Cross-border transfer authorization and user consent tracking
//! - Degradation modes for region failure (read-only, partial unavailable, delayed sync, standby)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataZone {
    Database,
    ObjectStorage,
    EventLog,
    VectorIndex,
    ModelLog,
}

impl DataZone {
    pub fn all() -> Vec<DataZone> {
        vec![
            DataZone::Database,
            DataZone::ObjectStorage,
            DataZone::EventLog,
            DataZone::VectorIndex,
            DataZone::ModelLog,
        ]
    }

    pub fn label(&self) -> &str {
        match self {
            DataZone::Database => "database",
            DataZone::ObjectStorage => "object_storage",
            DataZone::EventLog => "event_log",
            DataZone::VectorIndex => "vector_index",
            DataZone::ModelLog => "model_log",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegionDegradationMode {
    ReadOnly,
    PartialUnavailable,
    DelayedSync,
    StandbyRegion,
}

impl RegionDegradationMode {
    pub fn label(&self) -> &str {
        match self {
            RegionDegradationMode::ReadOnly => "read_only",
            RegionDegradationMode::PartialUnavailable => "partial_unavailable",
            RegionDegradationMode::DelayedSync => "delayed_sync",
            RegionDegradationMode::StandbyRegion => "standby_region",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionConfig {
    region_id: String,
    data_zones: Vec<DataZone>,
    degradation_mode: RegionDegradationMode,
}

impl RegionConfig {
    pub fn new(
        region_id: &str,
        data_zones: Vec<DataZone>,
        degradation: RegionDegradationMode,
    ) -> Self {
        Self {
            region_id: region_id.to_string(),
            data_zones,
            degradation_mode: degradation,
        }
    }

    pub fn region_id(&self) -> &str {
        &self.region_id
    }

    pub fn data_zones(&self) -> &[DataZone] {
        &self.data_zones
    }

    pub fn degradation_mode(&self) -> &RegionDegradationMode {
        &self.degradation_mode
    }

    pub fn is_zone_isolated(&self, zone: &DataZone) -> bool {
        self.data_zones.contains(zone)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossBorderTransfer {
    source_region: String,
    destination_region: String,
    authorized: bool,
    consent_obtained: bool,
    legal_basis: String,
}

impl CrossBorderTransfer {
    pub fn new(
        source: &str,
        destination: &str,
        authorized: bool,
        consent_obtained: bool,
        legal_basis: &str,
    ) -> Self {
        Self {
            source_region: source.to_string(),
            destination_region: destination.to_string(),
            authorized,
            consent_obtained,
            legal_basis: legal_basis.to_string(),
        }
    }

    pub fn is_permitted(&self) -> bool {
        self.authorized && self.consent_obtained && !self.legal_basis.is_empty()
    }

    pub fn source_region(&self) -> &str {
        &self.source_region
    }

    pub fn destination_region(&self) -> &str {
        &self.destination_region
    }
}

#[derive(Debug, Clone, Default)]
pub struct RegionRegistry {
    regions: Vec<RegionConfig>,
}

impl RegionRegistry {
    pub fn new() -> Self {
        let mut registry = Self::default();
        registry.register_defaults();
        registry
    }

    fn register_defaults(&mut self) {
        self.regions.push(RegionConfig::new(
            "cn-north",
            DataZone::all(),
            RegionDegradationMode::StandbyRegion,
        ));
        self.regions.push(RegionConfig::new(
            "us-west",
            DataZone::all(),
            RegionDegradationMode::DelayedSync,
        ));
        self.regions.push(RegionConfig::new(
            "eu-west",
            vec![
                DataZone::Database,
                DataZone::ObjectStorage,
                DataZone::EventLog,
            ],
            RegionDegradationMode::ReadOnly,
        ));
    }

    pub fn find_region(&self, region_id: &str) -> Option<&RegionConfig> {
        self.regions.iter().find(|r| r.region_id == region_id)
    }

    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    pub fn check_zone_isolation(&self, region_id: &str, zone: &DataZone) -> bool {
        self.find_region(region_id)
            .map(|r| r.is_zone_isolated(zone))
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResidencyDecision {
    pub user_region: String,
    pub data_region: String,
    pub model_call_region: String,
    pub log_storage_region: String,
    pub zones_isolated: bool,
    pub transfer_required: bool,
    pub transfer_permitted: bool,
}

pub fn compute_residency(registry: &RegionRegistry, user_region: &str) -> ResidencyDecision {
    let user_rc = registry.find_region(user_region);
    let data_region = user_region;
    let model_call_region = user_region;
    let log_storage_region = user_region;

    let zones_isolated = user_rc
        .map(|rc| DataZone::all().iter().all(|z| rc.is_zone_isolated(z)))
        .unwrap_or(false);

    let transfer_required = false;
    let transfer_permitted = !transfer_required;

    ResidencyDecision {
        user_region: user_region.to_string(),
        data_region: data_region.to_string(),
        model_call_region: model_call_region.to_string(),
        log_storage_region: log_storage_region.to_string(),
        zones_isolated,
        transfer_required,
        transfer_permitted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_zone_all_has_five() {
        assert_eq!(DataZone::all().len(), 5);
    }

    #[test]
    fn data_zone_labels() {
        assert_eq!(DataZone::Database.label(), "database");
        assert_eq!(DataZone::ObjectStorage.label(), "object_storage");
        assert_eq!(DataZone::EventLog.label(), "event_log");
        assert_eq!(DataZone::VectorIndex.label(), "vector_index");
        assert_eq!(DataZone::ModelLog.label(), "model_log");
    }

    #[test]
    fn region_registry_defaults() {
        let reg = RegionRegistry::new();
        assert_eq!(reg.region_count(), 3);
        assert!(reg.find_region("cn-north").is_some());
        assert!(reg.find_region("us-west").is_some());
        assert!(reg.find_region("eu-west").is_some());
    }

    #[test]
    fn cn_north_has_all_zones() {
        let reg = RegionRegistry::new();
        let cn = reg.find_region("cn-north").unwrap();
        assert_eq!(cn.data_zones().len(), 5);
    }

    #[test]
    fn eu_west_has_partial_zones() {
        let reg = RegionRegistry::new();
        let eu = reg.find_region("eu-west").unwrap();
        assert_eq!(eu.data_zones().len(), 3);
    }

    #[test]
    fn zone_isolation_check() {
        let reg = RegionRegistry::new();
        assert!(reg.check_zone_isolation("cn-north", &DataZone::Database));
        assert!(!reg.check_zone_isolation("eu-west", &DataZone::VectorIndex));
    }

    #[test]
    fn cross_border_transfer_permitted_when_all_conditions_met() {
        let t = CrossBorderTransfer::new("cn-north", "us-west", true, true, "user_consent");
        assert!(t.is_permitted());
    }

    #[test]
    fn cross_border_transfer_not_permitted_without_consent() {
        let t = CrossBorderTransfer::new("cn-north", "us-west", true, false, "user_consent");
        assert!(!t.is_permitted());
    }

    #[test]
    fn cross_border_transfer_not_permitted_without_legal_basis() {
        let t = CrossBorderTransfer::new("cn-north", "us-west", true, true, "");
        assert!(!t.is_permitted());
    }

    #[test]
    fn compute_residency_cn_north() {
        let reg = RegionRegistry::new();
        let d = compute_residency(&reg, "cn-north");
        assert_eq!(d.user_region, "cn-north");
        assert_eq!(d.data_region, "cn-north");
        assert!(d.zones_isolated);
        assert!(!d.transfer_required);
        assert!(d.transfer_permitted);
    }

    #[test]
    fn compute_residency_unknown_region() {
        let reg = RegionRegistry::new();
        let d = compute_residency(&reg, "unknown");
        assert!(!d.zones_isolated);
    }

    #[test]
    fn degradation_mode_labels() {
        assert_eq!(RegionDegradationMode::ReadOnly.label(), "read_only");
        assert_eq!(RegionDegradationMode::DelayedSync.label(), "delayed_sync");
        assert_eq!(
            RegionDegradationMode::StandbyRegion.label(),
            "standby_region"
        );
        assert_eq!(
            RegionDegradationMode::PartialUnavailable.label(),
            "partial_unavailable"
        );
    }
}
