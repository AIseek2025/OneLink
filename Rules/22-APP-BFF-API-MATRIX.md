# App BFF API Matrix

## 1. 目的

本文件把 App 页面和 BFF 接口做成映射矩阵，便于：

- App 开发排期
- BFF 契约冻结
- 后端 owner 对齐
- 测试与 mock 覆盖

## 2. 使用规则

- 一页可依赖多个接口
- 一个接口可服务多个页面
- 页面开工前至少要有 mock 契约
- 若 BFF 未冻结，不允许页面进入正式联调

## 3. 矩阵

| 页面 | 核心动作 | BFF 接口主题 | Owner service | 优先级 |
|------|----------|---------------|---------------|--------|
| 启动页 | 恢复会话 | session refresh / me summary | `identity-service` | A0 |
| 登录页 | 登录 | auth login | `identity-service` | A1 |
| 注册页 | 注册 | auth register | `identity-service` | A1 |
| 会话列表页 | 取最近会话 | conversation list | `ai-chat-service` | A1 |
| 聊天页 | 发消息 / 收回复 | chat send / conversation fetch | `ai-chat-service` + `model-gateway` | A1 |
| 画像确认卡 | 获取待确认事实 / 提交动作 | profile confirmation list / confirm action | `profile-service` + `context-service` | A1 |
| 设置首页 | 获取设置摘要 | settings summary | `profile-service` + `bff` | A1 |
| 找人输入页 | 提交请求 | find request create | `match-service` + `bff` | A2 |
| 条件补充页 | 提交澄清答案 | clarification answer submit | `match-service` | A2 |
| 推荐列表页 | 获取结果列表 | recommendation list | `match-service` | A2 |
| 推荐详情页 | 获取推荐详情和解释 | recommendation detail / explanation | `match-service` + `profile-service` | A2 |
| 推荐反馈弹层 | 提交反馈 | recommendation feedback | `match-service` | A2 |
| 首条私信页 | 创建草稿 / 提交首条消息 | dm draft / dm submit | `dm-service` + `safety-service` | A3 |
| 私信详情页 | 获取线程 / 发后续消息 | dm thread fetch / message send | `dm-service` | A3 |
| 举报页 | 提交举报 | report submit | `safety-service` | A3 |
| 拉黑页 | 执行拉黑 | block apply | `safety-service` | A3 |
| 申诉状态页 | 查询申诉状态 | appeal status | `safety-service` | A3 |
| 语言与地区页 | 保存 locale/region 等 | settings update / locale registry | `profile-service` + `bff` | A4 |
| 数据权利入口页 | 导出/删除/纠正 | compliance actions | `profile-service` + `context-service` + `bff` | A4 |

## 4. A0-A1 必须冻结接口

- `POST /auth/register`
- `POST /auth/login`
- `POST /auth/session/refresh`
- `GET /me/summary`
- `GET /chat/conversations`
- `POST /chat/messages`
- `GET /chat/conversations/{id}`
- `GET /profile/confirmations`
- `POST /profile/confirmations/{id}/actions`
- `GET /settings/summary`

## 5. A2 必须冻结接口

- `POST /find/requests`
- `GET /find/requests/{id}`
- `POST /find/requests/{id}/clarifications`
- `GET /recommendations`
- `GET /recommendations/{id}`
- `POST /recommendations/{id}/feedback`

## 6. A3 必须冻结接口

- `POST /dm/threads/draft`
- `POST /dm/threads/first-message`
- `GET /dm/threads/{id}`
- `POST /safety/reports`
- `POST /safety/blocks`
- `GET /safety/appeals/{id}`

## 7. A4 必须冻结接口

- `GET /settings/locale`
- `PATCH /settings/locale`
- `GET /compliance/summary`
- `POST /compliance/export`
- `POST /compliance/delete`
- `POST /compliance/correction`

## 8. 每个接口必须同步的资产

- OpenAPI
- mock response
- DTO / type
- error code map
- analytics event map
- page-state 映射说明

## 9. 埋点事件映射（analytics event map）

每个 BFF 接口对应的埋点事件如下，与 `repo/apps/mobile/src/services/tracking.ts` 和 `analytics.ts` 保持同步：

| BFF 接口 | 页面触发时机 | 埋点 event_name | 补充字段 |
|----------|-------------|-----------------|----------|
| `POST /auth/register` | 注册提交 | `registration.started` | `provider` |
| `POST /auth/register` | 注册成功 | `registration.completed` | `user_id`, `provider` |
| `POST /auth/register` | 注册失败 | `error.occurred` | `error_type=validation`, `error_message` |
| `POST /auth/login` | 登录提交 | `login.started` | `provider` |
| `POST /auth/login` | 登录成功 | `login.completed` | `user_id`, `provider` |
| `POST /auth/login` | 登录失败 | `error.occurred` | `error_type=auth`, `error_message` |
| `POST /auth/session/refresh` | 启动恢复会话 | `page.view` | `page_name=Splash`→`Home` |
| `GET /me/summary` | 启动完成 | `page.view` | `page_name=Home` |
| `GET /chat/conversations` | 会话列表曝光 | `page.view` | `page_name=Home` |
| `POST /chat/messages` | 发送消息 | `chat.message.sent` | `user_id`, `conversation_id`, `content_type` |
| `GET /chat/conversations/{id}` | 收到回复 | `chat.message.received` | `user_id`, `conversation_id` |
| `GET /profile/confirmations` | 画像确认卡曝光 | `profile.confirmation.viewed` | `user_id`, `completion_rate`, `missing_dimensions` |
| `POST /profile/confirmations/{id}/actions` | 确认事实 | `profile.fact.confirmed` | `user_id`, `fact_type`, `fact_value` |
| `POST /profile/confirmations/{id}/actions` | 忽略事实 | `profile.fact.dismissed` | `user_id`, `fact_type`, `fact_value` |
| `GET /settings/summary` | 设置页曝光 | `page.view` | `page_name=Me` |
| `POST /find/requests` | 找人请求提交 | `find.intent.submitted` | `user_id`, `query`, `query_length` |
| `POST /find/requests/{id}/clarifications` | 条件补充提交 | `find.intent.submitted` | `user_id`, `query`, `query_length` |
| `GET /recommendations` | 推荐列表曝光 | `page.view` | `page_name=Recommendations` |
| `GET /recommendations` | 推荐卡片曝光 | `recommendation.exposed` | `user_id`, `result_set_id`, `candidate_count`, `position` |
| `GET /recommendations/{id}` | 推荐详情 | `recommendation.detail.viewed` | `recommendation_id` |
| `POST /recommendations/{id}/feedback` | 推荐反馈 | `recommendation.feedback.submitted` | `recommendation_id`, `feedback_type` |
| `POST /dm/threads/first-message` | 首条私信提交 | `dm.first_message.sent` | `recommendation_id` |
| `POST /dm/threads/first-message` | 首条私信通过安全审查 | `dm.message.sent` | `user_id`, `thread_id`, `recipient_user_id` |
| `POST /dm/threads/first-message` | 首条私信被安全拦截 | `safety.report.submitted` | — |
| `GET /dm/threads/{id}` | 私信线程查看 | `page.view` | `page_name=DM` |
| `POST /safety/reports` | 举报提交 | `report.submitted` | `user_id`, `target_type`, `target_id`, `reason` |
| `POST /safety/blocks` | 拉黑 | `safety.block.submitted` | — |
| `PATCH /settings/locale` | 语言/地区保存 | `settings.saved` | — |
| `POST /compliance/export` | 数据导出请求 | `compliance.export.completed` | — |
| `POST /compliance/delete` | 数据删除请求 | `compliance.deletion.submitted` | — |
| `POST /compliance/correction` | 数据纠正请求 | `compliance.correction.submitted` | — |

埋点字段冻结状态：**已冻结（自 iteration 138 起正式声明）**。

### 冻结范围与交叉引用

1. **BFF OpenAPI 冻结**：`repo/platform/contracts/openapi/bff.yaml` v1.7.0 已将 `AnalyticsEvent.event_name` 枚举写入 schema（45 个值 A0-A4），变更需版本升级。
2. **Mobile App 冻结**：`repo/apps/mobile/src/services/analytics.ts` 的 `AnalyticsEvent` discriminated union 与 `repo/apps/mobile/src/services/tracking.ts` 的 `ScreenSpecEventName` → `SCREEN_SPEC_TO_ANALYTICS` 映射，与 BFF OpenAPI `event_name` enum 一一对应。
3. **Web App 冻结**：`repo/apps/web/src/analytics/events.ts` 的 `AnalyticsEvent` discriminated union 与 Mobile 端完全对齐（`platform` 字段额外含 `'web'`），与 BFF OpenAPI `event_name` enum 一一对应。
4. **本矩阵表**：Section 9 的 event_name / 补充字段列表为 App/Web 双端埋点唯一真相源。

### 冻结规则

- 新增埋点事件必须先在此表（Section 9）登记，同步到 `repo/platform/contracts/openapi/bff.yaml` 的 `AnalyticsEvent.event_name` enum，并同步到 `repo/apps/mobile/src/services/analytics.ts` + `repo/apps/mobile/src/services/tracking.ts` + `repo/apps/web/src/analytics/events.ts`。
- 已冻结事件的 `event_name` 和补充字段不可删除或重命名，只允许新增可选字段。
- 双端 `AnalyticsEvent` 类型定义必须与 BFF OpenAPI `AnalyticsEvent` schema 保持一致。

## 10. 验收规则

矩阵完整可用的判定标准：

- 所有 A1 页面都能映射到明确的 BFF 接口
- 所有 A2-A4 页面都能指向 owner service
- 没有页面依赖内部服务直连
- mock、OpenAPI、页面规格三者一致

## 11. 与其他规则的关系

- 页面规格见 `rules/21-APP-SCREEN-SPECS.md`
- App 总规划见 `rules/15-APP-DELIVERY-PLAN.md`
- 契约冻结规则见 `rules/17-BFF-CLIENT-CONTRACT-FREEZE.md`
