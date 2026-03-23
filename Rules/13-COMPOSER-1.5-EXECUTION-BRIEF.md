# OneLink Composer 1.5 执行任务书

> 角色：`Composer 1.5`
> 阶段：第二步
> 目标：基于已冻结的 OneLink MVP 总设计，输出可进入工程实现前的结构化工程草案

---

## 1. 任务目标

你现在不是继续做架构讨论，而是把已经定下来的 MVP 方案，细化成工程团队可以直接接住的第一版契约草案。

本轮必须产出四类核心交付物：

1. 建表 SQL 初稿
2. OpenAPI 接口草案
3. 事件 schema JSON 草案
4. 服务间接口契约清单

你的职责是把“规划文档”落成“工程草案”，但**不能擅自改动总设计边界**。

---

## 2. 你必须遵守的输入源优先级

以下文档是本轮唯一有效输入，按优先级从高到低使用：

1. `Rules/10-SERVICE-BOUNDARIES.md`
2. `Rules/11-DATA-EVENT-MODEL.md`
3. `Rules/02-TECH-ARCHITECTURE.md`
4. `Rules/01-PRODUCT-SYSTEM.md`
5. `Rules/03-AI-PROFILE-QUESTIONNAIRE.md`
6. `Rules/07-ENGINEERING-RULES.md`
7. `Rules/08-DELIVERY-ROADMAP.md`
8. `Rules/09-PROJECT-STRUCTURE.md`
9. `Rules/12-ARCHITECTURE-REVIEW.md`

如果文档之间存在表述差异，以：

- `10-SERVICE-BOUNDARIES.md`
- `11-DATA-EVENT-MODEL.md`

为最终口径。

**禁止**根据个人偏好重新改服务拆分、改语言选型、改 MVP 范围。

---

## 3. 当前已经冻结的 MVP 架构口径

你必须基于以下前提展开，不得重议：

### 3.1 MVP 核心在线服务

- `api-gateway`
- `bff`
- `identity-service`
- `profile-service`
- `ai-chat-service`
- `dm-service`
- `question-service`
- `match-service`
- `safety-service`
- `model-gateway`

### 3.2 MVP 核心后端语言

- 核心在线服务统一采用 `Rust`
- `Python` 只用于训练、特征工程、评估、实验
- `Go` 不是 MVP 默认主链路语言

### 3.3 已冻结的关键边界

- `bff` 作为 MVP 正式服务存在
- `ai-chat-service` 与 `dm-service` 从 MVP 就拆开
- `question-service` 进入 MVP
- `model-gateway` 从 MVP 起直接采用 `Rust`
- 所有 `profile_*` 写入统一归 `profile-service`
- `recommendation_feedbacks` 统一由 `match-service` 写
- `user_blocks` 统一由 `safety-service` 写
- `model_invocation_logs` 统一由 `model-gateway` 写
- MVP 不引入独立 `trust-service`

---

## 4. 本轮必须产出的文件

请在 `OneLink/` 工作区内直接新增以下文件：

### 4.1 SQL 草案

- `Rules/14-MVP-SQL-SCHEMA-DRAFT.md`

内容必须包含：

- 按服务分组的表清单
- 每张 MVP 必做表的第一版 DDL 草案
- 主键、唯一键、核心索引
- 必要的外键策略说明
- 哪些表需要分区或冷热分层
- 哪些高写入表先不加重外键约束

### 4.2 OpenAPI 草案

- `Rules/15-MVP-OPENAPI-DRAFT.md`

内容必须包含：

- 按服务分组的 API 草案
- 每个接口的：
  - 方法
  - 路径
  - 作用
  - 请求体
  - 响应体
  - 鉴权要求
  - 幂等要求
  - 主要错误码

### 4.3 事件 Schema 草案

- `Rules/16-MVP-EVENT-SCHEMAS-DRAFT.md`

内容必须包含：

- MVP 关键事件逐条列出
- 每个事件的 JSON schema 草案
- `partition_key`
- `idempotency_key`
- 生产者
- 消费者
- 触发时机
- 是否进入主链路

### 4.4 服务间契约清单

- `Rules/17-MVP-SERVICE-CONTRACTS.md`

内容必须包含：

- 服务间同步调用清单
- 服务间异步事件依赖清单
- 哪些接口只能内部调用
- 哪些接口是 BFF 面向前端的聚合接口
- 哪些调用必须经过 `model-gateway`

---

## 5. 你必须覆盖的 MVP 核心数据表

本轮 SQL 草案至少必须覆盖以下表。

### 5.1 账户域

- `users`
- `identity_bindings`
- `sessions`
- `verification_attempts`

### 5.2 资料与画像域

- `profiles`
- `discovery_preferences`
- `follows`
- `profile_facts`
- `profile_fact_revisions`
- `profile_traits`
- `trait_supporting_facts`
- `profile_summaries`
- `profile_embeddings`

### 5.3 AI 对话域

- `ai_conversations`
- `ai_messages`
- `ai_message_contents`

### 5.4 私信域

- `dm_threads`
- `dm_participants`
- `dm_messages`

### 5.5 问卷域

- `question_templates`
- `question_variants`
- `question_deliveries`
- `question_answers`

### 5.6 匹配域

- `find_requests`
- `recommendation_result_sets`
- `recommendation_cards`
- `recommendation_feedbacks`

### 5.7 安全治理域

- `risk_assessments`
- `report_tickets`
- `moderation_actions`
- `appeal_cases`
- `user_blocks`

### 5.8 模型平台域

- `model_invocation_logs`

**注意：**

- `trust_score_snapshots` 不属于 MVP 必做表
- 可以写成“预留 Phase 3”，但不要把它当本轮核心实现对象

---

## 6. 你必须覆盖的 MVP 核心 API

以下接口类型至少都要有草案。

### 6.1 `identity-service`

- 注册
- 登录
- 绑定登录方式
- 会话刷新
- 当前用户信息

### 6.2 `profile-service`

- 获取个人主页
- 更新个人主页
- 更新被找设置
- 获取画像完成度
- 查看关注列表
- 关注
- 取消关注

### 6.3 `ai-chat-service`

- 创建或获取 AI 会话
- 发送用户消息
- 拉取会话消息列表
- 获取当前会话上下文摘要

### 6.4 `dm-service`

- 创建私信线程
- 发送首条消息
- 获取线程列表
- 获取线程消息列表
- 标记已读

### 6.5 `question-service`

- 获取基础题包状态
- 拉取待回答问题
- 提交答案
- 获取问卷完成度

### 6.6 `match-service`

- 发起找人请求
- 获取澄清问题
- 提交澄清回答
- 获取推荐结果集
- 上报名片反馈

### 6.7 `safety-service`

- 提交投诉
- 获取投诉状态
- 获取处罚摘要
- 提交申诉

### 6.8 `bff`

至少要设计以下面向前端的聚合接口：

- 首页聚合数据
- AI 聊天页初始化
- 冷启动画像建档页
- 找人结果页
- 私信列表页
- 用户主页页

### 6.9 `model-gateway`

只定义内部接口，不定义前端接口。

至少包含能力调用契约：

- `chat.respond`
- `profile.extract_facts`
- `profile.summarize`
- `question.generate`
- `question.review`
- `safety.classify_request`
- `safety.review_message`
- `embedding.encode`

---

## 7. 你必须覆盖的 MVP 关键事件

以下事件必须进入本轮 schema 草案：

### 7.1 账户域

- `identity.user.registered.v1`
- `identity.binding.added.v1`
- `identity.user.logged_in.v1`

### 7.2 聊天域

- `chat.ai_conversation.created.v1`
- `chat.user_message.created.v1`
- `chat.ai_message.created.v1`

### 7.3 私信域

- `dm.thread.created.v1`
- `dm.message.created.v1`
- `dm.message.reviewed.v1`

### 7.4 画像域

- `profile.profile.updated.v1`
- `profile.discovery_preference.updated.v1`
- `profile.visibility_rule.updated.v1`
- `profile.fact.extracted.v1`
- `profile.fact.upserted.v1`
- `profile.fact.conflict_detected.v1`
- `profile.trait.updated.v1`
- `profile.summary.updated.v1`
- `profile.embedding.updated.v1`

### 7.5 社交域

- `social.follow.created.v1`
- `social.follow.removed.v1`

### 7.6 问题域

- `question.delivery.created.v1`
- `question.delivered.v1`
- `question.answered.v1`
- `question.skipped.v1`

### 7.7 匹配域

- `match.request.submitted.v1`
- `match.request.blocked.v1`
- `match.clarification.required.v1`
- `match.result_set.served.v1`
- `match.card.impression_logged.v1`
- `match.card.clicked.v1`
- `match.card.dm_started.v1`
- `match.card.dismissed.v1`
- `match.card.reported.v1`

### 7.8 安全治理域

- `safety.assessment.completed.v1`
- `safety.user_block.created.v1`
- `moderation.report.created.v1`
- `moderation.action.executed.v1`
- `moderation.appeal.submitted.v1`
- `moderation.appeal.resolved.v1`

### 7.9 模型平台域

- `model.invocation.completed.v1`

---

## 8. SQL 草案必须体现的关键约束

### 8.1 画像写路径

- 所有 `profile_*` 表只能由 `profile-service` 写
- `question-service` 不能绕过 `profile-service` 直写画像表
- `ai-chat-service` 不能绕过 `profile-service` 直写画像表

### 8.2 高增长表

至少对以下对象写出分区/冷热分层建议：

- `ai_messages`
- `ai_message_contents`
- `recommendation_feedbacks`
- `profile_fact_revisions`

### 8.3 反馈归并

- `recommendation_feedbacks` 必须带：
  - `source_event_id`
  - `source_event_name`

### 8.4 问卷链路

- `question_deliveries` 必须支持：
  - `onboarding_form`
  - `ai_chat`
  - `profile_completion`
- 必须支持分层必填：
  - `starter_required`
  - `profile_required`
  - `optional`

### 8.5 trust

- `trust_score_snapshots` 只写预留说明
- 本轮不要为它设计完整 MVP 主写路径

### 8.6 外键方向与命名

- `ai_messages` 与 `ai_message_contents` 之间只保留单向引用：`ai_message_contents.message_id` → `ai_messages.id`
- `ai_messages` 不包含 `content_ref_id` 字段，避免双向外键写入风险
- `question_deliveries` 和 `question_answers` 使用 `variant_id` 指向 `question_variants.id`，不使用 `question_id`
- 从 `variant_id` 可回溯到 `question_templates.id`（通过 `question_variants.template_id`）
- `question_answers` 只在用户实际提交答案后创建记录
- `question_deliveries.status` 的 `answered` 状态由 `question-service` 在收到 answer 后同步更新

---

## 9. OpenAPI 草案必须体现的关键约束

### 9.1 BFF 规则

- BFF 只做聚合，不拥有主数据
- BFF 面向前端的接口要尽量稳定、贴近页面视图
- 不要把所有内部服务接口原样透传成 BFF 接口

### 9.2 AI 对话与私信分离

- `ai-chat-service` 的接口不混入 DM 能力
- `dm-service` 的接口不混入 AI 会话能力

### 9.3 问卷系统

- 必须同时支持：
  - 结构化问题拉取
  - 答案提交
  - 完成度查询
  - AI 对话中触发的问题状态同步

### 9.4 幂等要求

以下接口必须显式说明幂等策略：

- 注册
- 绑定登录方式
- 发送用户消息
- 发送首条私信
- 提交问题答案
- 发起找人请求
- 上报名片反馈
- 提交投诉
- 提交申诉

---

## 10. 事件 schema 草案必须体现的关键约束

### 10.1 必须有的通用字段

每个事件至少包含：

- `event_id`
- `event_name`
- `event_version`
- `occurred_at`
- `producer`
- `trace_id`
- `region`
- `actor_user_id`
- `subject_id`
- `payload`

### 10.2 必须显式给出的元信息

每个事件都必须补充：

- `partition_key`
- `idempotency_key`
- `ordered_by`
- `consumers`
- `replay_required`

### 10.3 事件口径

- 默认投递语义：`at-least-once`
- 事件消费者必须可处理重复、延迟、乱序
- 不允许写成“概念性事件”，必须尽量贴近真实可落地 schema

---

## 11. 服务间契约清单必须体现的关键调用

### 11.1 同步调用

至少明确以下同步链路：

- `bff -> identity-service`
- `bff -> profile-service`
- `bff -> ai-chat-service`
- `bff -> dm-service`
- `bff -> question-service`
- `bff -> match-service`
- `bff -> safety-service`
- `ai-chat-service -> model-gateway`
- `match-service -> safety-service`
- `match-service -> model-gateway`
- `safety-service -> model-gateway`
- `ai-chat-service -> question-service`

### 11.2 异步链路

至少明确以下异步链路：

- `ai-chat-service -> profile-service`
- `question-service -> profile-service`
- `profile-service -> match-service`
- `dm-service -> match-service`
- `safety-service -> trust-service`（仅 Phase 3 预留说明）

---

## 12. 你不能做的事

### 12.1 禁止改总设计

禁止自行做以下变更：

- 删除 `bff`
- 把 `ai-chat-service` 和 `dm-service` 再合回一个服务
- 把 `question-service` 移出 MVP
- 把 `model-gateway` 改回 `Go`
- 引入独立 `trust-service` 进入 MVP
- 把 `ranking-service` 提前强行拆入 MVP

### 12.2 禁止过度发明

禁止：

- 额外增加一批未讨论的新服务
- 引入图数据库主存
- 把所有字段做成 JSON 黑盒
- 把 BFF 设计成新的领域主服务
- 跳过事件幂等和分区设计

---

## 13. 推荐执行顺序

请严格按下面顺序完成：

1. 先整理 MVP 服务 -> 表 -> 事件 -> API 的映射关系
2. 先写 `14-MVP-SQL-SCHEMA-DRAFT.md`
3. 再写 `16-MVP-EVENT-SCHEMAS-DRAFT.md`
4. 再写 `17-MVP-SERVICE-CONTRACTS.md`
5. 最后写 `15-MVP-OPENAPI-DRAFT.md`

原因：

- 表和事件先定，接口才不会漂
- 服务间契约先定，BFF 聚合接口才不会乱

---

## 14. 验收标准

完成后，必须满足以下标准：

### 14.1 完整性

- 所有 MVP 核心服务都被覆盖
- 所有 MVP 核心表都被覆盖
- 所有 MVP 关键事件都被覆盖
- 所有主链路 API 都被覆盖

### 14.2 一致性

- 服务边界与 `10-SERVICE-BOUNDARIES.md` 一致
- 数据模型与 `11-DATA-EVENT-MODEL.md` 一致
- MVP 范围与 `01`、`02`、`08` 一致

### 14.3 可执行性

- SQL 不是概念表，而是接近可建表的初稿
- OpenAPI 不是口号，而是接近真实接口定义
- 事件 schema 不是标题列表，而是有真实字段结构
- 服务契约清单能直接指导后续目录和模板生成

### 14.4 可交接性

生成的内容应当可以直接交给下一步：

- `Composer 2 Fast` 去铺目录和模板
- 或交给工程师开始做 service skeleton、migration、contract 文件

---

## 15. 最终输出口径

当你完成本轮任务时，你的结论必须明确回答：

1. MVP 第一版数据库骨架是什么
2. MVP 第一版服务 API 骨架是什么
3. MVP 第一版事件骨架是什么
4. 哪些点已冻结
5. 哪些点只是预留到 Phase 2 / Phase 3

不要再回到“是否应该这么设计”的讨论。

本轮目标不是继续做架构辩论，而是把已定架构变成工程团队的第一版可执行底稿。
