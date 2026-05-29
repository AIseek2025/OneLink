# Region & Data Residency Engineering Specification

## 1. Overview

OneLink operates across multiple data regions (CN, US, EU, SEA). This document specifies the engineering architecture for region-aware data residency, isolation boundaries, and cross-region data transfer controls.

## 2. Region Model

### 2.1 Supported Regions

| Region Code | Name | Data Zone | Default Locale | Default Timezone |
|-------------|------|-----------|---------------|-----------------|
| CN | China Mainland | zone-cn | zh-CN | Asia/Shanghai |
| US | United States | zone-us | en | America/New_York |
| EU | European Union | zone-eu | en | Europe/Berlin |
| SEA | Southeast Asia | zone-sea | en | Asia/Singapore |

### 2.2 Region Assignment

- User region is determined at registration from the request IP geolocation
- Region can be explicitly set via `POST /api/v1/bff/settings/locale/update` with `region` field
- Region change triggers a data migration request (async, may take up to 30 days per GDPR Art. 20)
- During migration, `RegionDegradationMode` determines service behavior

### 2.3 Region Isolation

- Each region has its own database instance (logical isolation at minimum)
- Cross-region API calls are prohibited except through the approved cross-border gateway
- Log storage must remain in the region where the data was generated
- Model gateway calls use `locale_hint` and `region` fields to route to region-local models

## 3. Data Residency

### 3.1 Data Classification

| Category | Storage Policy | Cross-Region Transfer |
|----------|---------------|----------------------|
| User Profile | Stored in user's home region | Allowed with explicit consent |
| Chat Messages | Stored in user's home region | Prohibited (end-to-end encrypted, region-locked) |
| Safety Reports | Stored in reporter's region | Allowed for cross-region moderation |
| Analytics Events | Stored in user's region, aggregated centrally | Aggregated only (no PII) |
| AI Model Interactions | Stored in user's region | Model invocation routed to region-local endpoint |
| Compliance Records | Stored in user's region | Prohibited |

### 3.2 Cross-Border Data Transfer

- All cross-border transfers require explicit user consent via `compliance.crossborder.notice` i18n key
- Transfer audit log entry must include: source_region, target_region, data_category, user_consent_id, timestamp
- Real-time data (chat, DM) is never transferred cross-border
- Batch transfers (profile sync for moderation) are logged and auditable

### 3.3 Region Degradation Modes

When a user's home region is unavailable:

| Mode | Behavior | Use Case |
|------|----------|----------|
| `read_only` | Read from local cache, writes queued | Temporary region outage |
| `delayed_sync` | Read/write to standby, sync when primary recovers | Extended outage |
| `standby_region` | Full read/write to standby region with consent | Disaster recovery |

Degradation mode is configured per-region in the `ResidencyConfig` model (see model-gateway `region.rs`).

## 4. Compliance Controls

### 4.1 User Data Rights (per Region)

| Right | Endpoint | Implementation |
|-------|----------|---------------|
| View | `GET /api/v1/bff/compliance/summary` | Returns data inventory from home region |
| Export | `POST /api/v1/bff/compliance/export` | Generates portable data package in home region |
| Delete | `POST /api/v1/bff/compliance/delete` | Marks for deletion, 30-day grace period, cascading delete across region DB |
| Correction | `POST /api/v1/bff/compliance/correction` | Updates field in home region, propagates to dependent services |

### 4.2 Minor Protection

- Users under 18 have restricted features (controlled by `compliance.underage.warning` i18n key)
- Region-specific age thresholds: CN=18, US=13 (COPPA), EU=16 (GDPR Art. 8), SEA=varies
- Age verification is required before feature unlock

### 4.3 Consent Management

- `privacy.consent.required` i18n key displayed before any cross-border or high-risk data processing
- Consent records are immutable and stored in user's home region
- Consent withdrawal triggers data processing stop within 48 hours

## 5. I18N Key Alignment

All user-facing text must come from the i18n registry, not hard-coded. The following keys are shared between App I18nRegistry and Model-Gateway TerminologyRegistry:

| Key | Category | Owner |
|-----|----------|-------|
| safety.block.applied | Safety | safety-team |
| safety.report.confirmation | Safety | safety-team |
| safety.reject.harmful | Safety | safety-team |
| safety.appeal.submitted | Safety | safety-team |
| safety.appeal.rejected | Safety | safety-team |
| safety.appeal.approved | Safety | safety-team |
| rejection.no_match.title | Match | match-team |
| rejection.no_match.encouragement | Match | match-team |
| dm.first_message.under_review | Safety | safety-team |
| dm.first_message.blocked | Safety | safety-team |
| privacy.data_export.title | Compliance | compliance-team |
| privacy.data_delete.confirmation | Compliance | compliance-team |
| privacy.data_correction.title | Compliance | compliance-team |
| compliance.underage.warning | Compliance | compliance-team |
| compliance.crossborder.notice | Compliance | compliance-team |
| compliance.region.policy | Compliance | compliance-team |
| privacy.consent.required | Compliance | compliance-team |

## 6. Monitoring & Audit

- Data residency violations trigger P1 alerts
- Cross-border transfer audit log reviewed monthly
- Region availability monitored via `/health` + `/ready` endpoints (per service)
- I18n key coverage gaps detected by contract tests (phase6_global_i18n_compliance_contract)
