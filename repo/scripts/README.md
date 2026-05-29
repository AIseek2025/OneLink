# scripts

本地与 CI 辅助脚本（生成代码、检查契约漂移、纵切面联调等）。

当前裁决顺序为：运行代码与测试 > `repo/platform/contracts/` > `repo/data-platform/` > `OneLink/rules/` > `OneLink/docs/archive/`。

脚本行为以当前脚本本身、`repo/tests/integration/`、`repo/platform/contracts/` 与 `OneLink/rules/07-ENGINEERING-QUALITY.md`、`OneLink/rules/09-NEXT-DEVELOPMENT-PLAN.md` 为准。archive 中的 brief、benchmark 与 closeout 仅用于历史背景和验收追溯。

## Chat → Memory → Profile 纵切面（Phase A/B 主线）

- **`local/run-chat-memory-profile-slice.sh`**：`print-ports` / `print-start` / `start-bg` / **`smoke`** / **`smoke-persistence`** / **`smoke-context-log`** / **`smoke-runtime-obs`** / **`smoke-forgetting`** / `benchmark-v1` / `benchmark-v2` / `benchmark-v2.1` / **`benchmark-v2.2`**（**chmod +x** 已要求）。
- **`smoke-chat-memory-profile.sh`**：HTTP 串行 smoke（需 **六**服务已启动：`identity`、`profile`、`bff`、`ai-chat`、`context`、`model-gateway`；`curl` + `jq`）；内嵌对 `facts` / `traits` / `completion` 五维等的 jq 断言（**不**绑死展示长句）。**不要求** `DATABASE_URL`（context 走 in-memory fallback 即可）。
- **`smoke-chat-memory-profile-phase-a.sh`**：可选别名，与上脚本等价（便于文档引用）。
- **`run-local-vertical-slice-terminals.sh`**：仅打印逐终端 `cargo run` 说明（与上类似，保留兼容）。

## Phase C：问卷 → context（questionnaire）→ profile

- **`smoke-questionnaire-profile.sh`**：在 Phase A/B 端口矩阵基础上 **另需 `question-service` 8086**，且 **BFF** 启动时须配置 **`QUESTION_SERVICE_BASE_URL`**（与 `run-chat-memory-profile-slice.sh start-bg` / `print-start` 一致）。  
  - 覆盖 **`GET /api/v1/bff/onboarding`**（问卷域 `progress` + pending）、**`chat/init`**、`POST /questions/answers`、异步后 **`profile/me`** / **`profile/me/completion`** / context **`asmr-lite`**。  
  - **`POST /answers` 之后**对 **`profile/me` 轮询**等待结构化 facts 出现（默认约 60s 预算），**非**固定 `sleep`；可调 **`QUESTIONNAIRE_SMOKE_POLL_MAX`**、**`QUESTIONNAIRE_SMOKE_POLL_INTERVAL`**。  
  - **不是**完整问卷产品验收；MVP / in-memory / dev-only relay 边界见各服务 README。

- **编排**：`bash scripts/local/run-chat-memory-profile-slice.sh smoke-questionnaire`（子命令名与 Phase A/B 并列，**未删减**旧入口）。

## context-service 跨重启 persistence smoke（**依赖 Postgres**）

- **`smoke-chat-memory-persistence.sh`**：**不是**完整生产持久化验收；仅验证「写入 → 观测计数 → **真实 kill 再起**同一二进制 → 计数不回退 / `policy_version_label` 一致」这条能力边界。
- **环境变量**
  - **`DATABASE_URL`**（必填）：目标库须已执行 **`001_identity.sql`**、**`003_context.sql`**、**`003_context_idempotency.sql`**（路径：`repo/data-platform/db-schema/drafts/`）。**不要求**服务内 auto-migrate。
  - **`INTERNAL_SHARED_SECRET`**（可选，默认 `onelink-dev-internal-token`）：与 `memory/write`、`asmr-lite` 的 `x-internal-token` 一致。
  - **`CONTEXT_PORT`**（可选，默认 `8089`）：脚本会**自行** `cargo build` 并两次启动 **`target/debug/context-service`**（中间 `kill`），**不要**与手工占用的同端口进程冲突。
- **依赖**：`curl`、`jq`、**`psql`**（用于检查表存在性及 `INSERT users` 满足 `memory_*` 外键）。
- **执行**

```bash
cd OneLink/repo
export DATABASE_URL='postgres://USER:PASS@127.0.0.1:5432/DBNAME'
bash scripts/smoke-chat-memory-persistence.sh
```

- **编排入口**：`bash scripts/local/run-chat-memory-profile-slice.sh smoke-persistence`（与 Phase A/B `smoke`、问卷 `smoke-questionnaire`、benchmark 子命令**并列**，旧入口不变）。

## context_logs：`POST /internal/context/build` 写入门禁（**依赖 Postgres**）

- **`smoke-chat-memory-context-log.sh`**：验证 Postgres 模式下真实调用 **`POST /internal/context/build`** 后 **`context_logs`** 表对该 `user_id` **至少新增一行**（与 persistence smoke 一样先 **`INSERT users … ON CONFLICT DO NOTHING`** 满足外键；并检查 **`context_logs` 表存在**）。**不**强制跨重启断言（可选加分项）。
- **实现侧对齐（与 `003_context.sql`）**：服务在 Postgres 路径写入 **`token_budget_json`**（**NOT NULL**），由 build 请求体的 **`max_tokens` / `memory_limit` / `summary_limit`** 构造；**无 `DATABASE_URL`**（in-memory）时**不写** `context_logs` 行。
- **环境变量**：与 persistence smoke 相同（**`DATABASE_URL`** 必填；库须含 **`001_identity.sql`** + **`003_context.sql`**（含 `context_logs`）；**不强制** `003_context_idempotency.sql`）。
- **执行**：`bash scripts/smoke-chat-memory-context-log.sh`；编排：`bash scripts/local/run-chat-memory-profile-slice.sh smoke-context-log`。

## context-service：activation scoring 门禁（**依赖 Postgres**）

- **`smoke-chat-memory-activation.sh`**：验证 Phase 1 activation 已真实接通：
  - `POST /internal/memory/write` 后 `memory_artifacts.importance_score` 非空
  - `POST /internal/context/build` 命中同一 artifact 后，`access_count` 严格增加，`last_accessed_at` 非空
- **前置**：**`DATABASE_URL`**；**`001_identity.sql`**、**`003_context.sql`**、**`003_context_activation.sql`**
- **退出码**：**0** 成功；**1** 实现/断言失败；**2** 环境或 SQL 前置缺失（未设置 **`DATABASE_URL`**、缺 activation 列等）
- **执行**：`bash scripts/smoke-chat-memory-activation.sh`
- **编排**：`bash scripts/local/run-chat-memory-profile-slice.sh smoke-activation`

## context-service：routing / failure 快照跨重启（**依赖 Postgres + 011**）

- **`smoke-chat-memory-runtime-observability.sh`**：验证 **`011_runtime_observability.sql`** 已建表后，routing 写入 DB，且 **`GET /internal/observability/asmr-lite`** 在 **kill 再起** 后仍能读到一致的 **`last_observation`** / failure 视角（与进程内 **`routing.total_requests` 等累加计数**分工不同，后者仍重启归零）。
- **前置**：**`DATABASE_URL`**；**`001_identity.sql`**、**`003_context.sql`**、**`011_runtime_observability.sql`**。
- **依赖**：`curl`、`jq`、**`psql`**。
- **退出码**：**0** 成功；**1** 实现/断言失败（HTTP 非 200、跨重启 preview 不一致、DB 行数未增等）；**2** 环境或前置缺失（未设置 **`DATABASE_URL`**、缺 **`context_routing_observations` / `context_failure_events`** 表等）。
- **执行**：`bash scripts/smoke-chat-memory-runtime-observability.sh`；编排：`bash scripts/local/run-chat-memory-profile-slice.sh smoke-runtime-obs`。

## context-service：`forgetting_decisions` 在线写入（**依赖 Postgres**）

- **`smoke-chat-memory-forgetting-decisions.sh`**：真实 **`POST /internal/memory/forgetting/decide`** 后断言 **`forgetting_decisions`** 对该测试用户新增行；先 **`INSERT users … ON CONFLICT DO NOTHING`**（与 `context_logs` smoke 同模式，满足 FK）。
- **前置**：**`DATABASE_URL`**；**`001_identity.sql`**、**`003_context.sql`**（含 **`forgetting_decisions`**）。
- **退出码**：**0** 成功；**1** 实现/断言失败；**2** 环境或前置缺失（未设置 **`DATABASE_URL`**、缺 **`forgetting_decisions`** 表等）。
- **无 DB 路径**：不跑本脚本；**`cargo test -p context-service --test forgetting_decide_no_db`** 覆盖 **`noop_no_database`** 响应。
- **执行**：`bash scripts/smoke-chat-memory-forgetting-decisions.sh`；编排：`bash scripts/local/run-chat-memory-profile-slice.sh smoke-forgetting`。

## Benchmark（与 chat 纵切共享服务进程）

- **`benchmark-asmr-lite-v1.sh`** / **`benchmark-asmr-lite-v2.sh`** / **`benchmark-asmr-lite-v2.1.sh`**：固定跑法见 `tests/integration/CHAT_MEMORY_PROFILE_SLICE.md` 与对应 `ASMR_LITE_BENCHMARK_*.md`。  
- **`benchmark-asmr-lite-v2.2.sh`**：**Postgres + `010_optimization.sql`（`policy_configs`）** 下种子 **`graph_enabled` / `rerank_enabled` / `enabled_retrieval_modes`**，断言 **`retrieval_used`** 含 **`graph`/`rerank`**（**不**替代 v2 / v2.1）。
- **`benchmark-asmr-lite-v2.2.sh` 退出码**：**0** 成功；**1** 实现/断言失败（build 非 200、**`retrieval_used`** 缺模式等）；**2** 环境或前置缺失（未设置 **`DATABASE_URL`**、缺 **`policy_configs`** 表等）。
- 亦可通过 **`run-chat-memory-profile-slice.sh benchmark-v1`**（或 `benchmark-v2`、`benchmark-v2.1`、**`benchmark-v2.2`**）调用。

### context-service：crate 级单测（无需 Postgres）

在 **`repo`** 根目录：

```bash
cargo test -p context-service
```

覆盖 **routing 进程内累加**、**policy 合并/过滤**、**`enabled_retrieval_modes` 解析（含 JSON `[]` 忽略）**、**graph/rerank 语义**、**forgetting no-db**（见 `services/context-service/README.md` §Runtime baseline hardening）。

## Rust 集成测试壳（可选 HTTP smoke）

在 **`repo`** 根目录：

```bash
cargo test -p integration_chat_memory_profile_slice
```

- **Phase A/B**（六服务 + `smoke-chat-memory-profile.sh`）：`RUN_SLICE_HTTP_SMOKE=1 cargo test -p integration_chat_memory_profile_slice slice_http_smoke -- --exact --nocapture`
- **Phase C**（七服务含 question + BFF `QUESTION_SERVICE_BASE_URL` + `smoke-questionnaire-profile.sh`）：`RUN_SLICE_QUESTIONNAIRE_SMOKE=1 cargo test -p integration_chat_memory_profile_slice slice_questionnaire_http_smoke -- --exact --nocapture`

详见 **`tests/integration/CHAT_MEMORY_PROFILE_SLICE.md`**。

## 契约与 OpenAPI

- 仓库内规范：`platform/contracts/openapi/*.yaml`；BFF **`GET /api/v1/bff/onboarding`** 见 **`bff.yaml`**（**`progress`** = 问卷域，**非** `profile/me/completion`）。

## ECS 部署脚本

- **`onelink-ecs-preflight-check.sh`**：共享 ECS 部署前预检，检查运行时、目录、部署资产、`onelink.env` 和 `nginx` 语法；若发现 `change-me`、`dev-only-shared-secret` 或缺 `production` 环境，直接失败。
- **`onelink-ecs-deploy.sh`**：在 ECS 上执行 OneLink 自有部署流程：
  - 构建 `apps/web`
  - 构建 Rust release 二进制
  - 生成 per-service 端口环境文件
  - 执行 `onelink-migration-runner`
  - 安装 `systemd` 模板
  - 重启 `onelink@<binary>.service`
  - 安装并加载 OneLink 自己的 `nginx` 站点
- **`onelink-ecs-cert.sh`**：在 `onelink.cool` DNS 生效后，使用 `certbot --webroot` 申请证书，并切换到 HTTPS 站点模板。

部署资产位置：

- `repo/deploy/ecs/README.md`
- `repo/deploy/ecs/onelink.env.example`
- `repo/deploy/ecs/systemd/onelink@.service`
- `repo/deploy/ecs/nginx/onelink.cool.http.conf`
- `repo/deploy/ecs/nginx/onelink.cool.https.conf`
