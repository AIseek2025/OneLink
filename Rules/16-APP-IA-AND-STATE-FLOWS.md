# App IA And State Flows

## 1. 目的

本文件把 `rules/15-APP-DELIVERY-PLAN.md` 进一步拆到可执行层，明确：

- App 的信息架构
- 关键页面清单
- 主链路状态流
- 页面和 BFF/后端依赖关系
- 需要优先冻结的交互和状态边界

本文件默认面向 Phase A0-A3，不覆盖桌面端。

## 2. 一级信息架构

App 第一版固定为五个一级导航：

1. `首页 / Lumi`
2. `找人`
3. `推荐`
4. `消息`
5. `我的`

设计原则：

- 用户第一次进入时先完成注册/登录/首聊，不先暴露复杂找人配置
- `首页 / Lumi` 是总入口，不让找人、推荐、私信割裂成独立冷产品
- `找人` 负责表达意图，`推荐` 负责消费结果，`消息` 负责互动和系统反馈
- `我的` 负责账号、语言、通知、隐私、帮助和数据权利入口

## 3. 页面树

## 3.1 A0-A1 必须页

- 启动页
- 登录页
- 注册页
- 会话列表页
- 聊天页
- 画像确认卡层
- 基础个人页
- 设置首页

## 3.2 A2 必须页

- 找人意图输入页
- 找人条件补充页
- 找人请求处理中页
- 推荐列表页
- 推荐详情页
- 推荐反馈弹层

## 3.3 A3 必须页

- 首条私信页
- 私信详情页
- 举报页
- 拉黑确认页
- 风险提示页
- 申诉状态页

## 3.4 A4 必须页

- 语言与地区设置页
- 通知设置页
- 隐私说明页
- 数据导出入口页
- 数据删除入口页
- 记忆纠错入口页

## 4. 主链路状态流

## 4.1 注册 / 登录流

状态：

- `idle`
- `submitting`
- `success`
- `requires_verification`
- `expired_session`
- `failed`

必须处理：

- 登录失败
- 已注册但密码错误
- 会话过期
- 重复提交
- 弱网或超时

输出：

- `auth_token`
- `refresh_policy`
- `user_profile_summary`
- `first_run_flags`

## 4.2 Lumi 首聊流

状态：

- `conversation_bootstrapping`
- `ready_for_first_message`
- `awaiting_reply`
- `reply_streaming`
- `reply_complete`
- `bridge_degraded`
- `model_fallback`
- `failed`

必须展示：

- 首包加载态
- 重试入口
- 风险提示
- 系统引导消息

输出：

- `conversation_id`
- `message_id`
- `memory_extractable_hint`
- `profile_confirmation_candidates`

## 4.3 画像确认流

状态：

- `not_ready`
- `pending_confirmation`
- `accepted`
- `rejected`
- `snoozed`
- `edited`

必须支持：

- 接受
- 拒绝
- 稍后处理
- 查看原因
- 跳转到 Web 深度编辑

输出：

- `fact_id`
- `action=accept|reject|snooze|edit`
- `feedback_payload`

## 4.4 找人流

状态：

- `draft`
- `validating`
- `submitted`
- `needs_clarification`
- `queued`
- `retrieving_candidates`
- `completed`
- `empty_result`
- `failed`

必须支持：

- 快速表达
- 条件补充
- 澄清追问
- 空结果重试

输出：

- `request_id`
- `clarification_questions`
- `query_projection`
- `safety_status`

## 4.5 推荐流

状态：

- `waiting_for_candidates`
- `candidates_ready`
- `card_exposed`
- `detail_viewed`
- `feedback_submitted`
- `connection_started`
- `dismissed`

必须支持：

- 喜欢
- 跳过
- 稍后看
- 查看解释
- 进入连接动作

输出：

- `candidate_id`
- `recommendation_id`
- `feedback_type`
- `explanation_version`

## 4.6 私信与安全流

状态：

- `dm_draft`
- `dm_under_review`
- `dm_approved`
- `dm_blocked`
- `dm_sent`
- `report_submitted`
- `block_applied`
- `appeal_pending`

必须支持：

- 首条消息审核中
- 风险被拦截时的解释
- 举报与拉黑
- 审核结果可见

输出：

- `dm_thread_id`
- `safety_decision`
- `report_id`
- `appeal_id`

## 5. 页面与后端依赖

| 页面 / 模块 | BFF 能力 | Owner service |
|-------------|----------|---------------|
| 登录 / 注册 | auth session | `identity-service` |
| Lumi 聊天 | chat conversation / reply | `ai-chat-service` + `model-gateway` |
| 画像确认 | profile facts / confirmation | `profile-service` + `context-service` |
| 找人输入 | find request submit | `bff` + `match-service` |
| 推荐列表 / 详情 | recommendations / explanation | `match-service` + `profile-service` |
| 首条私信 | dm draft / moderation | `dm-service` + `safety-service` |
| 举报 / 拉黑 | safety action / appeal | `safety-service` |
| 设置 / 语言 | preferences / locale | `profile-service` + `bff` |

## 6. 状态管理边界

App 需要至少以下状态域：

- `auth`
- `conversation`
- `profileConfirmation`
- `findRequest`
- `recommendation`
- `dm`
- `notification`
- `settings`
- `locale`

状态规则：

- 认证态必须持久化，但敏感信息不得明文写入不安全存储
- 推荐结果使用短缓存，不把推荐当永久主事实
- 画像确认和找人草稿允许本地恢复
- 多语言状态不能和地区、通知语言混用
- 所有错误态必须可追踪到请求 id / trace id

## 7. 空态 / 错误态 / 审核中态

每个主模块都必须有完整三类状态：

- 空态：第一次进入、无结果、无历史
- 错误态：网络错误、鉴权错误、后端失败、限流
- 审核中态：找人审核中、首条私信审核中、举报处理中、申诉处理中

禁止：

- 只用 toast 代替正式错误页
- 让用户停在无法恢复的 dead-end 页面
- 用纯技术错误码直接暴露给用户

## 8. App 与 Web 的分工边界

若交互满足以下条件，优先放到 Web：

- 超过 8 个字段的长表单
- 复杂批量操作
- 多表格、多筛选、多审核队列
- 大段文本比较、法务说明、导出下载

若交互满足以下条件，优先放到 App：

- 高频短操作
- 聊天和消息
- 推荐卡片消费
- 快速反馈
- 推送驱动返回

## 9. 设计评审门禁

任何新页面进入开发前，必须先明确：

- 页面目标
- 入口与出口
- 依赖哪一个 BFF 契约
- 失败时怎么退
- 埋点怎么打
- 是否涉及安全或合规文案

## 10. 交付顺序

推荐按以下顺序完成 IA 和页面落地：

1. 登录 / 注册
2. Lumi 会话列表与聊天页
3. 画像确认卡
4. 找人输入页
5. 推荐列表 / 详情页
6. 私信与安全页
7. 设置 / 语言 / 通知 / 隐私页

## 11. 本文件与其他规则的关系

- App 总规划见 `rules/15-APP-DELIVERY-PLAN.md`
- BFF 冻结范围见 `rules/17-BFF-CLIENT-CONTRACT-FREEZE.md`
- Phase 2B 闭环收口见 `rules/18-PHASE2B-CLOSURE-CHECKLIST.md`
