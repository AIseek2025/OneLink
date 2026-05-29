# OneLink Composer 2 Chat -> Memory -> Profile 实施任务书

> 角色：`Composer 2`
> 阶段：V2 执行阶段 / 第一条真实业务纵切面
> 目标：在不改动冻结边界的前提下，打通“聊天 -> 记忆投影 -> 画像可见”的第一条完整闭环

---

## 1. 任务定位

这不是新的架构讨论，也不是新的规则重写。

你的职责只有一个：

**把已经冻结的 V2 宪法、V1 契约草案与现有 repo 骨架，落成第一条真正可本地运行、可验证、可继续扩展的业务纵切面。**

这条切片要证明的不是功能多，而是下面 4 件事：

1. `context-service` 已真实进入 AI 聊天主链路
2. 长期记忆不再由 `ai-chat-service` 本地拼接
3. `profile-service` 仍然是唯一画像写路径 owner
4. 本地开发者可以稳定复现“记忆已进入画像”的结果

---

## 2. 本任务书的文档地位

本文件是 **Composer 2 的 canonical 执行入口**，位置固定在 `Rules-V2/EXECUTION/`。

说明：

- `Rules-V2/*` 负责提供当前最高优先级世界观、边界与字段口径
- `Rules/15`、`Rules/16`、`Rules/17` 提供当前工程草案、接口和事件口径
- `Rules/20-FIRST-RUNNABLE-VERTICAL-SLICE-BRIEF.md` 继续作为**验收基线**

也就是说：

- **执行入口** 以本文件为准
- **验收标准** 以 `Rules/20` 为主，并以本文件补足当前 repo 的实现细节

---

## 3. 必须遵守的输入源优先级

按下面顺序理解并执行：

1. `Rules-V2/00-CONSTITUTION.md`
2. `Rules-V2/ARCHITECTURE/system-overview.md`
3. `Rules-V2/ARCHITECTURE/memory-layer.md`
4. `Rules-V2/ARCHITECTURE/session-layer.md`
5. `Rules-V2/DATA/data-model.md`
6. `Rules-V2/CONTRACTS/context-service-contract.md`
7. `Rules/15-MVP-OPENAPI-DRAFT.md`
8. `Rules/16-MVP-EVENT-SCHEMAS-DRAFT.md`
9. `Rules/17-MVP-SERVICE-CONTRACTS.md`
10. `Rules/20-FIRST-RUNNABLE-VERTICAL-SLICE-BRIEF.md`
11. `Rules/07-ENGINEERING-RULES.md`

如果这些文档之间有表达差异，最终口径以：

- `Rules-V2/00-CONSTITUTION.md`
- `Rules-V2/DATA/data-model.md`
- `Rules-V2/CONTRACTS/context-service-contract.md`
- `Rules/16-MVP-EVENT-SCHEMAS-DRAFT.md`
- `Rules/17-MVP-SERVICE-CONTRACTS.md`

为准。

---

## 4. 本轮已经冻结、不得重议的前提

### 4.1 核心边界

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
  - 发出 `chat.user_message.created.v1`
- `profile-service` 负责：
  - 消费 `profile.memory_projection.requested.v1`
  - 形成最小画像事实或完成度变化
  - 所有 `profile_*` 写路径

### 4.2 关键约束

- 所有长期记忆上下文必须经过 `context-service`
- 所有模型能力必须经过 `model-gateway`
- 所有画像写入必须经过 `profile-service`
- 事件名、字段名、producer 不允许自行发明新口径
- 不允许 `ai-chat-service` 或 `context-service` 直写 `profile_*`
- 不允许 `profile-service` 直接读取原始 chat 代替事件投影

### 4.3 本轮不是要做的事

这轮**不是**：

- 不是继续改架构文档
- 不是完整实现 11 个服务
- 不是接入真实 PostgreSQL / Redis / Kafka / Qdrant
- 不是做问卷、匹配、风控、私信主链路
- 不是做生产级鉴权、重试、迁移、容灾
- 不是把 AutoResearch 插进在线主链路

---

## 5. 本轮唯一必须打通的闭环

```text
register
 -> login
 -> bff chat init
 -> create/get conversation
 -> send message
 -> ai-chat-service calls context-service /internal/context/build
 -> ai-chat-service calls model-gateway chat.respond
 -> ai-chat-service returns reply
 -> ai-chat-service emits chat.user_message.created.v1
 -> context-service extracts memory
 -> context-service emits context.memory.extracted.v1
 -> context-service updates working summary
 -> context-service emits context.memory.summary.updated.v1
 -> context-service emits profile.memory_projection.requested.v1
 -> profile-service consumes projection request
 -> profile-service writes minimal profile fact / completion update
 -> GET /api/v1/profile/me or /api/v1/profile/me/completion shows result
```

这是本轮唯一必须证明的链路。

---

## 6. 本轮必须真正参与运行的服务

- `bff`
- `identity-service`
- `ai-chat-service`
- `context-service`
- `profile-service`
- `model-gateway`

以下服务可保持占位，不进入主验收：

- `api-gateway`
- `question-service`
- `dm-service`
- `match-service`
- `safety-service`

说明：

- `question-service` 在 `bff/chat/init` 中可返回空数组或占位结果
- 不得因为本轮不实现它们而破坏已有 repo 骨架

---

## 7. 当前 repo 现状与本轮必须改动的文件

### 7.1 现状判断

当前 `identity-service` 与 `profile-service` 都还是纯骨架：

- `repo/services/identity-service/src/http/routes.rs`
- `repo/services/profile-service/src/http/routes.rs`

都只暴露 `GET /api/v1/placeholder`。

对应 OpenAPI 也仍是 skeleton：

- `repo/platform/contracts/openapi/identity-service.yaml`
- `repo/platform/contracts/openapi/profile-service.yaml`

`repo/scripts/` 与 `repo/tests/integration/` 也还没有可执行验证入口：

- `repo/scripts/README.md`
- `repo/tests/integration/README.md`

### 7.2 本轮默认端口矩阵

为了保证本地 6 服务可同时运行，本轮默认端口冻结为：

- `api-gateway`: `8080`
- `identity-service`: `8081`
- `profile-service`: `8082`
- `bff`: `8083`
- `ai-chat-service`: `8085`
- `context-service`: `8089`
- `model-gateway`: `8090`

如果需要改端口，只允许通过环境变量覆盖，不允许私自改写服务边界、路由前缀或 base URL 语义。

### 7.3 你本轮必须改动的文件区域

至少覆盖：

- `repo/services/identity-service/src/http/routes.rs`
- `repo/services/profile-service/src/http/routes.rs`
- `repo/services/bff/src/http/routes.rs` 或等价 BFF 路由文件
- `repo/tests/integration/`
- `repo/scripts/`

允许按需要补充：

- `identity-service` / `profile-service` 的 `state.rs`、`repository.rs`、`models.rs` 一类最小文件
- `platform/shared-libs/` 下的 dev-only event relay 工具
- `repo/services/ai-chat-service/` 中与事件发射相关的最小必要改动
- `repo/services/context-service/` 中与事件接收、事件自消费和投影投递相关的最小必要改动

说明：

- `identity-service.yaml`、`profile-service.yaml`、`bff.yaml` 的统一同步由 `Composer 2 fast` 在主实现完成后承担
- `Composer 2` 本轮以 Rust 代码、配置和可运行链路为主，不把 OpenAPI 文件作为主交付物

---

## 8. 本轮必须实现的接口

### 8.1 identity-service

按 `Rules/15` 最小实现：

- `POST /api/v1/identity/register`
- `POST /api/v1/identity/login`
- `GET /api/v1/identity/me`

最低要求：

- 使用最小本地 session / token 方案
- 请求/响应字段名对齐 `Rules/15`
- 至少能支撑 `bff/chat/init` 与后续 profile 查询

### 8.2 profile-service

按 `Rules/15` 最小实现：

- `GET /api/v1/profile/me`
- `GET /api/v1/profile/me/completion`

可继续保持占位：

- `PATCH /api/v1/profile/me`
- discovery / follows 等其它接口

最低要求：

- `profile-service` 内部维护最小 in-memory profile store
- 能消费 `profile.memory_projection.requested.v1`
- 当目标 `user_id` 的画像尚不存在时，自动创建最小空画像，再执行本次投影
- 能把投影结果表现为：
  - 最小画像事实
  - 或 completion 增量

### 8.3 bff

按 `Rules/15` 最小实现：

- `GET /api/v1/bff/chat/init`

聚合至少包含：

- `identity.me`
- `ai-chat.getOrCreateConversation`
- `question.pending` 的空数组或占位响应

默认实现选择：

- `bff` 不自行解析 token 语义
- `bff` 透传 `Authorization` 头给 `identity-service` 的 `GET /api/v1/identity/me`
- `bff` 需要配置：
  - `identity_service_base_url`
  - `ai_chat_service_base_url`

可选：

- `GET /api/v1/bff/home`

---

## 9. 本轮必须落地的事件链路

下面 4 个事件必须真实出现在代码与日志中：

1. `chat.user_message.created.v1`
2. `context.memory.extracted.v1`
3. `context.memory.summary.updated.v1`
4. `profile.memory_projection.requested.v1`

要求：

- producer 必须与 `Rules/16` 一致
- payload 字段名必须与 `Rules/16` 一致
- 不允许把 `source_type` 改回 `source`
- 不允许把 `network_type` 改回 `memory_type` 或 `type`
- 不允许改成下划线风格事件名

本轮允许采用 dev-only 本地事件适配器，例如：

- 进程内 async event bus
- channel / relay
- 本地 dispatcher

但为了避免实现代理自行发明架构，本轮**推荐模式冻结为**：

- dev-only 事件传递采用 `HTTP envelope relay`
- 生产者异步 `POST` 标准事件 envelope 到消费者服务的内部接收端点
- 推荐接收端点统一使用：`POST /internal/events/receive`
- `ai-chat-service` 在 `send_message` 成功返回后，异步投递 `chat.user_message.created.v1` 到 `context-service`
- `context-service` 在抽取/汇总后，异步投递 `profile.memory_projection.requested.v1` 到 `profile-service`
- `context.memory.extracted.v1` 与 `context.memory.summary.updated.v1` 必须保留真实 envelope 和真实日志；允许 `context-service` 在服务内部自消费或本地 relay
- 这是 Kafka / 事件总线接入前的开发态替身；未来只替换投递层，不改事件 schema、producer、consumer 关系

但必须满足：

- 仍保留真实 envelope
- 仍保留 producer / consumer 关系
- 不允许 `ai-chat-service` 直接改 `profile-service`
- 不允许绕过 `context-service`

---

## 10. 本轮允许的简化实现

### 10.1 存储

允许先使用：

- 每服务独立 in-memory repository
- dev-only 本地内存存储

本轮不要求接入：

- PostgreSQL
- Redis
- Kafka
- Qdrant

但注意：

- 数据结构命名必须对齐 `Rules/11` / `Rules/14`
- 不能因为使用内存存储就改字段名或改 owner

### 10.2 模型

`model-gateway` 本轮允许继续返回 deterministic mock response。

但必须满足：

- `ai-chat-service` 不得绕过 `model-gateway`
- 如 `context-service` / `profile-service` 需要模型能力，也必须通过 `model-gateway`

### 10.3 记忆提取与画像投影

本轮允许使用：

- 规则提取
- 关键词提取
- 简单启发式摘要

例如用户消息：

> 我对 AI 创业很感兴趣，希望认识投资人和技术合伙人

可提取为最小结果：

- `network_type = opinion`
- `content = 对 AI 创业感兴趣`

然后投影为：

- 一条最小 profile fact
- 或 `completion_rate` 的变化

重点是**链路成立**，不是算法精度。

---

## 11. 推荐实现顺序

严格按下面顺序推进：

1. `identity-service`
2. `profile-service` 的最小 in-memory store 与读接口
3. dev-only event relay
4. `profile-service` 消费 `profile.memory_projection.requested.v1`
5. `bff` 的 `GET /api/v1/bff/chat/init`
6. integration smoke test 或本地 runner
7. README / OpenAPI / 运行说明同步

说明：

- `ai-chat-service`、`context-service`、`model-gateway` 主链已具备基础骨架，本轮只做必要对接，不重写它们
- `ai-chat-service` 增加事件发射逻辑、`context-service` 增加事件接收/处理与向 `profile-service` 的投递逻辑，都属于“必要对接”，不属于推翻已有同步主链
- 先把“画像可见”补出来，再做 BFF 聚合，否则 `Rules/20` 的验收无法闭环

---

## 12. 强制交付物

### 12.1 代码与契约

必须交付：

- `identity-service` 最小可用接口
- `profile-service` 最小可用接口
- `bff/chat/init` 最小聚合接口
- 能被 `Composer 2 fast` 直接据此同步的接口行为与字段口径

### 12.2 验证入口

至少二选一，最好同时具备：

1. 本地运行脚本
   - 例如：`repo/scripts/local/run-chat-memory-profile-slice.sh`
2. 集成 smoke test
   - 例如：`repo/tests/integration/chat_memory_profile_slice.rs`

### 12.3 验证文档

至少提供一份可读说明，例如：

- `repo/tests/integration/CHAT_MEMORY_PROFILE_SLICE.md`

内容至少包括：

- 如何启动
- 如何发送测试请求
- 如何验证“记忆已进入画像”

---

## 13. 明确不允许做的事

不要做以下事情：

1. 不重新讨论服务拆分
2. 不把多服务揉成单体
3. 不让 `ai-chat-service` 直拼长期记忆
4. 不让 `context-service` 直写画像
5. 不让 `profile-service` 直接读原始 chat 代替事件投影
6. 不改 `Rules-V2` 宪法口径
7. 不改冻结事件名、冻结字段名
8. 不把本轮目标扩张成问卷、匹配、风控或完整 infra 接入

---

## 14. 验收标准

### 14.1 用户链路验收

1. 能注册
2. 能登录
3. 能调用 `GET /api/v1/bff/chat/init`
4. 能创建或获取 AI 会话
5. 能发送一条消息
6. 能收到 AI 回复

### 14.2 架构链路验收

1. `ai-chat-service` 实际调用 `context-service /internal/context/build`
2. `ai-chat-service` 实际调用 `model-gateway`
3. `chat.user_message.created.v1` 被发出
4. `context.memory.extracted.v1` 被发出
5. `context.memory.summary.updated.v1` 被发出
6. `profile.memory_projection.requested.v1` 被发出
7. `profile-service` 实际消费并形成画像结果

### 14.3 可见结果验收

至少满足以下组合中的任意一种：

- `GET /api/v1/chat/conversations/{conversationId}/context` 能看到上下文包/摘要
- `GET /api/v1/profile/me` 能看到最小画像结果
- `GET /api/v1/profile/me/completion` 的结果因投影而变化

推荐做到前两项都可见。

### 14.4 本地运行验收

必须提供稳定复现方式，例如：

```bash
cd OneLink/repo
./scripts/local/run-chat-memory-profile-slice.sh
```

或：

```bash
cd OneLink/repo
cargo test --test chat_memory_profile_slice
```

### 14.5 后续 ASMR-Lite 扩展参考

本轮主实现**不扩张**到完整 `ASMR-Lite` 路由、benchmark 或推荐主链。

但从本轮之后，涉及记忆 / 推理 / 推荐匹配增强时，统一参考以下执行材料：

- `Rules-V2/EXECUTION/asmr-lite-engineering-implementation-brief.md`
- `Rules-V2/EXECUTION/asmr-lite-benchmark-and-acceptance-checklist.md`

要求：

- 本轮代码结构不要和这两份文档的方向相冲突
- 允许预留 `L1 / L2 / L3`、异步结构化提取、benchmark 埋点的扩展接口
- 不允许借口“为以后 ASMR-Lite 做准备”而把本轮主切片范围扩大

---

## 15. 完成后必须汇报的内容

请按以下格式汇报：

1. 本轮实际改动了哪些服务和文件
2. 哪些接口已真正可用
3. 哪些事件已真正打通
4. 本地如何运行
5. 如何验证“记忆已进入画像”
6. 哪些地方仍然是 mock / in-memory / placeholder
7. 下一步最合理的工程推进顺序

---

## 16. 一句话目标

> 这轮不是把 OneLink 做完，而是把 OneLink 最核心的在线命题再向前推进一刀：
> **AI 回复经过 Memory Compute Layer，记忆再投影进画像，最后让画像变化真正可见。**
