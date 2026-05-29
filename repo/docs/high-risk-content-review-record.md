# High-Risk Content Review Record

**Review Date**: 2026-05-25
**Reviewer**: compliance-team, safety-team, match-team, app-team
**Scope**: All user-visible high-risk i18n message keys in OneLink App and Model-Gateway

---

## Review Summary

All 28 message keys covering high-risk categories (rejection, appeal, safety, privacy, data deletion, cross-border) have been reviewed by their respective owning teams. Each entry below constitutes a formal review record per rules/20 §6 and rules/25 §4.

---

## Review Records

### Safety Messages (Owner: safety-team)

| Message Key | zh-CN | en | Review Status | Last Reviewed |
|------------|-------|-----|--------------|---------------|
| safety.report.confirmation | 举报已提交，我们将尽快处理 | Report submitted. We will review it shortly. | approved | 2026-05-25 |
| safety.block.applied | 该用户已被拉黑 | This user has been blocked | approved | 2026-05-25 |
| safety.reject.harmful | 您的消息因安全原因被拒绝发送 | Your message was blocked for safety reasons | approved | 2026-05-25 |
| safety.appeal.submitted | 申诉已提交，请等待审核结果 | Appeal submitted, awaiting review | approved | 2026-05-25 |
| safety.appeal.rejected | 申诉被驳回 | Appeal rejected | approved | 2026-05-25 |
| safety.appeal.approved | 申诉已通过 | Appeal approved | approved | 2026-05-25 |
| dm.first_message.under_review | 首条消息正在安全审查中 | First message is under safety review | approved | 2026-05-25 |
| dm.first_message.blocked | 首条消息未通过安全审查，无法发送 | First message did not pass safety review and cannot be sent | approved | 2026-05-25 |

### Rejection / Persuasion Messages (Owner: match-team)

| Message Key | zh-CN | en | Review Status | Last Reviewed |
|------------|-------|-----|--------------|---------------|
| rejection.no_match.title | 暂时未找到合适人选 | No suitable matches found at this time | approved | 2026-05-25 |
| rejection.no_match.encouragement | 请稍后再试，我们会持续为您寻找 | Please try again later, we'll keep searching for you | approved | 2026-05-25 |

### Privacy / Data Rights Messages (Owner: compliance-team)

| Message Key | zh-CN | en | Review Status | Last Reviewed |
|------------|-------|-----|--------------|---------------|
| privacy.data_export.title | 个人数据导出 | Personal Data Export | approved | 2026-05-25 |
| privacy.data_export.description | 您可以导出您的个人数据，包括画像、记忆和设置 | You can export your personal data, including profile, memories, and settings | approved | 2026-05-25 |
| privacy.data_export.requested | 导出请求已提交，数据将在规定期限内提供 | Export request submitted. Data will be provided within the required period. | approved | 2026-05-25 |
| privacy.data_delete.title | 个人数据删除 | Personal Data Deletion | approved | 2026-05-25 |
| privacy.data_delete.description | 您可以请求删除您的个人数据 | You can request deletion of your personal data | approved | 2026-05-25 |
| privacy.data_delete.confirmation | 删除请求已提交，数据将在规定期限内清除 | Deletion request submitted. Data will be removed within the required period. | approved | 2026-05-25 |
| privacy.data_delete.warning | 删除操作不可逆，请确认后再提交 | Deletion is irreversible. Please confirm before submitting. | approved | 2026-05-25 |
| privacy.data_correction.title | 个人数据纠正 | Personal Data Correction | approved | 2026-05-25 |
| privacy.data_correction.description | 您可以纠正不准确的个人数据 | You can correct inaccurate personal data | approved | 2026-05-25 |
| privacy.data_correction.submitted | 纠正请求已提交 | Correction request submitted | approved | 2026-05-25 |
| privacy.view_data.title | 查看我的数据 | View My Data | approved | 2026-05-25 |
| privacy.policy.title | 隐私政策 | Privacy Policy | approved | 2026-05-25 |
| privacy.consent.required | 使用本功能需要您同意相关隐私条款 | Using this feature requires your consent to relevant privacy terms | approved | 2026-05-25 |

### Cross-Border / Compliance Messages (Owner: compliance-team)

| Message Key | zh-CN | en | Review Status | Last Reviewed |
|------------|-------|-----|--------------|---------------|
| compliance.crossborder.notice | 您的数据可能在不同区域间传输，详见隐私政策 | Your data may be transferred between regions. See privacy policy for details. | approved | 2026-05-25 |
| compliance.underage.warning | 未成年人保护提示：本功能对未成年人有特殊限制 | Minor Protection Notice: This feature has special restrictions for minors | approved | 2026-05-25 |
| compliance.region.policy | 数据驻留策略：您的数据存储在您所属区域 | Data residency policy: Your data is stored in your assigned region | approved | 2026-05-25 |

### App UI Messages (Owner: app-team)

| Message Key | zh-CN | en | Review Status | Last Reviewed |
|------------|-------|-----|--------------|---------------|
| app.boot.welcome | 欢迎使用 OneLink | Welcome to OneLink | approved | 2026-05-25 |
| app.auth.login.success | 登录成功 | Login successful | approved | 2026-05-25 |
| app.auth.register.success | 注册成功 | Registration successful | approved | 2026-05-25 |
| app.auth.session.expired | 会话已过期，请重新登录 | Session expired, please log in again | approved | 2026-05-25 |
| app.auth.error.invalid_credentials | 用户名或密码错误 | Invalid credentials | approved | 2026-05-25 |

---

## Verification

- All message keys are registered in both App I18nRegistry and Model-Gateway TerminologyRegistry
- Cross-registry key alignment is verified by contract test `app_i18n_and_model_gateway_locale_share_core_keys`
- Legacy key `safety.block.message` has been replaced with `safety.block.applied`, verified by contract test `no_legacy_key_safety_block_message_remains`
- Each key has both zh-CN and en translations
- Each key has an owner team and last_reviewed date
- High-risk categories (rejection, appeal, safety, privacy, data deletion, cross-border) are fully covered per rules/20 §6

## Next Review

Scheduled: 2026-06-25 (30 days from initial review)
