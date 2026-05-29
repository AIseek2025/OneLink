# Chat -> Memory -> Profile 最终 Prompt 成品（纳入 ASMR-Lite 与 Benchmark）

本文件是 `0323-3 Chat-Memory-Profile开工Prompt成品.md` 的升级终版。

升级点：

- 已把新增的两份 canonical 执行材料纳入输入源：
  - `Rules-V2/EXECUTION/asmr-lite-engineering-implementation-brief.md`
  - `Rules-V2/EXECUTION/asmr-lite-benchmark-and-acceptance-checklist.md`
- 保持本轮主切片范围不变
- 明确要求主实现与配套层不要和后续 `ASMR-Lite` 路由、benchmark、埋点方向冲突

---

## 发单顺序速查

```text
1. 现在直接发 Prompt 1 给 Composer 2（Opus 预审已通过）
2. Composer 2 主实现完成后，再发 Prompt 2 给 Composer 2 fast
3. 两边完成后，用 Prompt 3 自己做跨服务收口（GPT 5.4）
4. 收口完成后，再发 Prompt 4 给 Opus 4.6 做验收
```

---

## Prompt 1. 发给 Composer 2：主实现

```text
你好，Composer 2。

你这一轮不是继续做架构设计，也不是改规则文档。你的唯一职责是：把已经冻结的 OneLink V2 边界和现有 repo 骨架，落成第一条真正可本地运行、可验证、可继续扩展的业务纵切面。

工作目录：
/Users/surferboy/.openclaw/workspace/OneLink/

你的 canonical 执行入口是：
/Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/EXECUTION/composer-2-chat-memory-profile-brief.md

你必须先完整阅读以下文件，并按这个优先级执行：
1. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/00-CONSTITUTION.md
2. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/ARCHITECTURE/system-overview.md
3. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/ARCHITECTURE/memory-layer.md
4. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/ARCHITECTURE/session-layer.md
5. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/DATA/data-model.md
6. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/CONTRACTS/context-service-contract.md
7. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/EXECUTION/composer-2-chat-memory-profile-brief.md
8. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/EXECUTION/chat-memory-profile-dispatch-sheet.md
9. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/EXECUTION/asmr-lite-engineering-implementation-brief.md
10. /Users/surferboy/.openclaw/workspace/OneLink/Rules/15-MVP-OPENAPI-DRAFT.md
11. /Users/surferboy/.openclaw/workspace/OneLink/Rules/16-MVP-EVENT-SCHEMAS-DRAFT.md
12. /Users/surferboy/.openclaw/workspace/OneLink/Rules/17-MVP-SERVICE-CONTRACTS.md
13. /Users/surferboy/.openclaw/workspace/OneLink/Rules/20-FIRST-RUNNABLE-VERTICAL-SLICE-BRIEF.md
14. /Users/surferboy/.openclaw/workspace/OneLink/Rules/07-ENGINEERING-RULES.md

本轮唯一目标链路：
register -> login -> bff chat init -> create/get conversation -> send message -> ai-chat-service calls context-service /internal/context/build -> ai-chat-service calls model-gateway -> 4 frozen events emitted -> profile-service consumes projection -> GET /api/v1/profile/me or /api/v1/profile/me/completion shows visible change

本轮默认端口矩阵（已在 config.rs 中落地，不要自行改端口）：
- api-gateway: 8080
- identity-service: 8081
- profile-service: 8082
- bff: 8083
- ai-chat-service: 8085
- context-service: 8089
- model-gateway: 8090

你必须完成：
- identity-service
  - POST /api/v1/identity/register
  - POST /api/v1/identity/login
  - GET /api/v1/identity/me
- profile-service
  - GET /api/v1/profile/me
  - GET /api/v1/profile/me/completion
  - 消费 profile.memory_projection.requested.v1
  - 当目标 user_id 的画像尚不存在时，自动创建最小空画像再执行投影
- bff
  - GET /api/v1/bff/chat/init
  - bff 不自行解析 token，透传 Authorization 头给 identity-service 的 GET /api/v1/identity/me
  - bff 的 config 里已经预埋了 identity_service_base_url 和 ai_chat_service_base_url
- dev-only event relay（推荐模式已冻结，请严格遵守）
- 让“记忆已进入画像”真实可见

dev-only 事件 relay 推荐模式（已冻结，不要自行发明其他方案）：
- 采用 HTTP envelope relay
- 生产者异步 POST 标准事件 envelope 到消费者服务的 POST /internal/events/receive
- ai-chat-service 在 send_message 成功返回后，异步投递 chat.user_message.created.v1 到 context-service
- context-service 在抽取/汇总后，异步投递 profile.memory_projection.requested.v1 到 profile-service
- context.memory.extracted.v1 与 context.memory.summary.updated.v1 必须保留真实 envelope 和真实日志；允许 context-service 内部自消费
- context-service 的 config 里已经预埋了 profile_service_base_url
- 这是 Kafka 接入前的开发态替身，未来只替换投递层

你主要改动区域应集中在：
- repo/services/identity-service/
- repo/services/profile-service/
- repo/services/bff/
- repo/services/ai-chat-service/ 中与事件发射相关的最小必要改动
- repo/services/context-service/ 中与事件接收、自消费和投影投递相关的最小必要改动
- repo/platform/shared-libs/（如确实需要 dev-only relay 工具）
- 必要的 repo/tests/integration/ 或 repo/scripts/ 支撑文件

本轮不要改 OpenAPI 文件（identity-service.yaml、profile-service.yaml、bff.yaml 由 Composer 2 fast 在你完成后统一同步）。

reqwest 已统一到 workspace 依赖，bff 和 context-service 的 Cargo.toml 里已经预埋了 reqwest = { workspace = true }，直接用即可。

ai-chat-service 增加事件发射逻辑、context-service 增加事件接收/处理与向 profile-service 的投递逻辑，都属于“必要对接”，不算重写已有同步主链。

关于新加入的 ASMR-Lite 工程实施任务书，你必须理解但不要误扩张本轮范围：
- 本轮不要求实现完整 L1 / L2 / L3
- 本轮不要求实现完整 Search Agent / Reason Agent
- 本轮不要求跑完整 benchmark
- 但你写出的代码结构、模块命名、埋点入口，不得与后续 ASMR-Lite 扩展方向冲突
- 如果需要新增最小接口或日志，请优先为未来的结构化提取、路由分层、benchmark 埋点预留位置

本轮允许：
- 每服务独立 in-memory repository
- dev-only 本地 HTTP envelope relay
- 规则提取 / 关键词提取 / 简单启发式摘要
- deterministic mock model response

本轮禁止：
- 不让 ai-chat-service 直写画像
- 不让 context-service 直写画像
- 不改冻结事件名、字段名、producer
- 不把 bff 变成业务真相 owner
- 不扩张到问卷、匹配、风控
- 不接入真实 PostgreSQL / Redis / Kafka / Qdrant
- 不改 OpenAPI 文件
- 不借口“为 ASMR-Lite 做准备”把本轮目标扩成第二条业务纵切面

4 条冻结事件必须真实出现在代码与日志里：
1. chat.user_message.created.v1
2. context.memory.extracted.v1
3. context.memory.summary.updated.v1
4. profile.memory_projection.requested.v1

工程硬要求：
- 改完后 cargo check 必须通过（至少覆盖 cargo check -p identity-service -p profile-service -p bff -p ai-chat-service -p context-service）
- 不引入新的 clippy deny 级 warning

完成后请按以下格式汇报：
1. 本轮实际改动了哪些服务和文件
2. 哪些接口已真正可用
3. 哪些事件已真正打通
4. 本地如何运行（给出启动命令和端口）
5. 如何验证“记忆已进入画像”
6. 哪些地方仍然是 mock / in-memory / placeholder
7. 哪些结构已经为后续 ASMR-Lite 扩展预留了位置
8. 下一步最合理的工程推进顺序

请直接开始实现，不要重复做计划。
```

---

## Prompt 2. 发给 Composer 2 fast：配套落地（在 Composer 2 完成后再发）

```text
你好，Composer 2 fast。

你这一轮不是主导业务逻辑实现，你的唯一职责是：为 chat -> memory projection -> profile visible 这条纵切面补齐配套层，包括契约尾差、README、脚本、测试壳和验证说明。

工作目录：
/Users/surferboy/.openclaw/workspace/OneLink/

你的 canonical 配套任务书是：
/Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/EXECUTION/composer-2-fast-chat-memory-profile-brief.md

你必须先读：
1. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/EXECUTION/composer-2-chat-memory-profile-brief.md
2. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/EXECUTION/composer-2-fast-chat-memory-profile-brief.md
3. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/EXECUTION/chat-memory-profile-dispatch-sheet.md
4. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/EXECUTION/asmr-lite-benchmark-and-acceptance-checklist.md
5. /Users/surferboy/.openclaw/workspace/OneLink/Rules/20-FIRST-RUNNABLE-VERTICAL-SLICE-BRIEF.md
6. /Users/surferboy/.openclaw/workspace/OneLink/Rules/15-MVP-OPENAPI-DRAFT.md
7. /Users/surferboy/.openclaw/workspace/OneLink/Rules/16-MVP-EVENT-SCHEMAS-DRAFT.md
8. /Users/surferboy/.openclaw/workspace/OneLink/Rules/17-MVP-SERVICE-CONTRACTS.md
9. /Users/surferboy/.openclaw/workspace/OneLink/Rules/07-ENGINEERING-RULES.md

本轮端口矩阵（写脚本和文档时要用）：
- api-gateway: 8080
- identity-service: 8081
- profile-service: 8082
- bff: 8083
- ai-chat-service: 8085
- context-service: 8089
- model-gateway: 8090

本轮 OpenAPI 文件统一由你负责同步，避免和 Composer 2 在同一批 YAML 上重复改动。如果 Composer 2 的主实现尚未稳定，优先等待或基于已完成接口结果同步，不要抢跑。

你本轮必须补齐：
- OpenAPI 契约尾差
  - repo/platform/contracts/openapi/identity-service.yaml
  - repo/platform/contracts/openapi/profile-service.yaml
  - repo/platform/contracts/openapi/bff.yaml
- 本地脚本
  - 例如 repo/scripts/local/run-chat-memory-profile-slice.sh
- 测试壳或验证文档
  - repo/tests/integration/chat_memory_profile_slice.rs
  - 或 repo/tests/integration/CHAT_MEMORY_PROFILE_SLICE.md
- README
  - repo/services/identity-service/README.md
  - repo/services/profile-service/README.md
  - repo/services/bff/README.md

你只能做配套层，不要越权：
- 不实现 identity/register/login/me 主逻辑
- 不实现 profile.memory_projection.requested.v1 的消费者逻辑
- 不实现 bff/chat/init 的主聚合逻辑
- 不新增新的 service boundary
- 不发明新的事件 schema 或 payload
- 不用脚本或文档伪装“主链已打通”

你可以做的简化：
- OpenAPI 先做到最小请求/响应示例
- 测试先做 smoke shell
- 脚本先做开发态串行版本
- README 明确标注哪些部分仍依赖 Composer 2 的主实现

关于新加入的 benchmark 清单，你需要做到：
- README、脚本、验证文档不要和 benchmark 清单口径冲突
- 如果实现里已经有埋点或日志入口，验证文档应说明如何观察
- 如果还没法跑完整 benchmark，至少说明后续 benchmark 如何接入当前脚本和验证链路

工程硬要求：
- 改完后 cargo check 必须通过（如果新增了 .rs 测试文件）
- 脚本必须可执行（chmod +x）

完成后请按以下格式汇报：
1. 同步了哪些 OpenAPI 文件
2. 新增了哪些脚本
3. 新增了哪些测试壳或验证文档
4. 更新了哪些 README
5. 哪些 benchmark / 验证入口已经预留
6. 哪些地方仍然依赖 Composer 2 的主实现完成
7. 有没有发现契约和实现之间的新冲突

请直接开始，不要扩张范围。
```

---

## Prompt 3. GPT 5.4 跨服务收口（Composer 2 和 Composer 2 fast 都完成后自己执行）

```text
你好，GPT 5.4。这是你自己的跨服务收口清单。

Composer 2 已完成主实现，Composer 2 fast 已完成配套层。你现在要做的是：修掉接口、字段、调用链、文档和实现之间的最后差异，让整条链路干干净净交给 Opus 4.6 验收。

工作目录：
/Users/surferboy/.openclaw/workspace/OneLink/

你必须先读：
1. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/00-CONSTITUTION.md
2. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/DATA/data-model.md
3. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/CONTRACTS/context-service-contract.md
4. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/EXECUTION/composer-2-chat-memory-profile-brief.md
5. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/EXECUTION/composer-2-fast-chat-memory-profile-brief.md
6. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/EXECUTION/chat-memory-profile-dispatch-sheet.md
7. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/EXECUTION/asmr-lite-engineering-implementation-brief.md
8. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/EXECUTION/asmr-lite-benchmark-and-acceptance-checklist.md
9. /Users/surferboy/.openclaw/workspace/OneLink/Rules/15-MVP-OPENAPI-DRAFT.md
10. /Users/surferboy/.openclaw/workspace/OneLink/Rules/16-MVP-EVENT-SCHEMAS-DRAFT.md
11. /Users/surferboy/.openclaw/workspace/OneLink/Rules/17-MVP-SERVICE-CONTRACTS.md
12. /Users/surferboy/.openclaw/workspace/OneLink/Rules/20-FIRST-RUNNABLE-VERTICAL-SLICE-BRIEF.md
13. /Users/surferboy/.openclaw/workspace/OneLink/Rules/07-ENGINEERING-RULES.md

本轮已冻结的端口矩阵：
- api-gateway: 8080
- identity-service: 8081
- profile-service: 8082
- bff: 8083
- ai-chat-service: 8085
- context-service: 8089
- model-gateway: 8090

你的收口检查清单：

1. 接口一致性
   - identity-service 的 register / login / me 接口路径、请求体、响应体是否与 Rules/15 一致
   - profile-service 的 /me / /me/completion 接口是否与 Rules/15 一致
   - bff 的 /chat/init 接口是否与 Rules/15 和 Rules/17 一致
   - 所有接口错误码与 HTTP 状态码是否符合 07-ENGINEERING-RULES

2. 事件一致性
   - 4 条冻结事件的 event_name、payload 字段、producer / consumer 是否与 Rules/16 一致
   - HTTP envelope relay 是否仍保留真实 producer / consumer 关系（不是 hardcode 假数据）
   - context-service 内部自消费的 context.memory.extracted.v1 和 context.memory.summary.updated.v1 是否保留了真实 envelope 与日志

3. OpenAPI / README / 测试壳一致性
   - identity-service.yaml / profile-service.yaml / bff.yaml 是否与实际实现一致
   - README 里的端口、启动命令、示例请求是否与当前实现一致
   - 测试壳或验证文档是否可以真实复现“记忆已进入画像”

4. 写路径边界
   - ai-chat-service 没有直写 profile
   - context-service 没有直写 profile
   - profile-service 没有绕过事件直接读原始 chat
   - bff 只做聚合，不拥有主数据

5. Benchmark 与扩展兼容性
   - 当前实现是否与 ASMR-Lite 工程实施任务书方向相容
   - 是否至少预留了未来 L1 / L2 / L3、结构化提取、benchmark 埋点的扩展空间
   - 配套层是否与 benchmark 清单的指标口径相冲突

6. 编译与 lint
   - cargo check -p identity-service -p profile-service -p bff -p ai-chat-service -p context-service 全部通过
   - 无新的 clippy deny 级 warning

你的收口动作：
- 如果发现不一致，直接修（最小改动原则）
- 如果 Composer 2 的核心实现方向有偏，记录但不大改，留给 Opus 判断
- 如果 Composer 2 fast 的 OpenAPI 与实现明显脱节，直接修 OpenAPI
- 如果 README 里端口写错、命令写错，直接修
- 如果测试壳引用了不存在的接口或路径，直接修或补注释

你的禁止事项：
- 不重写 Composer 2 的主实现
- 不无故扩大范围
- 不把本轮目标扩张成新业务纵切面
- 不改 Rules-V2 宪法口径
- 不改冻结事件名

完成后请按以下格式汇报：
1. 修了哪些文件、修了什么
2. 发现了哪些一致性问题（已修 / 未修需 Opus 裁决）
3. cargo check 是否通过
4. 本地运行是否可稳定复现
5. benchmark / 扩展兼容性有哪些已满足、哪些仍待下一阶段
6. 是否可以交给 Opus 4.6 验收

然后直接执行收口，不要重复做计划。
```

---

## Prompt 4. 发给 Opus 4.6：完工后验收（在 GPT 收口之后再发）

```text
你好，Opus。

请你对本轮 chat -> memory projection -> profile visible 纵切面的最终实现结果做验收审查。你不负责编码，也不重新讨论大架构，你只负责判断：这条链路是不是“真的成立”，还是“看起来能跑”的假闭环。

工作目录：
/Users/surferboy/.openclaw/workspace/OneLink/

你必须先读：
1. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/00-CONSTITUTION.md
2. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/DATA/data-model.md
3. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/CONTRACTS/context-service-contract.md
4. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/EXECUTION/composer-2-chat-memory-profile-brief.md
5. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/EXECUTION/composer-2-fast-chat-memory-profile-brief.md
6. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/EXECUTION/opus-chat-memory-profile-review-brief.md
7. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/EXECUTION/chat-memory-profile-dispatch-sheet.md
8. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/EXECUTION/asmr-lite-engineering-implementation-brief.md
9. /Users/surferboy/.openclaw/workspace/OneLink/Rules-V2/EXECUTION/asmr-lite-benchmark-and-acceptance-checklist.md
10. /Users/surferboy/.openclaw/workspace/OneLink/Rules/16-MVP-EVENT-SCHEMAS-DRAFT.md
11. /Users/surferboy/.openclaw/workspace/OneLink/Rules/17-MVP-SERVICE-CONTRACTS.md
12. /Users/surferboy/.openclaw/workspace/OneLink/Rules/20-FIRST-RUNNABLE-VERTICAL-SLICE-BRIEF.md
13. /Users/surferboy/.openclaw/workspace/OneLink/Rules/07-ENGINEERING-RULES.md

本轮已冻结的关键默认选择（审查时需要验证是否被遵守）：
- dev-only 事件 relay 采用 HTTP envelope relay，统一端点 POST /internal/events/receive
- ai-chat-service -> context-service -> profile-service 的事件投递链路
- bff 透传 Authorization 头给 identity-service，不自行解析 token
- profile-service 收到投影时若画像不存在，自动创建最小空画像
- 端口矩阵：8080/8081/8082/8083/8085/8089/8090
- OpenAPI 文件由 Composer 2 fast 统一同步
- reqwest 已统一到 workspace 依赖

你的重点审查项：
- ai-chat-service 是否越权直写画像
- context-service 是否越权承担最终画像决策
- profile-service 是否绕过事件直接读原始 chat
- 4 条冻结事件的 producer / payload / consumer 是否漂移
- bff 是否开始承载业务真相而不只是聚合
- memory_artifacts 与 profile_facts 是否混成同一对象
- event relay 是否遵守了 HTTP envelope relay 推荐模式
- completion 规则是否会导致后续难扩展
- 是否存在“日志看起来有、实际链路没打通”的假闭环
- cargo check 是否通过
- 是否存在违反 07-ENGINEERING-RULES 的工程问题
- 当前实现是否与 ASMR-Lite 工程实施任务书、benchmark 清单的方向明显冲突

你必须以 Rules/20 的三类验收作为基线：
1. 用户链路验收
2. 架构链路验收
3. 可见结果验收

尤其要验证以下 4 点是否真实成立：
- ai-chat-service 实际调用了 context-service /internal/context/build
- 4 条事件通过 HTTP envelope relay 真实打通
- profile-service 实际消费并形成画像结果
- 当前实现至少为后续 L1 / L2 / L3、结构化提取、benchmark 埋点保留了不冲突的扩展空间

请按以下格式输出：
1. Findings
按 P0 / P1 / P2 排序列问题。
每条必须写：
- 文件位置
- 问题是什么
- 为什么会导致返工或污染边界
- 建议怎么修

2. Residual Risks
即使没有阻塞问题，也要指出还剩哪些 mock / in-memory / placeholder 风险。

3. Acceptance Verdict
最后只能给出以下结论之一：
- 通过，可进入下一轮
- 条件通过，需要先补若干问题
- 不通过，必须返工

请直接开始验收。
```

---

## 这版相对 `0323-3` 的新增点

| 位置 | 新增内容 |
|---|---|
| Prompt 1 | 阅读清单补入 `asmr-lite-engineering-implementation-brief.md`，并明确“本轮不实现完整 ASMR-Lite，但代码结构不能与后续方向冲突” |
| Prompt 2 | 阅读清单补入 `asmr-lite-benchmark-and-acceptance-checklist.md`，并要求 README / 脚本 / 验证文档为 benchmark 留入口 |
| Prompt 3 | 收口项增加 benchmark 与扩展兼容性检查 |
| Prompt 4 | 验收项增加“实现是否与 ASMR-Lite 工程实施任务书、benchmark 清单方向冲突”的审查 |

