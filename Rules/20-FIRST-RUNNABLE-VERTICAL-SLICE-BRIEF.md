# OneLink First Runnable Vertical Slice 执行任务书

> 角色：`Composer 2`
> 阶段：第四步（第一条可运行闭环）
> 目标：在不改动已冻结架构边界的前提下，打通 OneLink 第一条“可本地运行、可验证、可继续扩展”的纵向闭环，证明 `context-service` 已真正进入主链路

---

## 1. 任务定位

你这一轮不是继续做架构设计，也不是补新的草案文档。

你的职责只有一个：

**把已经冻结的服务边界、接口契约、事件链路和 repo 骨架，落成第一条真正可以跑起来的 MVP 纵向切片。**

这条切片不是“全站 MVP”，而是“验证核心架构成立”的第一条闭环。

你需要证明的不是功能多，而是：

1. `context-service` 确实进入 AI 聊天主链路
2. 长期记忆不再由 `ai-chat-service` 直接拼接
3. 事件名、调用边界、画像写路径没有被绕开
4. 本地开发者能在单机环境下把这条链路跑起来并看见结果

---

## 2. 你必须遵守的输入源优先级

按下面顺序理解并执行：

1. `Rules/10-SERVICE-BOUNDARIES.md`
2. `Rules/11-DATA-EVENT-MODEL.md`
3. `Rules/19-CONTEXT-MEMORY-ARCHITECTURE.md`
4. `Rules/15-MVP-OPENAPI-DRAFT.md`
5. `Rules/16-MVP-EVENT-SCHEMAS-DRAFT.md`
6. `Rules/17-MVP-SERVICE-CONTRACTS.md`
7. `Rules/18-COMPOSER-2-FAST-EXECUTION-BRIEF.md`
8. `Rules/14-MVP-SQL-SCHEMA-DRAFT.md`
9. `Rules/02-TECH-ARCHITECTURE.md`
10. `Rules/01-PRODUCT-SYSTEM.md`
11. `Rules/09-PROJECT-STRUCTURE.md`
12. `Rules/07-ENGINEERING-RULES.md`

如果这些文档之间有表达差异，最终口径以：

- `10-SERVICE-BOUNDARIES.md`
- `11-DATA-EVENT-MODEL.md`
- `19-CONTEXT-MEMORY-ARCHITECTURE.md`

为准。

---

## 3. 本轮已经冻结、不得重议的前提

### 3.1 核心服务边界

- `context-service = Memory Compute Layer`
- `context-service` 只做：
  - memory extraction
  - memory distillation
  - context assembly
- `ai-chat-service` 负责：
  - 对话入口
  - 会话 owner
  - 调用 `context-service`
  - 调用 `model-gateway`
  - 返回 AI 回复
- `profile-service` 负责：
  - 所有 `profile_*` 写路径
  - 消费 `profile.memory_projection.requested.v1`
  - 将记忆投影进画像事实层

### 3.2 关键架构原则

- 所有长期记忆上下文必须经过 `context-service`
- 所有模型能力必须经过 `model-gateway`
- 所有画像写入必须经过 `profile-service`
- 同步负责响应，异步负责演化
- 事件名、字段名、producer 不允许自行发明新口径

### 3.3 本轮不是要做的事

这轮**不是**：

- 不是完整实现 11 个服务
- 不是完整接入 PostgreSQL / Redis / Kafka / Qdrant
- 不是做真实推荐 / 问卷 / 风控 / 私信主链路
- 不是做生产级鉴权、容灾、重试、迁移
- 不是优化模型效果

---

## 4. 本轮必须打通的最小闭环

你必须打通下面这条链路：

```text
注册
 -> 登录
 -> 进入聊天初始化页
 -> 创建/获取 AI 会话
 -> 发送一条用户消息
 -> ai-chat-service 调 context-service /internal/context/build
 -> ai-chat-service 调 model-gateway chat.respond
 -> ai-chat-service 返回 AI 回复
 -> ai-chat-service 发出 chat.user_message.created.v1
 -> context-service 消费并抽取记忆
 -> context-service 发出 context.memory.extracted.v1
 -> context-service 生成 working summary
 -> context-service 发出 context.memory.summary.updated.v1
 -> context-service 发出 profile.memory_projection.requested.v1
 -> profile-service 消费并写入画像事实
 -> 本地可验证“记忆已进入画像”
```

这是本轮唯一必须证明的链路。

---

## 5. 本轮纳入范围的服务

### 5.1 必须真正参与运行的服务

- `bff`
- `identity-service`
- `ai-chat-service`
- `context-service`
- `profile-service`
- `model-gateway`

### 5.2 可保持最小占位或不进入运行主链路的服务

- `api-gateway`
- `dm-service`
- `question-service`
- `match-service`
- `safety-service`

说明：

- 本轮验收**不要求**找人、问卷、私信、风控链路打通
- 但不得破坏它们现有的 repo 骨架

---

## 6. 本轮必须实现的接口闭环

### 6.1 身份链路

按 `Rules/15` 最小实现以下接口：

- `POST /api/v1/identity/register`
- `POST /api/v1/identity/login`
- `GET /api/v1/identity/me`

要求：

- 可以使用**最小本地会话实现**
- 可接受简单 token / session 方案
- 但接口路径、请求/响应命名必须与 `Rules/15` 保持一致

### 6.2 聊天链路

按 `Rules/15` 最小实现以下接口：

- `POST /api/v1/chat/conversations`
- `POST /api/v1/chat/conversations/{conversationId}/messages`
- `GET /api/v1/chat/conversations/{conversationId}/messages`
- `GET /api/v1/chat/conversations/{conversationId}/context`

要求：

- 发送消息时必须真实调用 `context-service`
- 发送消息时必须真实调用 `model-gateway`
- `GET /context` 必须能看见本次上下文组装结果或其最小摘要

### 6.3 BFF 聚合链路

按 `Rules/15` 最小实现以下接口：

- `GET /api/v1/bff/chat/init`

可选实现：

- `GET /api/v1/bff/home`

要求：

- `GET /api/v1/bff/chat/init` 至少聚合：
  - `identity.me`
  - `ai-chat.getOrCreateConversation`
  - `question.pending` 可先返回空结果/占位

### 6.4 context-service 内部接口

必须真实实现：

- `POST /internal/context/build`

可保持最小占位但路径存在：

- `POST /internal/memory/write`
- `GET /internal/memory/search`

要求：

- `/internal/context/build` 必须接收：
  - `user_id`
  - `conversation_id`
  - `input`
  - `task_type`
  - `max_tokens`
  - `memory_limit`
  - `summary_limit`
- 必须返回：
  - `system_prompt`
  - `user_context`
  - `memory_context`
  - `task_context`
  - `selected_summary_ids`
  - `selected_memory_ids`
  - `token_budget`

### 6.5 画像验证链路

必须至少能通过以下接口之一验证画像侧确实收到投影结果：

- `GET /api/v1/profile/me`
- `GET /api/v1/profile/me/completion`

要求：

- 不要求做完整画像展示
- 但必须能证明 profile-service 收到并落下至少一条由记忆投影得到的事实/完成度变化

---

## 7. 本轮必须落地的事件链路

以下事件名必须真实出现在代码与日志里，且 envelope 名称保持冻结口径：

1. `chat.user_message.created.v1`
2. `context.memory.extracted.v1`
3. `context.memory.summary.updated.v1`
4. `profile.memory_projection.requested.v1`

### 7.1 事件要求

- producer 必须与 `Rules/16` 一致
- payload 字段名必须与 `Rules/16` 一致
- 不允许把 `source_type` 改回 `source`
- 不允许把 `network_type` 改回 `memory_type` 或 `type`
- 不允许改事件名为下划线风格

### 7.2 本轮允许的实现策略

MVP 第一条纵切面允许使用**本地开发态事件适配器**，例如：

- 进程内异步事件总线
- 本地 channel / relay
- dev-only event dispatcher

但必须满足：

- 仍然保留真实事件 envelope
- 仍然按 producer / consumer 关系分发
- 不允许 `ai-chat-service` 直接改 `profile-service`
- 不允许绕过 `context-service`

---

## 8. 本轮允许的简化实现

为了保证这条切片能在合理时间内落地，本轮允许以下简化：

### 8.1 存储层

允许先使用：

- 每服务独立的 in-memory repository
- dev-only 本地内存存储
- 文件级简易存储（仅在绝对必要时）

不要求本轮就接入：

- PostgreSQL
- Redis
- Kafka
- Qdrant

但注意：

- 数据结构命名必须对齐 `Rules/11` / `14`
- 不能因为使用内存存储就改字段名或改 ownership

### 8.2 模型层

`model-gateway` 本轮允许返回：

- deterministic mock response
- 规则生成的占位回复
- 可重复的假 embedding / 假 summary

但必须满足：

- `ai-chat-service` 不得绕过 `model-gateway`
- `context-service` / `profile-service` 如需模型能力，也必须通过 `model-gateway`

### 8.3 记忆提取与投影

本轮允许使用：

- 规则提取
- 关键词提取
- 简单启发式摘要

例如当用户消息包含：

> “我对 AI 创业很感兴趣，希望认识投资人和技术合伙人”

可提取为最小 `memory_artifact`：

- `network_type = opinion`
- `content = 对 AI 创业感兴趣`

并进一步投影出最小画像事实。

重点是**链路成立**，不是算法精度。

---

## 9. `/context/build` 的最小实现要求

这是本轮最关键的接口。

必须体现以下行为：

1. 读取当前会话的最小 working context
2. 读取已存在的最小 memory summaries / memory artifacts
3. 进行 token budget 控制
4. 产出结构化上下文包
5. 当向量检索不可用时，明确降级为：
   - working memory
   - recent summaries
   - 跳过 persistent memory 向量召回

本轮即使没有真实 Qdrant，也必须把**降级逻辑**在代码结构上写出来。

---

## 10. `profile-service` 的最小实现要求

本轮不是做完整画像系统，但必须证明：

- `profile-service` 是画像写路径 owner
- `profile.memory_projection.requested.v1` 进入后，能形成最小画像事实结果

至少要做到：

1. 消费 `profile.memory_projection.requested.v1`
2. 根据 `memory_ids` 或投影输入形成最小事实记录
3. 通过 `GET /api/v1/profile/me` 或 `GET /api/v1/profile/me/completion` 体现结果变化

**禁止**：

- `context-service` 直接写 `profile_*`
- `ai-chat-service` 直接写 `profile_*`

---

## 11. 本轮建议的工程策略

### 11.1 代码组织

优先在现有 repo 结构中补实现，不新开平行目录。

如确实需要本地开发态公共能力，可放在：

- `repo/platform/shared-libs/`
- `repo/scripts/`
- `repo/tests/integration/`

### 11.2 强烈建议新增的最小支撑物

本轮建议新增以下内容以便验收：

1. 本地运行脚本
   - 例如：`repo/scripts/local/run-first-slice.sh`
2. 一份纵切面验证说明
   - 例如：`repo/tests/integration/FIRST_RUNNABLE_VERTICAL_SLICE.md`
3. 或一份最小自动化 smoke test
   - 例如：`repo/tests/integration/first_runnable_vertical_slice.rs`

只要验收能稳定复现，具体文件名可调整，但必须有可交付的验证入口。

### 11.3 推荐实现顺序

1. `identity-service`
2. `ai-chat-service`
3. `context-service`
4. `model-gateway`
5. `profile-service`
6. `bff`
7. 事件 relay / integration test / local runner

---

## 12. 本轮明确不允许做的事

不要做以下事情：

1. 不重新讨论服务拆分
2. 不把多服务揉成单体服务
3. 不让 `ai-chat-service` 直接拼长期记忆
4. 不让 `context-service` 直接写画像
5. 不让 `profile-service` 直接读原始 chat 代替事件投影
6. 不改 `Rules/10/11/15/16/17/19` 的已冻结口径
7. 不擅自引入与当前切片无关的大型依赖
8. 不把本轮目标扩张成“接全数据库 / 接全消息队列 / 接全向量库”

---

## 13. 验收标准

本轮完成后，必须能证明以下事项全部成立：

### 13.1 用户链路验收

1. 能注册
2. 能登录
3. 能进入聊天初始化页
4. 能创建或获取会话
5. 能发送一条消息
6. 能收到 AI 回复

### 13.2 架构链路验收

1. `ai-chat-service` 实际调用了 `context-service /internal/context/build`
2. `ai-chat-service` 实际调用了 `model-gateway`
3. `chat.user_message.created.v1` 被发出
4. `context.memory.extracted.v1` 被发出
5. `context.memory.summary.updated.v1` 被发出
6. `profile.memory_projection.requested.v1` 被发出
7. `profile-service` 实际消费并形成画像结果

### 13.3 可见结果验收

至少满足以下任一组合：

- `GET /api/v1/chat/conversations/{conversationId}/context` 能看到上下文包/摘要
- `GET /api/v1/profile/me` 能看到最小画像结果
- `GET /api/v1/profile/me/completion` 的结果因投影而变化

### 13.4 本地运行验收

必须提供一套开发者可以执行的最小验证方式，例如：

```bash
cd OneLink/repo
./scripts/local/run-first-slice.sh
```

或：

```bash
cd OneLink/repo
cargo test --test first_runnable_vertical_slice
```

二选一即可，最好同时具备。

---

## 14. 完成后必须汇报的内容

请按以下格式汇报：

1. 本轮实际改动了哪些服务和文件
2. 哪些接口已真正可用
3. 哪些事件已真正打通
4. 本地如何运行
5. 如何验证“记忆已进入画像”
6. 哪些地方仍然是 mock / in-memory / placeholder
7. 下一步最合理的工程推进顺序

---

## 15. 一句话目标

> 这轮不是把 OneLink 做完，而是把 OneLink 最核心的架构命题跑通：
> **AI 回复不是直接从聊天记录里临时拼出来，而是经过 Memory Compute Layer，再进入长期理解与画像演化。**
