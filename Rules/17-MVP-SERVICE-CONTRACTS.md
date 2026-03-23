# OneLink MVP Service Contracts

> 基于 `10-SERVICE-BOUNDARIES.md` 产出
> 目标：服务间同步调用与异步事件依赖清单，明确内部/外部接口边界

---

## 1. 调用拓扑总览

```
                    ┌─────────────────┐
                    │   api-gateway    │
                    └────────┬────────┘
                             │
                    ┌────────▼────────┐
                    │      bff        │
                    └────────┬────────┘
       ┌───────┬───────┬───────┬───────┬───────┬───────┬────────┐
        │       │       │       │       │       │       │
   ┌────▼───┐ ┌─▼────┐ ┌▼────┐ ┌▼────┐ ┌▼────┐ ┌▼────┐ ┌▼──────┐
   │identity│ │profile│ │ai-  │ │ dm- │ │quest│ │match│ │safety │
   │service │ │service│ │chat │ │svc  │ │ion  │ │svc  │ │svc    │
   └────────┘ └──┬───┘ └──┬──┘ └─────┘ └──┬──┘ └──┬──┘ └──┬─────┘
                 │        │               │       │       │
                 │   ┌────▼────┐          │       │       │
                 │   │ context │          │       │       │
                 │   │ service │          │       │       │
                 │   └────┬────┘          │       │       │
                 │        │               │       │       │
                 └────────┴──────┬────────┴───────┴───────┘
                                 ▼
                           ┌─────────┐
                           │ model-  │
                           │ gateway │
                           └─────────┘
```

---

## 2. 同步调用清单

### 2.1 BFF → 领域服务（面向前端聚合）

| 调用方 | 被调方 | 接口类型 | 说明 |
|--------|--------|----------|------|
| bff | identity-service | 聚合 | 当前用户、会话刷新 |
| bff | profile-service | 聚合 | 主页、关注列表、画像完成度 |
| bff | ai-chat-service | 聚合 | 会话、消息列表、上下文摘要 |
| bff | dm-service | 聚合 | 线程列表、消息列表 |
| bff | question-service | 聚合 | 题包状态、待答问题、完成度 |
| bff | match-service | 聚合 | 找人结果、澄清问题 |
| bff | safety-service | 聚合 | 投诉状态、处罚摘要 |

### 2.2 领域服务间同步调用

| 调用方 | 被调方 | 接口 | 说明 |
|--------|--------|------|------|
| ai-chat-service | model-gateway | chat.respond | AI 对话生成 |
| ai-chat-service | context-service | POST /internal/context/build | 同步组装上下文包 |
| ai-chat-service | context-service | POST /internal/session/checkpoint | 会话结束或切换时保存 runtime checkpoint |
| ai-chat-service | question-service | 拉取待答问题 | 会话中自然追问 |
| match-service | safety-service | 风险评估 | 找人请求审查 |
| match-service | model-gateway | embedding.encode | 请求向量化 |
| match-service | model-gateway | match.explain | 推荐理由生成 |
| safety-service | model-gateway | safety.classify_request | 找人请求风险分类 |
| safety-service | model-gateway | safety.review_message | 私信内容审核 |
| dm-service | safety-service | 首条私信审核 | 陌生人私信放行前 |

### 2.3 必须经过 model-gateway 的调用

| 调用方 | 能力 | 说明 |
|--------|------|------|
| ai-chat-service | chat.respond | 对话生成 |
| context-service | embedding.encode | memory artifact 向量化 |
| context-service | profile.summarize | working memory 摘要压缩 |
| ai-chat-service | question.generate | 问题生成（Phase 2） |
| profile-service | profile.extract_facts | 画像事实抽取（消费事件后调用） |
| profile-service | embedding.encode | 向量生成 |
| profile-service | profile.summarize | 画像摘要 |
| match-service | embedding.encode | 请求/用户向量 |
| match-service | match.explain | 推荐理由 |
| safety-service | safety.classify_request | 请求风险分类 |
| safety-service | safety.review_message | 消息审核 |
| question-service | question.generate | 问题生成（Phase 2） |
| question-service | question.review | 问题审核（Phase 2） |

---

## 3. 异步事件依赖清单

### 3.1 生产者 → 消费者

| 事件 | 生产者 | 消费者 | 用途 |
|------|--------|--------|------|
| chat.user_message.created.v1 | ai-chat-service | context-service | 触发记忆抽取 |
| context.memory.extracted.v1 | context-service | context-service | 内部去噪、压缩、写入 artifacts |
| context.memory.summary.updated.v1 | context-service | ai-chat-service | 更新 working memory 摘要视图 |
| profile.memory_projection.requested.v1 | context-service | profile-service | 请求画像投影 |
| question.answered.v1 | question-service | context-service | 触发问卷记忆抽取 |
| profile.fact.upserted.v1 | profile-service | profile-service | 内部 trait/embedding 更新 |
| profile.trait.updated.v1 | profile-service | profile-service | 内部 embedding 重算 |
| profile.embedding.updated.v1 | profile-service | match-service | 向量索引更新 |
| profile.summary.updated.v1 | profile-service | match-service | 名片展示更新 |
| profile.profile.updated.v1 | profile-service | match-service | 召回索引更新 |
| profile.discovery_preference.updated.v1 | profile-service | match-service | 召回索引更新 |
| social.follow.created.v1 | profile-service | match-service | 推荐反馈归并 |
| social.follow.removed.v1 | profile-service | match-service | 推荐反馈归并 |
| dm.thread.created.v1 | dm-service | match-service | 反馈归并（source_card_id） |
| dm.message.created.v1 | dm-service | match-service | 反馈归并（dm_start/dm_reply） |
| dm.message.reviewed.v1 | safety-service | match-service | 反馈归并 |
| match.result_set.served.v1 | match-service | match-service | 反馈归并 |
| match.card.impression_logged.v1 | match-service | match-service | 反馈归并（前端经 BFF 上报，match-service 落表后发事件） |
| match.card.clicked.v1 | match-service | match-service | 反馈归并（前端经 BFF 上报，match-service 落表后发事件） |
| match.card.dm_started.v1 | dm-service | match-service | 反馈归并（dm-service 创建线程后发事件） |
| match.card.dismissed.v1 | match-service | match-service | 反馈归并（前端经 BFF 上报，match-service 落表后发事件） |
| match.card.reported.v1 | safety-service | match-service | 反馈归并（safety-service 创建工单后发事件） |
| match.request.submitted.v1 | match-service | safety-service | 风险评估 |
| safety.assessment.completed.v1 | safety-service | match-service | 找人放行 |
| safety.assessment.completed.v1 | safety-service | dm-service | 私信放行 |
| safety.user_block.created.v1 | safety-service | match-service | 召回过滤 |
| identity.user.registered.v1 | identity-service | profile-service | 创建空 profile |

### 3.2 Phase 3 预留

| 事件 | 生产者 | 消费者 | 说明 |
|------|--------|--------|------|
| safety.* / moderation.* | safety-service | trust-service | trust-service 只消费历史事件，不同步反查 |

---

## 4. 接口可见性分类

### 4.1 仅内部调用（不暴露给 BFF/前端）

| 服务 | 接口 | 调用方 |
|------|------|--------|
| context-service | POST /internal/context/build | ai-chat-service（MVP），后续可开放给 match-service |
| context-service | POST /internal/session/checkpoint | ai-chat-service（MVP），后续 agent runtime 可复用 |
| context-service | internal memory write | 仅 context-service 事件消费者 |
| context-service | POST /internal/memory/consolidate | 仅 context-service 事件消费者 / 定时任务 |
| profile-service | UpsertFact（内部） | 仅 profile-service 内部消费者 |
| profile-service | 画像事实写入 API | 无；所有写入经事件消费后内部调用 |
| match-service | 反馈归并写入 | 仅 match-service 内部消费者 |
| model-gateway | 所有能力接口 | ai-chat, profile, match, safety, question |
| safety-service | 风险评估（内部） | match-service, dm-service |

### 4.2 BFF 面向前端的聚合接口

| BFF 聚合接口 | 聚合的后端调用 | 页面 |
|--------------|----------------|------|
| GET /bff/home | identity.me, profile.summary, question.progress | 首页 |
| GET /bff/chat/init | identity.me, ai-chat.getOrCreateConversation, question.pending | AI 聊天页 |
| GET /bff/onboarding | identity.me, question.pending, question.progress | 冷启动画像建档页 |
| GET /bff/find/results | identity.me, match.getResultSet, match.getClarification | 找人结果页 |
| GET /bff/dm/list | identity.me, dm.getThreadList | 私信列表页 |
| GET /bff/profile/:userId | identity.me, profile.getProfile, profile.getFollowStatus | 用户主页页 |

### 4.3 可直接由前端调用的领域接口（经 BFF 代理）

以下接口由 BFF 代理转发，不直接暴露给前端：

- identity: register, login, refresh, bind
- profile: getProfile, updateProfile, getFollowList, follow, unfollow
- ai-chat: sendMessage, getMessages, getContextSummary
- dm: createThread, sendMessage, getThreads, getMessages, markRead
- question: getPending, submitAnswer, getProgress
- match: submitFindRequest, getClarification, submitClarification, getResultSet, reportFeedback
- safety: submitReport, getReportStatus, getModerationSummary, submitAppeal

---

## 5. 契约文件产出建议

### 5.1 同步接口契约

- 每个服务一个 OpenAPI 文件：`identity-service.yaml`, `profile-service.yaml`, ...
- model-gateway 只定义内部 gRPC/HTTP 能力契约，不对外
- BFF 定义面向前端的聚合 API：`bff-api.yaml`

### 5.2 异步事件契约

- 每个事件一个 JSON Schema：`events/identity.user.registered.v1.json`
- 或按域分组：`events/identity/*.json`, `events/chat/*.json`, ...

### 5.3 服务发现与路由

- api-gateway 路由规则：`/api/v1/*` → bff
- bff 内部调用：通过服务名解析（Kubernetes Service / Consul / 配置）

---

## 6. 关键约束摘要

| 约束 | 说明 |
|------|------|
| 上下文组装 | 所有长期记忆上下文必须经 context-service，禁止 ai-chat 直拼长期对话 |
| 记忆写路径 | 所有 memory_summaries / memory_artifacts / memory_entities / memory_entity_links / agent_runtime_checkpoints / forgetting_decisions / context_logs 写入必须经 context-service |
| 记忆整合 | consolidation pipeline 从 MVP 起必须可重放、可按 `event_id` 幂等重跑 |
| 会话运行时 | runtime checkpoint 必须版本化，包含 `schema_version`，禁止把每用户 agent 实现为常驻进程 |
| 画像写路径 | 所有 profile_* 写入必须经 profile-service，禁止 ai-chat、question 直写 |
| 推荐反馈写路径 | 所有 recommendation_feedbacks 写入必须经 match-service 消费者，禁止其他服务直写 |
| 模型调用 | 所有 AI 能力必须经 model-gateway，禁止业务服务直连外部模型 |
| trust-service | MVP 不引入；Phase 3 只消费事件，不与 safety-service 双向同步 |
| BFF 职责 | 只做聚合，不拥有主数据，不承载领域逻辑 |
