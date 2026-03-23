# OneLink — 工程仓库（骨架阶段）

OneLink 是依托「AI 好朋友」一度人脉连接的社交平台。本目录 `repo/` 为**代码仓库根**；架构与规则见 `OneLink/Rules/`（**请勿与规划文档混为同一 Git 根**，按 `18-COMPOSER-2-FAST-EXECUTION-BRIEF.md` 约定）。

## 当前阶段

**骨架 + 第一条纵切面（chat → memory → profile）可联调。** 其余域仍为占位。  
配套说明（脚本、OpenAPI 尾差、验证文档）：`Rules-V2/EXECUTION/composer-2-fast-chat-memory-profile-brief.md`。

## 目录说明

| 路径 | 说明 |
|------|------|
| `services/` | MVP 11 个在线 Rust 服务 |
| `platform/contracts/` | OpenAPI / 内部契约骨架 |
| `data-platform/db-schema/` | DDL 草案（`drafts/`）与 migration 占位 |
| `data-platform/event-schemas/` | 事件 JSON Schema 骨架 |
| `apps/` | 前端占位（Web / Admin） |
| `ai-platform/` | 提示词 / 评测 / 推理占位 |
| `infra/` | Docker / K8s / CI 占位 |
| `tests/` | 契约 / 集成 / E2E 占位 |
| `scripts/` | 工具脚本占位 |

## MVP 服务清单

- `api-gateway`, `bff`, `identity-service`, `profile-service`, `ai-chat-service`, `context-service`, `dm-service`, `question-service`, `match-service`, `safety-service`, `model-gateway`

## 本地启动

### 纵切面（Chat → Memory → Profile）

默认端口：**8081** identity，**8082** profile，**8083** bff，**8085** ai-chat，**8089** context，**8090** model-gateway（与 `Rules/20` / 任务书一致）。

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

- `OneLink/Rules/10-SERVICE-BOUNDARIES.md`
- `OneLink/Rules/11-DATA-EVENT-MODEL.md`
- `OneLink/Rules/14-MVP-SQL-SCHEMA-DRAFT.md` ~ `17-MVP-SERVICE-CONTRACTS.md`
- `OneLink/Rules/18-COMPOSER-2-FAST-EXECUTION-BRIEF.md`
- `OneLink/Rules/19-CONTEXT-MEMORY-ARCHITECTURE.md`
