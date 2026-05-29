# OneLink — 工程仓库（骨架阶段）

OneLink 是依托「AI 好朋友」一度人脉连接的社交平台。本目录 `repo/` 为**代码仓库根**；当前架构与规则见 `OneLink/rules/`（**请勿与规划文档混为同一 Git 根**）。

## 文档导航

- **Agent 轻量记忆（Cursor / 新会话优先读）**：仓库根 [`docs/AGENT_MEMORY_BRIEF.md`](../docs/AGENT_MEMORY_BRIEF.md)（当前规则入口、端口与密钥、排障指针；与 `rules/` 冲突时以规则与代码为准）。
- **统一规则入口（canonical）**：[`rules/README.md`](../rules/README.md)、[`rules/09-NEXT-DEVELOPMENT-PLAN.md`](../rules/09-NEXT-DEVELOPMENT-PLAN.md)。
- **历史归档入口**：[`docs/archive/rules-legacy-2026-05-15/`](../docs/archive/rules-legacy-2026-05-15/)（仅作历史依据与字段追溯，不作当前默认开发入口）。
- **ECS 部署入口**：[`../docs/ONELINK_ECS_DEPLOY_PRODUCTION_README.md`](../docs/ONELINK_ECS_DEPLOY_PRODUCTION_README.md)、[`deploy/ecs/README.md`](deploy/ecs/README.md)、[`scripts/README.md`](scripts/README.md)。

## 当前阶段

**骨架 + 第一条纵切面（chat → memory → profile）可联调。** BFF 边界已冻结：客户端统一通过 `/api/v1/bff/*` 接入；`/api/v1/app/*` 已移除。

**当前规划入口（canonical）**：`../rules/`。当前 phase 为 Phase 11 (Wave 0 Engineering Reset And Boundary Freeze)。  
纵切面联调说明（验证步骤、脚本）：`tests/integration/CHAT_MEMORY_PROFILE_SLICE.md`。旧执行 brief 已归档到 `../docs/archive/rules-legacy-2026-05-15/`，不再作为默认发单入口。

## 目录说明

| 路径 | 说明 |
|------|------|
| `services/` | MVP 11 个在线 Rust 服务 |
| `platform/contracts/` | OpenAPI / 内部契约骨架 |
| `data-platform/db-schema/` | DDL 草案（`drafts/`）与 migration 占位 |
| `data-platform/event-schemas/` | 事件 JSON Schema 骨架 |
| `apps/app/` | Rust Axum 服务聚合入口（`onelink-app` crate，依赖 `onelink-app-server`；**非移动端**，仅为服务端启动器） |
| `apps/app-server/` | Rust 库 `onelink-app-server`，被 `apps/app/` 引用，提供路由与状态集成 |
| `apps/admin/` | 最小后台管理面板（占位） |
| `apps/mobile/` | React Native + TypeScript 移动端客户端 |
| `apps/web/` | React + Vite Web 客户端 |
| `ai-platform/` | 提示词 / 评测 / 推理占位 |
| `infra/` | Docker / K8s / CI 占位 |
| `tests/` | 契约 / 集成 / E2E 占位 |
| `scripts/` | 工具脚本占位 |

## MVP 服务清单

- `api-gateway`, `bff`, `identity-service`, `profile-service`, `ai-chat-service`, `context-service`, `dm-service`, `question-service`, `match-service`, `safety-service`, `model-gateway`

## 本地启动

### 纵切面（Chat → Memory → Profile）

默认端口：**8081** identity，**8082** profile，**8083** bff，**8085** ai-chat，**8089** context，**8090** model-gateway。

**`INTERNAL_SHARED_SECRET`**：`ai-chat` / `context` / `profile` 间 dev-only 内部 relay 须 **同值**（header `x-internal-token`）。`run-chat-memory-profile-slice.sh start-bg` 会导出默认 `onelink-dev-internal-token`；手动多终端时请自行对齐。

当前 `context-service` 已提供最小 **ASMR-Lite** 观测面：`GET /internal/observability/asmr-lite`（需 `x-internal-token`），可查看路由计数、失败样本、checkpoint 数与 artifact / summary / entity 数。`ai-chat-service` 也提供 `GET /internal/observability/chat-relay`，用于查看 chat relay 失败记录。

```bash
cd repo
# 打印逐终端启动命令：
bash scripts/local/run-chat-memory-profile-slice.sh print-start
# 或后台启动（开发态，日志在 /tmp/onlink-*.log）：
bash scripts/local/run-chat-memory-profile-slice.sh start-bg
sleep 12
bash scripts/local/run-chat-memory-profile-slice.sh smoke
bash scripts/local/run-chat-memory-profile-slice.sh benchmark-v1
```

验证文档：`tests/integration/CHAT_MEMORY_PROFILE_SLICE.md`。  
OpenAPI（本轮）：`platform/contracts/openapi/identity-service.yaml`、`profile-service.yaml`、`bff.yaml`、`ai-chat-service.yaml`。

### 其他服务（骨架）

```bash
cd repo
export RUST_LOG=info
cargo run -p api-gateway   # 默认 PORT=8080
# …
```

各 crate 默认 `PORT` 见各自 `config.rs` / 任务书；未设置时 **不一定** 为 8080。

## 规范来源

- `OneLink/rules/00-CONSTITUTION.md`
- `OneLink/rules/02-SYSTEM-ARCHITECTURE.md`
- `OneLink/rules/03-SERVICE-BOUNDARIES.md`
- `OneLink/rules/04-DATA-EVENT-CONTRACTS.md`
- `OneLink/rules/07-ENGINEERING-QUALITY.md`
- `OneLink/rules/09-NEXT-DEVELOPMENT-PLAN.md`

若服务 README、archive 历史文档与上述文件冲突，以当前代码、`repo/` 内现行契约 / schema 与这些 `rules/` 文件为准。
