# Service Boundaries

## 1. 拆分原则

- 一服务一主责。
- 一类主数据一个 owner。
- 写必须通过 owner，读可以通过接口、事件投影或只读视图。
- MVP 控制在线主链路服务数量，避免为了微服务而微服务。
- 占位服务必须定义转正门槛，不能长期停留在 placeholder。

## 2. MVP 服务清单

### `api-gateway`

Rust。负责统一接入、鉴权入口、限流、路由、trace 注入、基础请求日志、Prompt Gateway 前置策略挂载。不承载领域逻辑。Prompt Gateway 若落在接入层，owner 为 `api-gateway` 或平台边车，而不是下游领域服务。

### `bff`

Rust。为 App/Web 聚合接口，做轻量视图组装和客户端友好协议。保持薄 BFF，不拥有长期状态。

### `identity-service`

Rust。拥有 users、identities、sessions、verification records。下一阶段必须从内存态和弱密码处理升级为生产级账号、密码哈希、会话持久化和撤销机制。

### `ai-chat-service`

Rust。拥有 AI conversations、AI messages、AI message contents。负责对话会话、消息写入、调用 `context-service`、调用 `model-gateway`、返回 Lumi 响应。不拥有长期记忆和画像事实。

### `context-service`

Rust。Memory Compute Layer，包含 session domain 和 memory domain。拥有 memory artifacts、summaries、entities、entity links、context logs、runtime checkpoints、forgetting decisions。只读消费 policy config，不主写 policy 表。

### `profile-service`

Rust。拥有用户主页、可见性、被找设置、关注关系、画像事实、画像摘要、画像向量和事实版本。AI、问卷和记忆只能通过投影请求或内部 upsert 能力进入画像主写路径。

### `question-service`

Rust。拥有题库、题目投放、答案、完成度和问题质量指标。负责结构化问卷与 AI 自然追问协同。

### `match-service`

Rust。拥有找人请求、推荐结果集、推荐卡、推荐反馈。负责候选召回、规则过滤、图筛、语义精筛、排序和反馈归并。

### `dm-service`

Rust。拥有用户间私信线程、参与者、消息、送达和已读状态。与 AI 聊天隔离，不承担 AI 上下文或画像抽取。

### `safety-service`

Rust。拥有风险评估、举报、处罚、申诉、拉黑。负责找人请求、私信、资料和画像回溯审查。

### `model-gateway`

Rust。统一模型路由、Prompt 版本、缓存、成本、审计、限流、熔断和降级。必须按能力域隔离连接池和预算。

## 3. 0327 吸收能力的 owner 约束

以下能力已进入现行规划，但在形成完整 contract 前只冻结 owner 候选边界，不冻结具体实现：

- Prompt Gateway：默认 owner 为 `api-gateway` 或 `model-gateway`；不得分散到各业务服务各自实现一套。
- Skill metadata / replay log：默认 owner 为 `context-service` 或后续独立 optimization/agent tooling 域；不得直接塞进 BFF 或前端本地状态。
- Graph data / graph query：默认归入 `context-service` 的 memory graph 子域或后续独立 graph capability；不得绕过 `profile-service`、`match-service` 的主写边界直改画像或推荐结果。

## 4. Go 的配合位置

Go 可用于：

- 运维控制面和内部管理 API。
- 异步批处理 worker。
- 数据同步、迁移、压测、回放工具。
- 与云厂商、消息系统、移动推送生态集成。

Go 不替代 Rust 在线核心服务主线。

## 5. 转正门槛

一个服务从 skeleton 转为正式能力，至少需要：

- 明确 owner 数据。
- OpenAPI 或 internal contract。
- 持久化或明确的无状态说明。
- 单元测试、契约测试或 smoke。
- README 更新。
- 观测指标和健康检查。
