# OneLink Composer 2 Fast 执行任务书

> 角色：`Composer 2 Fast`
> 阶段：第三步
> 目标：基于已冻结的 OneLink MVP 总设计与已产出的 14~17 工程草案，批量铺设实际目录树、样板文件和契约骨架

---

## 1. 任务定位

你这一轮不是继续做架构讨论，也不是补做 SQL / OpenAPI / 事件设计。

你的职责只有一个：

**把已经定好的架构和工程草案，落成可交给工程师继续开发的“仓库骨架”。**

你要做的是：

1. 建立实际目录结构
2. 建立 README 和约定说明
3. 建立服务模板和契约文件骨架
4. 建立事件 schema 文件骨架
5. 建立 migration / config / tests / infra 的空壳目录与样板

你**不要**做的是：

1. 不重做架构设计
2. 不补充新的服务拆分
3. 不把草案直接实现成完整业务代码
4. 不把数据库 migration 一次性写完
5. 不擅自安装依赖并生成大量不可控文件

---

## 2. 你必须遵守的输入源优先级

按下面顺序理解并执行：

1. `Rules/10-SERVICE-BOUNDARIES.md`
2. `Rules/11-DATA-EVENT-MODEL.md`
3. `Rules/19-CONTEXT-MEMORY-ARCHITECTURE.md`
4. `Rules/14-MVP-SQL-SCHEMA-DRAFT.md`
5. `Rules/16-MVP-EVENT-SCHEMAS-DRAFT.md`
6. `Rules/17-MVP-SERVICE-CONTRACTS.md`
7. `Rules/15-MVP-OPENAPI-DRAFT.md`
8. `Rules/09-PROJECT-STRUCTURE.md`
9. `Rules/07-ENGINEERING-RULES.md`
10. `Rules/13-COMPOSER-1.5-EXECUTION-BRIEF.md`

如果这些文档之间有表达差异，最终口径以：

- `10-SERVICE-BOUNDARIES.md`
- `11-DATA-EVENT-MODEL.md`

为准。

---

## 3. 当前冻结的 MVP 口径

### 3.1 MVP 核心在线服务

- `api-gateway`
- `bff`
- `identity-service`
- `profile-service`
- `context-service`
- `ai-chat-service`
- `dm-service`
- `question-service`
- `match-service`
- `safety-service`
- `model-gateway`

### 3.2 MVP 核心后端语言

- 核心在线服务统一采用 `Rust`
- `Python` 只用于 AI 平台和离线智能
- `Go` 不是 MVP 默认主链路语言

### 3.3 MVP Rust 技术基线（必须统一）

所有 MVP 在线 Rust 服务必须使用以下统一基线，不得自行选型：

| 层 | crate | 用途 |
|----|-------|------|
| 异步运行时 | `tokio` | 全部服务统一异步运行时 |
| HTTP 框架 | `axum` | 全部 HTTP 服务统一框架 |
| 日志与追踪 | `tracing` + `tracing-subscriber` | 结构化日志、分布式追踪 |
| 序列化 | `serde` + `serde_json` | JSON 序列化/反序列化 |

**约束说明：**

- 这 4 个 crate 是"不需要讨论"级别的选择，Composer 2 在模板中必须统一使用
- `axum` 约束 MVP 在线 HTTP 服务；未来纯消费者/批处理/离线 worker 不一定需要 `axum`，但 `tokio + tracing + serde` 仍需保持一致
- 除此之外，骨架模板不引入其他依赖（不接数据库、不接 Kafka、不接外部模型）
- 根 `Cargo.toml` workspace 中统一管理这些依赖的版本，各服务 `Cargo.toml` 使用 `workspace = true` 继承

### 3.4 已冻结的关键边界

- `bff` 是 MVP 正式服务
- `ai-chat-service` 与 `dm-service` 从第一天起分开
- `question-service` 进入 MVP
- `context-service` 作为 MVP 核心服务进入主链路
- `model-gateway` 为 Rust
- 所有 `profile_*` 写入统一归 `profile-service`
- `recommendation_feedbacks` 统一由 `match-service` 写
- 所有 AI 能力必须经过 `model-gateway`
- 所有长期记忆上下文必须经过 `context-service`
- MVP 不引入独立 `trust-service`
- 每个事件只有唯一一个生产者（不允许"A 或 B"）

---

## 4. 你这轮必须直接产出的实际内容

### 4.1 目录树

**重要**：代码仓库根目录为 `OneLink/repo/`，不是 `OneLink/`。`OneLink/Rules/` 和 `OneLink/docs/` 是规划文档区，仍在高频修改中，不与代码仓库混在一起。

在 `OneLink/repo/` 下实际建立以下 MVP 最小子集目录：

```text
OneLink/
├── Rules/          ← 已冻结的规划文档（不动）
├── docs/           ← 早期产品文档（不动）
└── repo/           ← 代码仓库根
    ├── apps/
    │   ├── web/
    │   └── admin/
    ├── services/
    │   ├── api-gateway/
    │   ├── bff/
    │   ├── identity-service/
    │   ├── profile-service/
    │   ├── context-service/
    │   ├── ai-chat-service/
    │   ├── dm-service/
    │   ├── question-service/
    │   ├── match-service/
    │   ├── safety-service/
    │   └── model-gateway/
    ├── platform/
    │   ├── contracts/
    │   ├── shared-libs/
    │   └── observability/
    ├── data-platform/
    │   ├── db-schema/
    │   └── event-schemas/
    ├── ai-platform/
    │   ├── prompts/
    │   ├── evaluators/
    │   └── inference/
    ├── infra/
    ├── tests/
    └── scripts/
```

### 4.2 根级样板文件

在 `OneLink/repo/` 下至少创建：

- `README.md`
- `Cargo.toml`（workspace，统一管理 `tokio`、`axum`、`tracing`、`tracing-subscriber`、`serde`、`serde_json` 版本）
- `.gitignore`
- `.editorconfig`
- `Makefile` 或等价的最小开发命令入口

### 4.3 每个 Rust 服务的模板文件

每个 MVP Rust 服务至少创建：

- `Cargo.toml`
- `README.md`
- `src/main.rs`
- `src/lib.rs`
- `src/config.rs`
- `src/http/mod.rs`
- `src/http/routes.rs`
- `src/health.rs`
- `src/errors.rs`

如需进一步分层，可增加：

- `src/application/`
- `src/domain/`
- `src/infrastructure/`

但**不要**在这一轮写入大量业务实现。

### 4.4 契约文件骨架

需要把第二步草案落成实际文件骨架（均在 `repo/` 下）：

- `repo/platform/contracts/openapi/`
- `repo/platform/contracts/internal/`
- `repo/data-platform/event-schemas/`
- `repo/data-platform/db-schema/`

至少创建：

- 每个前端聚合层 / 领域服务一个 OpenAPI/YAML 文件骨架
- 每个关键事件一个 JSON Schema 文件骨架
- 一份数据库 schema README
- 一份 migration README

### 4.5 测试与基础设施骨架

至少创建（均在 `repo/` 下）：

- `repo/tests/contract/`
- `repo/tests/integration/`
- `repo/tests/e2e/`
- `repo/infra/docker/`
- `repo/infra/kubernetes/`
- `repo/infra/ci/`

并在关键目录放最小 README 或占位文件，保证新成员一打开仓库就知道这里未来放什么。

---

## 5. 你必须生成的核心文件清单

### 5.1 根目录（`repo/`）

- `repo/README.md`
- `repo/Cargo.toml`
- `repo/.gitignore`
- `repo/.editorconfig`
- `repo/Makefile`

### 5.2 apps

- `repo/apps/web/README.md`
- `repo/apps/admin/README.md`

### 5.3 services

以下每个服务至少一套最小 Rust 模板：

- `repo/services/api-gateway/*`
- `repo/services/bff/*`
- `repo/services/identity-service/*`
- `repo/services/profile-service/*`
- `repo/services/context-service/*`
- `repo/services/ai-chat-service/*`
- `repo/services/dm-service/*`
- `repo/services/question-service/*`
- `repo/services/match-service/*`
- `repo/services/safety-service/*`
- `repo/services/model-gateway/*`

### 5.4 contracts

- `repo/platform/contracts/README.md`
- `repo/platform/contracts/openapi/README.md`
- `repo/platform/contracts/internal/README.md`
- `repo/platform/contracts/openapi/` 中**不强制**单独创建 `api-gateway.yaml`
- 权威对外 API 契约由 `bff.yaml` 与各领域服务 YAML 持有；`api-gateway` 作为统一入口与转发层，不单独作为权威业务 OpenAPI 来源
- `repo/platform/contracts/openapi/bff.yaml`
- `repo/platform/contracts/openapi/identity-service.yaml`
- `repo/platform/contracts/openapi/profile-service.yaml`
- `repo/platform/contracts/openapi/context-service-internal-reference.md`
- `repo/platform/contracts/openapi/ai-chat-service.yaml`
- `repo/platform/contracts/openapi/dm-service.yaml`
- `repo/platform/contracts/openapi/question-service.yaml`
- `repo/platform/contracts/openapi/match-service.yaml`
- `repo/platform/contracts/openapi/safety-service.yaml`
- `repo/platform/contracts/internal/model-gateway.yaml`
- `repo/platform/contracts/internal/context-service.yaml`

### 5.5 event schemas

至少创建以下事件文件骨架：

- `repo/data-platform/event-schemas/identity.user.registered.v1.json`
- `repo/data-platform/event-schemas/chat.user_message.created.v1.json`
- `repo/data-platform/event-schemas/context.memory.extracted.v1.json`
- `repo/data-platform/event-schemas/context.memory.summary.updated.v1.json`
- `repo/data-platform/event-schemas/dm.message.created.v1.json`
- `repo/data-platform/event-schemas/profile.memory_projection.requested.v1.json`
- `repo/data-platform/event-schemas/profile.fact.upserted.v1.json`
- `repo/data-platform/event-schemas/profile.embedding.updated.v1.json`
- `repo/data-platform/event-schemas/social.follow.created.v1.json`
- `repo/data-platform/event-schemas/question.answered.v1.json`
- `repo/data-platform/event-schemas/match.request.submitted.v1.json`
- `repo/data-platform/event-schemas/match.result_set.served.v1.json`
- `repo/data-platform/event-schemas/safety.assessment.completed.v1.json`
- `repo/data-platform/event-schemas/model.invocation.completed.v1.json`

其余关键事件可继续补齐，但至少先把主链路全部建出来。

### 5.6 db schema

至少创建：

- `repo/data-platform/db-schema/README.md`
- `repo/data-platform/db-schema/migrations/README.md`（占位，说明正式 migration 由工程师执行）
- `repo/data-platform/db-schema/drafts/README.md`
- `repo/data-platform/db-schema/drafts/001_identity.sql`
- `repo/data-platform/db-schema/drafts/002_profile.sql`
- `repo/data-platform/db-schema/drafts/003_context.sql`
- `repo/data-platform/db-schema/drafts/004_ai_chat.sql`
- `repo/data-platform/db-schema/drafts/005_dm.sql`
- `repo/data-platform/db-schema/drafts/006_question.sql`
- `repo/data-platform/db-schema/drafts/007_match.sql`
- `repo/data-platform/db-schema/drafts/008_safety.sql`
- `repo/data-platform/db-schema/drafts/009_model_gateway.sql`

**SQL 落地策略**：

- `drafts/` 目录的 `.sql` 文件直接包含 `14-MVP-SQL-SCHEMA-DRAFT.md` 中对应域的完整 DDL（不是占位），头部标注 `-- Draft from Rules/14-MVP-SQL-SCHEMA-DRAFT.md, subject to review`
- 工程师可以直接 `psql -f` 建表做本地开发
- `migrations/` 目录只放占位 README，正式 migration 由工程师按实际需要创建，不混淆草案和生产用 migration

---

## 6. Rust 服务模板约束

### 6.1 每个服务至少具备

- 健康检查路由
- 占位配置结构
- 占位错误类型
- 占位路由注册
- 占位 README（写清职责、拥有数据、对外接口、依赖）

### 6.2 `main.rs` 应做的事情

只需要做最小启动壳：

1. 读取配置
2. 初始化日志
3. 注册 health check
4. 注册占位路由
5. 启动 HTTP 服务

### 6.3 绝对不要做的事情

- 不实现业务逻辑
- 不接数据库
- 不接 Kafka
- 不接外部模型
- 不自行决定框架——统一使用 3.3 节冻结的 Rust 技术基线（`tokio` + `axum` + `tracing` + `serde`）
- 不在骨架模板中引入基线之外的其他 crate

---

## 7. 契约文件落地规则

### 7.1 OpenAPI 文件

只落骨架，不写全量真实 schema。

每个 YAML 至少包含：

- `openapi`
- `info`
- `servers`
- `paths`
- `components/schemas`

且至少把 15 中定义的核心路径占位进去。

### 7.2 Event Schema 文件

每个 JSON 文件至少包含：

- `$schema`
- `title`
- `type`
- `properties`
- `required`

并显式包含：

- `event_id`
- `event_name`
- `event_version`
- `occurred_at`
- `producer`
- `payload`

### 7.3 内部契约

`model-gateway` 的内部能力契约单独放在：

- `repo/platform/contracts/internal/model-gateway.yaml`

`context-service` 的内部核心接口契约也单独放在：

- `repo/platform/contracts/internal/context-service.yaml`

不要把它放进前端 OpenAPI 文件中。

---

## 8. README 规范

### 8.1 每个服务 README 必须写明

- 服务职责
- 拥有的数据
- 对外接口
- 依赖哪些服务
- 不负责什么
- 对应的 rules 文档来源

### 8.2 根 README 必须写明

- OneLink 是什么
- 仓库主要目录解释
- MVP 服务清单
- 如何本地启动骨架
- 当前阶段是“骨架完成，业务待实现”

---

## 9. 你不能做的事

### 9.1 禁止改设计

禁止：

- 删除 `bff`
- 把 `ai-chat-service` 和 `dm-service` 合并
- 把 `question-service` 移出 MVP
- 把 `model-gateway` 改回 Go
- 提前引入 `trust-service`
- 新增未批准的服务

### 9.2 禁止过度实现

禁止：

- 直接把 14 的 SQL 全部变成 migration 并强行执行
- 直接把 15 变成完整后端实现
- 直接把 16 变成完整消息系统接入
- 引入大量依赖、脚手架、自动生成物
- 生成庞大前端项目代码

### 9.3 禁止偷换骨架定义

你这轮是“铺路”，不是“修路”。

不要把：

- 占位 schema
- 占位路由
- 占位 service

直接冒充成“已实现 MVP 功能”。

---

## 10. 推荐执行顺序

1. 先建立目录树
2. 再建立根级文件
3. 再建立 11 个服务的 Rust 模板
4. 再建立 contracts / event-schemas / db-schema 骨架
5. 再建立 tests / infra / scripts 占位目录
6. 最后统一补 README

---

## 11. 验收标准

完成后必须满足：

### 11.1 结构完整

- `repo/` 目录树与 `09-PROJECT-STRUCTURE.md` 的 MVP 子集一致
- 11 个 MVP 服务目录全部存在于 `repo/services/`
- contracts / db-schema / event-schemas 目录全部存在于 `repo/` 对应位置
- `Rules/` 和 `docs/` 目录未被修改

### 11.2 技术基线一致

- 所有服务 `Cargo.toml` 使用 `workspace = true` 继承统一依赖版本
- 所有服务模板使用 `axum` + `tokio` + `tracing` + `serde`，无其他框架级依赖
- 根 `Cargo.toml` workspace 声明了全部服务成员

### 11.3 边界正确

- 没有把业务逻辑误写进骨架
- 没有引入新服务
- 没有改变服务边界

### 11.4 可交接

- 工程师能直接在每个服务目录继续写代码
- 契约文件位置清晰
- README 足以让新成员快速理解仓库
- `db-schema/drafts/` 中的 SQL 可以直接 `psql -f` 执行

### 11.5 可复用

- 后续工程师可以直接基于模板继续实现
- 后续不会因为模板结构错误而返工整个仓库

---

## 12. 最终输出口径

完成时，你需要明确告诉总设计师：

1. 实际创建了哪些目录
2. 实际创建了哪些模板文件
3. 哪些契约骨架已经落地
4. 哪些仍然只是占位，等待工程师实现
5. 是否有任何地方与你的输入文档存在冲突

不要回到“应该怎么设计”的讨论。

这一轮目标只有一个：

**把第二步的工程草案，稳定地落成第三步可继续开发的真实仓库骨架。**
