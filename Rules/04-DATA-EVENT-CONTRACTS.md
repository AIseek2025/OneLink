# Data Event And Contracts

## 1. 数据原则

- 原始数据、长期记忆、画像事实、推荐结果、安全处罚分层存储。
- 所有主表必须能映射到 owner service。
- 关键异步链路必须以 event_id 幂等处理。
- 长期记忆、画像投影、策略优化、runtime checkpoint 必须可回放。

## 2. 核心域

### Identity Domain

用户、身份凭证、会话、验证记录。P0 要求生产级密码哈希、会话持久化、撤销和多实例共享。

### Chat Domain

AI 对话、消息、内容、会话归档。原始聊天属于 `ai-chat-service`，不可被 `context-service` 替代主存。

### Memory Domain

`memory_artifacts`、`memory_summaries`、`memory_entities`、`memory_entity_links`、`context_logs`。owner 为 `context-service`。

### Profile Domain

用户可编辑资料、画像事实、画像版本、画像摘要、画像向量、可见性和被找设置。owner 为 `profile-service`。

### Question Domain

题库、投放、答案、完成度、质量指标。owner 为 `question-service`。

### Matching Domain

找人请求、候选、推荐卡、反馈。owner 为 `match-service`。

### Safety Domain

风险评估、举报、处罚、申诉、拉黑。owner 为 `safety-service`。

### Policy Domain

策略配置、实验、回放、shadow、canary、rollback。owner 为 optimization-layer。在线服务只读生效配置。

## 3. 事件骨干

MVP 必须优先稳定以下事件流：

| 事件 | Producer | 主要 consumers | 核心字段 | Schema |
|------|----------|-----------------|----------|--------|
| `identity.user.registered.v1` | `identity-service` | profile、analytics、safety | user_id、registered_at、region | `repo/data-platform/event-schemas/identity.user.registered.v1.json` |
| `chat.user_message.created.v1` | `ai-chat-service` | context、safety、analytics | conversation_id、message_id、user_id、created_at | `repo/data-platform/event-schemas/chat.user_message.created.v1.json` |
| `context.memory.extracted.v1` | `context-service` | profile、analytics、optimization | memory_artifact_id、user_id、network_type、source_message_ids | `repo/data-platform/event-schemas/context.memory.extracted.v1.json` |
| `context.memory.summary.updated.v1` | `context-service` | ai-chat、profile、optimization | summary_id、user_id、summary_type、policy_version | `repo/data-platform/event-schemas/context.memory.summary.updated.v1.json` |
| `profile.memory_projection.requested.v1` | `context-service` | profile | request_id、user_id、candidate_facts、source_memory_ids | `repo/data-platform/event-schemas/profile.memory_projection.requested.v1.json` |
| `question.answered.v1` | `question-service` | profile、context、analytics | answer_id、question_id、user_id、answer_payload | `repo/data-platform/event-schemas/question.answered.v1.json` |
| `match.request.submitted.v1` | `match-service` 或 BFF | match、safety、analytics | request_id、user_id、intent、constraints | `repo/data-platform/event-schemas/match.request.submitted.v1.json` |
| `dm.message.created.v1` | `dm-service` | safety、analytics、notification | thread_id、message_id、sender_id、recipient_id | `repo/data-platform/event-schemas/dm.message.created.v1.json` |
| `safety.assessment.completed.v1` | `safety-service` | match、dm、profile、analytics | assessment_id、subject_type、subject_id、risk_level、decision | `repo/data-platform/event-schemas/safety.assessment.completed.v1.json` |

事件 schema 变更必须版本化，不允许静默改 payload。

事件名、producer 和核心 payload 以 `repo/data-platform/event-schemas/` 下当前文件为准。规则层不得为冻结事件自定义别名、改写命名或制造第二套口径。

## 4. 契约优先级

工程事实源按以下顺序校验：

1. 运行中代码和测试。
2. `repo/platform/contracts/`。
3. `repo/data-platform/db-schema/` 与 event schemas。
4. 本 `rules/` 目录。
5. 归档历史资料。

`docs/archive/` 只用于历史追溯、字段来源核对与审计说明，不作为当前默认执行入口，也不得单独作为新变更依据。

若规则与工程事实冲突，必须选择一个方向修正，不能长期双源漂移。

## 5. 当前 P0 数据债

- identity、ai-chat、question 的关键状态仍偏内存态。
- 内部服务鉴权有默认开发共享口令风险。
- context-service 在无 DB 时可回退内存态，但共享环境必须使用持久化。
- contract test 与 E2E 基本缺位。

下一步开发必须优先补齐这些底座。
