# Phase 10 Launch Checklist

**Per rules/25 §8**: Phase 10 (Global I18N Compliance Rollout) must satisfy all items below. Any unchecked item is a launch blocker.

---

## §2 文案资源化检查

- [x] App 用户可见文案已资源化 — I18nRegistry with 28 message keys covering all high-risk categories
- [x] BFF 未直接返回硬编码中文用户文案 — Verified: no hard-coded Chinese in BFF source code
- [x] 安全、举报、申诉、隐私、删除相关文案均使用 message key — safety.*, privacy.*, compliance.* keys registered
- [x] zh-CN 与 en 两套资源齐备 — All 28 keys have both zh-CN and en translations
- [x] App 错误响应已使用 i18n — AppErrorBody.localized_message populated via I18nRegistry lookup with Accept-Language header

## §3 语言与地区字段检查

- [x] locale 已独立建模 — SettingsDto.locale
- [x] region 已独立建模 — SettingsDto.region
- [x] timezone 已独立建模 — SettingsDto.timezone
- [x] content_language 已独立建模 — SettingsDto.content_language
- [x] notification_language 已独立建模 — SettingsDto.notification_language
- [x] App 设置页可读写上述字段 — PATCH /api/v1/bff/settings/locale/update endpoint + integration test
- [x] BFF 能正确透传 — settings_locale_update handler sends all 5 fields to BFF

## §4 高风险文案审核检查

- [x] 拒绝文案已审核 — rejection.no_match.title/encouragement reviewed by match-team
- [x] 劝导文案已审核 — rejection.no_match.encouragement reviewed by match-team
- [x] 举报文案已审核 — safety.report.confirmation reviewed by safety-team
- [x] 申诉文案已审核 — safety.appeal.submitted/rejected/approved reviewed by safety-team
- [x] 隐私文案已审核 — privacy.* keys reviewed by compliance-team
- [x] 数据导出/删除/纠正文案已审核 — privacy.data_export/delete/correction.* reviewed by compliance-team
- [x] 审核记录已形成真实交付物 — docs/high-risk-content-review-record.md

## §5 用户数据权利检查

- [x] 用户可查看关键画像与记忆事实 — GET /api/v1/bff/compliance/summary returns profile_facts, memory_summaries, key_artifacts
- [x] 用户可发起导出请求 — POST /api/v1/bff/compliance/export
- [x] 用户可发起纠正请求 — POST /api/v1/bff/compliance/correction
- [x] 用户可发起删除请求 — POST /api/v1/bff/compliance/delete
- [x] 后端有真实动作或明确处理流 — BFF endpoints proxied; mock BFF integration test covers full view→export→correct→delete chain

## §6 区域与驻留检查

- [x] 已说明用户区域判定 — docs/region-data-residency.md §2.2
- [x] 已说明数据驻留区域 — docs/region-data-residency.md §3.1
- [x] 已说明模型调用区域 — docs/region-data-residency.md §3.1 (AI Model Interactions)
- [x] 已说明日志存储区域 — docs/region-data-residency.md §3.1 (Log storage)
- [x] 已说明跨境依据 — docs/region-data-residency.md §3.2
- [x] 已说明区域故障降级方式 — docs/region-data-residency.md §3.3

## §7 工程一致性检查

- [x] App/BFF 对 locale 字段解释一致 — Contract test app_i18n_and_model_gateway_locale_share_core_keys verifies 10 core keys aligned
- [x] i18n key 体系统一 — Legacy key safety.block.message replaced with safety.block.applied; verified by contract test no_legacy_key_safety_block_message_remains
- [x] 不存在 App 和 Web 各自发明不同 message key 的情况 — Shared key alignment between App I18nRegistry and Model-Gateway TerminologyRegistry

## §8 上线阻塞项检查

- [x] 关键用户文案不再硬编码 — App error responses use I18nRegistry; verified by 4 i18n error integration tests
- [x] 用户数据权利有真实入口 — Compliance data rights e2e integration test covers view/export/correct/delete
- [x] 高风险文案已审核 — docs/high-risk-content-review-record.md
- [x] 语言/地区字段独立建模且可操作 — 5 independent fields with defaults, update endpoint, integration test
- [x] 区域和驻留有工程说明 — docs/region-data-residency.md

---

## Evidence References

| Item | Evidence Location |
|------|-------------------|
| 28 i18n message keys | repo/apps/app-server/src/i18n.rs |
| 18 model-gateway terminology keys | repo/services/model-gateway/src/locale.rs |
| i18n error response tests | repo/apps/app-server/tests/integration_test.rs (test_unauth_error_returns_i18n_*, test_validation_error_returns_i18n_*, test_dm_empty_message_error_returns_i18n_*) |
| Compliance data rights e2e test | repo/apps/app-server/tests/integration_test.rs (test_compliance_data_rights_e2e) |
| Key alignment contract test | repo/tests/contract/tests/phase6_global_i18n_compliance_contract.rs |
| Region & data residency doc | repo/docs/region-data-residency.md |
| High-risk content review record | repo/docs/high-risk-content-review-record.md |
| Launch checklist | repo/docs/phase10-launch-checklist.md |

---

**Review Date**: 2026-05-25
**Status**: all_items_checked
**Reviewer**: autopilot-iteration-87
