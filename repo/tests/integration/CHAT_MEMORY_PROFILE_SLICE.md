# Chat → Memory Projection → Profile 可见 — 验证说明

> **验证说明**（纵切面联调；非当前多代理发单入口）。主链路由与事件消费在各服务 crate 内实现；本文只描述如何联调与观察结果。  
> **当前规则入口**：`OneLink/rules/`。旧多代理发单索引已归档到 `OneLink/docs/archive/rules-legacy-2026-05-15/Rules-V2/EXECUTION/README.md`，勿当作默认下一主波次工单。  
> 验收基线：`OneLink/docs/archive/rules-legacy-2026-05-15/Rules/20-FIRST-RUNNABLE-VERTICAL-SLICE-BRIEF.md`。  
> 事件口径：`OneLink/docs/archive/rules-legacy-2026-05-15/Rules/16-MVP-EVENT-SCHEMAS-DRAFT.md`（勿改事件名 / producer）。  
> V2 约束：`OneLink/docs/archive/rules-legacy-2026-05-15/Rules-V2/00-CONSTITUTION.md`、`OneLink/docs/archive/rules-legacy-2026-05-15/Rules-V2/CONTRACTS/context-service-contract.md`、`OneLink/docs/archive/rules-legacy-2026-05-15/Rules-V2/EXECUTION/asmr-lite-benchmark-and-acceptance-checklist.md` 用于约束 MVP 边界与验收清单口径。
> **冲突处理**：若 archive 资料与当前 `repo/` 内代码、契约、schema 或 `OneLink/rules/` 不一致，以现行工程资产与 `rules/` 为准；archive 仅用于历史追溯与冻结字段核对。

## 端口矩阵（冻结）

| 服务 | 端口 |
|------|------|
| api-gateway | 8080（本轮纵切面可不启） |
| identity-service | 8081 |
| profile-service | 8082 |
| bff | 8083 |
| ai-chat-service | 8085 |
| question-service | **8086**（Phase C；端口冻结） |
| context-service | 8089 |
| model-gateway | 8090 |

## 鉴权与内部口令（本轮纵切面）

### Bearer（用户态）

- **ai-chat** 的 **`GET …/messages`**、**`GET …/context`** 与写接口一样，必须带 **`Authorization: Bearer <token>`**，且 **会话须属于当前用户**（否则 403/404）。
- **profile** 的 **`GET /profile/me`**、**`GET /profile/me/completion`** 需要 Bearer。
- **BFF** `GET /api/v1/bff/chat/init` 与 **`GET /api/v1/bff/onboarding`** 仅 **透传** `Authorization` 到 identity / question-service（`chat/init` 另透传到 ai-chat），不在 BFF 侧解析 token。

### `INTERNAL_SHARED_SECRET` + `x-internal-token`（dev-only 服务间）

- **Header 名**：`x-internal-token`（实现按此名读取）。
- **值**：与各服务环境变量 **`INTERNAL_SHARED_SECRET`** 相同；未设置时实现默认 **`onelink-dev-internal-token`**。
- **须一致的服务进程**：**ai-chat-service**、**context-service**、**profile-service**；**question-service** 向 context relay `question.answered.v1` 时也校验同一 header（本地 `start-bg` 或 `print-start` 时请统一 export）。
- **典型调用链**（由实现发起，**不要**在浏览器或 smoke 里伪造）：
  - ai-chat → context：`POST /internal/context/build`（带 `x-internal-token`）
  - context → ai-chat：`GET /internal/chat/.../messages/{id}`（带 `x-internal-token`）
  - context → profile：`POST /internal/events/receive`（带 `x-internal-token`）
  - question-service → context：`POST /internal/events/receive`（`question.answered.v1`，带 `x-internal-token`）
  - ai-chat → context：`POST /internal/events/receive`（带 `x-internal-token`）
  - profile → context：`POST /internal/memory/resolve`（带 `x-internal-token`）
- **说明**：这是 **开发态 relay 共享口令**，**不是**生产级零信任 / 服务身份方案。

### Identity `expires_at` 与 smoke

- `session.expires_at` 为 **真实过期时间**（当前实现约 **30 天**）。`GET /api/v1/identity/me` 过期返回 **401**。
- 若 smoke 失败且返回 401，先确认是否用了**旧 token** 或长时间挂机后重跑；**重新 register** 即可排除「过期误判为链路坏了」。

## 最小验证顺序

1. **注册**（拿 `session.token`）  
   `POST http://127.0.0.1:8081/api/v1/identity/register`  
   Body 字段见 `OneLink/docs/archive/rules-legacy-2026-05-15/Rules/15` §2.1 与 `platform/contracts/openapi/identity-service.yaml`。

2. **（可选）登录** — 与注册二选一用于拿 token；若已注册则用 login。

3. **BFF 聊天初始化** — 透传 `Authorization: Bearer <token>`  
   `GET http://127.0.0.1:8083/api/v1/bff/chat/init`  
   期望 JSON：`user`（identity.me）、`conversation`、`pending_questions`（**Phase C**：question-service **8086** 可用且 BFF 配置 `QUESTION_SERVICE_BASE_URL` 时为 **待答题 `items` 形** 的数组；question-service 失败时 **降级为 `[]`**，**不阻断** init）。

3b. **（Phase C）BFF 冷启动问卷聚合** — 同 Bearer，**不**创建聊天会话  
   `GET http://127.0.0.1:8083/api/v1/bff/onboarding`  
   期望 JSON：`user`、`pending_questions`（与上 **同形** 降级语义）、**`progress`**（透传 question-service **`GET /api/v1/questions/completion`** 的问卷域计数；**不是** `profile/me/completion`）。question-service 异常时 `progress` 可为 **`{ "degraded": true, "reason": "..." }`**。

4. **发用户消息** — 需同一 Bearer  
   `POST http://127.0.0.1:8085/api/v1/chat/conversations/{conversation_id}/messages`  
   Body：`content_type`, `content_text`, `idempotency_key`（见 `OneLink/docs/archive/rules-legacy-2026-05-15/Rules/15` §4）。
   说明：`idempotency_key` 当前仅作前向兼容字段保留，**MVP 不执行请求去重**；不要把它当作真实重试保护。

5. **（推荐）列消息** — 验证读路径鉴权：  
   `GET http://127.0.0.1:8085/api/v1/chat/conversations/{conversation_id}/messages`  
   Header：`Authorization: Bearer <token>`（与 `scripts/smoke-chat-memory-profile.sh` 一致）。

6. **等待异步链路** — dev-only HTTP relay：`ai-chat` → `context` → `profile`；建议 `sleep 1`～`2`。  
   若画像长期不更新，检查三服务 **`INTERNAL_SHARED_SECRET`** 是否一致（见上）。

7. **读画像**  
   - `GET http://127.0.0.1:8082/api/v1/profile/me`  
   - `GET http://127.0.0.1:8082/api/v1/profile/me/completion`  

   **`profile/me/completion` 与问卷**：此为 **五维结构化画像覆盖率**，与 **question-service** 的 **`/questions/status`、`/questions/completion`（问卷题包进度）不联动**；勿混用口径。

   **Phase A/B 期望（结构化，不靠「随便有字」）**：响应体含 `facts`（非空数组）、`traits`（**须含键** `location_label`，可为 `null`；`interest_tags` / `connection_goal_tags` / `communication_preferences` 等轴在 smoke 样本下非空）；`completion.required_dimensions` 为五维结构化集合；**勿**将 `headline`/`bio` 的具体长句当作稳定契约。`headline` 仍应包含子串 **`记忆已同步`**（benchmark v1 断言依赖）。  
   **Phase B 事实层**：每条 `facts[]` 元素含 **`confidence`**（0~1 启发式，非模型分）、**`source_memory_id`**（与 `memory/resolve` 的 `memory_id` 对齐）；**`source_message_id`** 仅在上游记忆行携带时出现。  
   当前实现 **不**在 `GET /me` 中返回 `memory_highlights`（仅存服务内部用于派生），以 `repo/platform/contracts/openapi/profile-service.yaml` 为准。

8. **（推荐）查看 ASMR-Lite 观测面**  
   `GET http://127.0.0.1:8089/internal/observability/asmr-lite`  
   Header：`x-internal-token: ${INTERNAL_SHARED_SECRET:-onelink-dev-internal-token}`  
   期望：
   - `artifact_count` / `summary_count` / `entity_count` 大于 0
   - `checkpoint_count` 可用于确认 checkpoint 内部接口已具备真实写入行为（无 `DATABASE_URL` 时为进程内；有 `DATABASE_URL` 时为 Postgres 聚合）
   - `routing.total_requests` 递增（**进程内累加**，跨重启归零；与 DB 快照字段分工不同）
   - `routing.last_observation.executed_route == "L1"`
   - `routing.last_observation.summary_hits` / `artifact_hits` / `entity_hits` 可读（「问城市但未点名」时 `entity_hits` 可对齐用户已抽取的 `location` 实体数）
   - `routing.last_observation.route_confidence` / `estimated_llm_calls` / `estimated_tokens` / `query_preview` 可读
   - `routing.last_observation.query_preference_polarity` / `evidence_preference_polarity` 可读（`evidence_preference_polarity` 对齐证据蒸馏极性；`query_preference_polarity` 为查询侧启发式，开放式偏好问句常为 `neutral`，见 `context-service` README）
   - `routing.last_observation.retrieval_modes`：数组，与最近一次 **`POST /internal/context/build`** 经策略过滤后的实际检索模式一致（可与响应体 **`retrieval_used`** 对照）
   - 根级 **`policy_version_label`**、**`latest_summary_policy_version`**：前者为当前进程策略单值标签；后者在有聊天/问卷等已写入的 summary 时应非空，并与前者对齐（MVP 门禁；**不**表示 optimization-layer **`policy_*`** 表已在线驱动本服务）
   - **计数来源（双模式）**：`artifact_count` / `summary_count` / `entity_count` / `link_count` / `checkpoint_count` 在 **未设置 `DATABASE_URL`** 时反映**进程内**累计；在 **已设置 `DATABASE_URL`** 且表已初始化时反映 **Postgres 表行数（持久化视角）**。**`routing.total_requests` 等聚合计数** 始终为**当前进程**内视角。**在 Postgres 且已执行 `011_runtime_observability.sql`** 时，**`routing.last_observation`、`recent_failures`、根级 `total_failures`** 可由 DB **最新快照**覆盖（跨进程重启可读）；否则仍为进程内池（与 `repo/platform/contracts/internal/context-service.yaml` 一致）。
   - 若 `candidate_route` 为 `L2`/`L3`，当前实现会记录到 `escalation_reasons` 与 `recent_failures`
   - 若本轮样本触发 `route_escalation_deferred`，对应 failure record 应尽量带非空 `trace_id`，便于从 chat 请求关联回 observability

9. **（推荐）查看 chat relay 失败面**  
   `GET http://127.0.0.1:8085/internal/observability/chat-relay`  
   Header：`x-internal-token: ${INTERNAL_SHARED_SECRET:-onelink-dev-internal-token}`  
   期望：
   - 正常样本下 `total_failures == 0`
   - 若 `ai-chat -> context` 连续失败 3 次，最近失败记录应包含 `attempt_count` 与 `trace_id`

10. **注意 `202 Accepted` 语义**  
   `POST /internal/events/receive` 返回 `202` 只表示**已接受异步处理**，不表示后续 `message fetch`、`projection relay` 已成功完成；请结合 `asmr-lite` / `chat-relay` 观测面判断最终状态。

## context-service：可选 Postgres 持久化与跨重启 smoke

> **能力边界**：验证「核心 memory + checkpoint 可跨进程重启恢复」的**最小工程门禁**，**不是**完整生产级持久化或备份恢复验收。与 Phase A/B **六服务 smoke** 互补：后者**不要求**数据库；本节约**要求** `DATABASE_URL` 与已建表。

**前置**

1. PostgreSQL 已应用（至少）：`repo/data-platform/db-schema/drafts/001_identity.sql`、`003_context.sql`、`003_context_idempotency.sql`。
2. 环境变量 **`DATABASE_URL`** 指向该库。
3. 启动 `context-service` 时**导出同一 `DATABASE_URL`**（与 `repo/services/context-service/README.md` 一致）。

**脚本**（自行编译并两次启动 `target/debug/context-service`，中间 `kill`，真实重启）：

```bash
cd OneLink/repo
export DATABASE_URL='postgres://...'
bash scripts/smoke-chat-memory-persistence.sh
```

或编排：`bash scripts/local/run-chat-memory-profile-slice.sh smoke-persistence`

**断言方向**：写入后 `artifact_count` / `checkpoint_count` 合理；重启后计数**不下降**；`policy_version_label` 与重启前一致。不断言具体实现字段映射（JSONB / entity link 映射等属实现细节）。

**旧入口不变**：`smoke-chat-memory-profile.sh`、`benchmark-asmr-lite-v2.sh`、`benchmark-asmr-lite-v2.1.sh` 在**不设置** `DATABASE_URL` 时仍应可跑（in-memory fallback）。

## context_logs：`POST /internal/context/build` 写入门禁（**依赖 Postgres**）

> **能力边界**：证明「真实调用 build 后 **`context_logs` 对该 `user_id` 至少多一行**」的**最小工程门禁**，**不是**完整审计平台或跨重启恢复验收。与 **persistence smoke** 互补：后者验 memory/checkpoint **计数跨重启不回退**；本条**不**把「kill 再起后仍能查到同一行」当作硬性断言。

**前置**

1. PostgreSQL 已应用（至少）：`001_identity.sql`（**`users`**，满足 `context_logs.user_id` 外键）、**`003_context.sql`**（含 **`context_logs`**）。
2. **`DATABASE_URL`**；**不强制** `003_context_idempotency.sql`（与本门禁无关）。
3. 脚本会自行 `cargo build` 并启动 **`target/debug/context-service`**（默认端口 **8089**）。

**脚本**

```bash
cd OneLink/repo
export DATABASE_URL='postgres://...'
bash scripts/smoke-chat-memory-context-log.sh
```

或编排：`bash scripts/local/run-chat-memory-profile-slice.sh smoke-context-log`

**脚本侧环境准备**：检查 **`context_logs` 表存在**；**`INSERT INTO users (id) … ON CONFLICT DO NOTHING`**（与 persistence smoke 一致，避免 FK 误判为实现缺陷）。

**断言方向**：`POST /internal/context/build` 返回 **200** 后，该测试 `user_id` 在 **`context_logs`** 中行数 **严格增加**。不读取 `asmr-lite` 替代 DB 断言。

**与 in-memory 模式**：未设置 `DATABASE_URL` 时，实现**不写** `context_logs` 行；本脚本**不适用**，请仍用六服务 **`smoke`** / benchmark。

## Activation scoring：Phase 1 门禁（**依赖 Postgres**）

**前置**：`001_identity.sql`、`003_context.sql`、`003_context_activation.sql`；**`DATABASE_URL`**。

```bash
cd OneLink/repo
export DATABASE_URL='postgres://...'
bash scripts/smoke-chat-memory-activation.sh
```

或编排：`bash scripts/local/run-chat-memory-profile-slice.sh smoke-activation`

**断言方向**：`memory/write` 后 `importance_score` 非空；后续 `context/build` 命中同一 artifact 后，`access_count` 严格增加，`last_accessed_at` 非空。

## Routing / failure 观测快照：`011_runtime_observability` + asmr-lite 跨重启（**依赖 Postgres**）

> **能力边界**：验证 **routing/failure 行写入 DB** 且新进程 **`asmr-lite`** 仍能对齐**重启前**的 **`last_observation` / failure 计数视角**；**不**断言 **`routing.total_requests` 等聚合计数**跨重启保留（实现侧为进程内 Mutex）。

**前置**：`001_identity.sql`、`003_context.sql`、`011_runtime_observability.sql`；**`DATABASE_URL`**。

```bash
cd OneLink/repo
export DATABASE_URL='postgres://...'
bash scripts/smoke-chat-memory-runtime-observability.sh
```

或：`bash scripts/local/run-chat-memory-profile-slice.sh smoke-runtime-obs`

## `forgetting_decisions`：`POST /internal/memory/forgetting/decide`（**依赖 Postgres**）

**前置**：`001_identity.sql`、`003_context.sql`（含 **`forgetting_decisions`**）；脚本含 **`users` 预插**（FK）。

```bash
cd OneLink/repo
export DATABASE_URL='postgres://...'
bash scripts/smoke-chat-memory-forgetting-decisions.sh
```

或：`bash scripts/local/run-chat-memory-profile-slice.sh smoke-forgetting`

## Benchmark v2.2：`policy_configs` 只读 + `graph`/`rerank` 门禁（**依赖 Postgres**）

**前置**：`001_identity.sql`、`003_context.sql`、**`010_optimization.sql`**（**`policy_configs`**）。脚本会 **UPSERT** `graph_enabled` / `rerank_enabled` / `enabled_retrieval_modes`（optimization-layer  keys；context-service **只读**）。

```bash
cd OneLink/repo
export DATABASE_URL='postgres://...'
bash scripts/benchmark-asmr-lite-v2.2.sh
```

或：`bash scripts/local/run-chat-memory-profile-slice.sh benchmark-v2.2`

## Runtime baseline hardening：证明抓手与脚本退出码（Composer 2 / Composer 2 fast）

> **不是**新平台能力。`Runtime Baseline Hardening` 在 `OneLink/docs/archive/rules-legacy-2026-05-15/Rules-V2/EXECUTION/` 叙述上已终验归档；本节描述**仍可在本仓库运行的**测试与脚本约定，与历史任务书 `composer-2-runtime-baseline-hardening-brief.md` 对齐。当前开发入口见 `OneLink/rules/09-NEXT-DEVELOPMENT-PLAN.md`。

**`cargo test -p context-service`**（**repo** 根目录）：进程内 **routing 累加**、**in-memory 无持久化 asmr 快照**、**`PolicyConfigStore`** 合并与 **`filter_retrieval_modes`**、**`policy_configs` 值解析**（**`enabled_retrieval_modes` 的 JSON `[]` 视为无覆盖**，不进入逗号拆字面量路径；非法布尔字符串跳过该键）、**graph 拉入非首轮候选**、**rerank second-pass 改序**、集成测 **`forgetting_decide_no_db`**（**`noop_no_database`**）。

**下列脚本退出码**（与实现缺陷 vs 环境缺失区分）：

- **0**：成功
- **1**：实现/断言失败（HTTP 非 200、DB 断言失败、**`retrieval_used`** 不符合预期等）
- **2**：环境或 SQL 前置缺失（未设置 **`DATABASE_URL`**、缺 **`011`** / **`forgetting_decisions`** / **`policy_configs`** 等表）

涉及脚本：**`smoke-chat-memory-runtime-observability.sh`**、**`smoke-chat-memory-forgetting-decisions.sh`**、**`benchmark-asmr-lite-v2.2.sh`**。编排入口仍见 **`run-chat-memory-profile-slice.sh`**（子命令 **`smoke-runtime-obs`** / **`smoke-forgetting`** / **`benchmark-v2.2`**）。

## Phase C：问卷主线 → context（`source_type = questionnaire`）→ profile

**最小纵切**（非完整问卷系统）：种子题 in-memory；用户 **`POST /api/v1/questions/answers`** 后，**question-service 不直写 profile**，仅 relay **`question.answered.v1`** 至 context；context 写入记忆的 **`source_type = questionnaire`**（与聊天 `chat` 区分），再走既有 **`profile.memory_projection.requested.v1`**。

- **BFF**：`chat/init` 聚合 pending 失败 → **`pending_questions: []`**，聊天页仍可初始化。  
- **`onboarding`**：同形 `pending_questions` 降级；**`progress`** 仅问卷域（`questions/completion`），与 **`profile/me/completion`** 无关。  
- **完成度**：问卷侧进度见 question-service / BFF `progress`；**画像**五维见 `GET /profile/me/completion`，**两者独立**。

**专用 smoke**（需 question **8086** + 已用 `QUESTION_SERVICE_BASE_URL` 启动 BFF）：

```bash
cd OneLink/repo
bash scripts/smoke-questionnaire-profile.sh
```

或：`bash scripts/local/run-chat-memory-profile-slice.sh smoke-questionnaire`

**等待异步链路**：脚本在 `POST /answers` 后对 **`profile/me` 轮询**直至出现期望 **facts**（含 `location` 等），**不再**依赖固定 `sleep`（默认最多约 60s，可通过 **`QUESTIONNAIRE_SMOKE_POLL_MAX`**、**`QUESTIONNAIRE_SMOKE_POLL_INTERVAL`** 调整）。  

**可选集成测试壳**（七服务就绪后）：

```bash
RUN_SLICE_QUESTIONNAIRE_SMOKE=1 cargo test -p integration_chat_memory_profile_slice slice_questionnaire_http_smoke -- --exact --nocapture
```

断言方向（脚本内 jq，**不绑死题干/选项文案**）：**`bff/onboarding`** 含 `user`、`pending_questions` 非空、**`progress`** 非 degraded 且含问卷计数；`chat/init` 的 `pending_questions` 非空；`POST /answers` 响应含 `answer_id` / `answered_at`；`profile/me` 出现结构化 `facts` 与 **`location` 类事实**（选择题取第一选项即可触发）；`profile/me/completion` 的 **`required_dimensions` 五维集合**与 Phase A 一致；**context** `asmr-lite` 上 **`artifact_count` 与 `summary_count`** 在仅跑问卷、不发聊天消息时仍 **> 0**（证明记忆写入经 context，而非 question 直写 profile）。以上为 **Phase C 最小纵切门禁**，**不是**完整问卷产品验收。

## 一键 Shell Smoke

依赖：`curl`、`jq`；**Phase A/B 主纵切**为六服务（不含 question）。**Phase C 问卷 smoke** 另需 question **8086** 与 BFF 指向该地址。

```bash
cd OneLink/repo
bash scripts/smoke-chat-memory-profile.sh
```

可选别名（与上完全等价，便于检索）：`bash scripts/smoke-chat-memory-profile-phase-a.sh`

编排入口（含后台启动提示）：

```bash
bash scripts/local/run-chat-memory-profile-slice.sh print-start   # 打印 cargo run 命令
bash scripts/local/run-chat-memory-profile-slice.sh start-bg       # 可选：后台起服务（含 question-service）
sleep 12
bash scripts/local/run-chat-memory-profile-slice.sh smoke
```

## 结构化画像投影 Phase A + Phase B（facts / traits / completion）

> 主实现：`profile-service` 内 `projection.rs` + `http/routes.rs`；`context-service` 仅对 `POST /internal/memory/resolve` **加法**返回原始元数据（`keywords`、`temporal_state`、`preference_polarity`、`source_message_id`），**不**判定事实类型。  
> **Phase B 边界**：`preference_polarity` 仍由 context 蒸馏并用于 L1/观测面；**profile 侧 `communication_preference` 事实与 `traits.communication_preferences` 仅来自用户文本中的显式沟通措辞**，不再把泛化 positive/negative 极性包装成沟通偏好。可信度/溯源字段为 **MVP 最小增强**，不是完整审计链。

### `GET /me` 字段分层

| 层级 | 字段 | 说明 |
|------|------|------|
| 事实层 | `facts[]` | `fact_type`：`interest` / `goal` / `location` / `communication_preference`；`value` 为启发式结果，**集成断言不要硬编码整句**。Phase B：`confidence`、`source_memory_id`（必有）；`source_message_id`（可选，依赖上游是否写入）。 |
| Trait 聚合 | `traits` | `interest_tags`、`connection_goal_tags`、`location_label`（**键始终存在**，无地点时为 `null`）、`communication_preferences`（仅显式沟通事实聚合）。 |
| 派生展示 | `headline`、`bio` | 由事实/trait 刷新；**不作为** completion 主计分。 |
| 内部态（当前不随 GET 返回） | `memory_highlights` | 实现内存字段，用于派生文案；OpenAPI 未列。 |

### `GET /me/completion` 五维（Phase A，计分口径未变）

1. `display_name` — 用户显式资料（smoke 新用户常为空）  
2. `interest_tags` — `traits.interest_tags` 非空  
3. `connection_goals` — `traits.connection_goal_tags` 非空  
4. `current_location` — `city_level_location` 非空或 `traits.location_label` 有值  
5. `communication_preferences` — `traits.communication_preferences` 非空  

`completion_rate = |filled| / 5`（再按实现舍入）。它反映的是当前结构化覆盖率，**不是**演示常数；固定 smoke 句在当前实现下通常会命中兴趣/目标/沟通三轴，但 smoke 不再把某个精确分数冻结为契约。

### 固定 smoke 句能触发什么？

脚本内固定句（见 `smoke-chat-memory-profile.sh`）：在「兴趣 + 连接目标」之外，追加 **显式沟通偏好**（示例：**「沟通上不喜欢拐弯抹角，希望直接一点。」**）。原因：Phase B 起 **`communication_preference` 不再由 `preference_polarity` 泛化生成**；若仍用纯积极/目标句而不带显式沟通措辞，`traits.communication_preferences` 可能为空，导致 smoke / completion 断言失败。

- **interest / goal**：仍由正文中的兴趣/目标类措辞映射（与 Phase A 一致）。  
- **communication_preference**：仅当正文命中 profile 内 **显式沟通启发式**（如不喜欢拐弯抹角、希望直接一点、不要推销式沟通等，见 `projection.rs`）时生成；**与** context 侧 `preference_polarity` **解耦**。  
- **location**：上述固定句**不含**地点锚点 → 通常 **`traits.location_label` 为 `null`**，且 **`current_location`** 仍可能缺失（符合 smoke 对 `missing_dimensions` 的断言）。  
- 若要手测 **location**：在单条或后续消息中使用 **已知城市名**（如上海/北京）和/或 **口语锚点**（如「我现在在…」「base在…」「常驻…」）；profile 会取 **全文最靠左** 的已知城市，或对锚点后的片段做截断抽取（见 `projection.rs`，非完整 GEO 引擎）。

### `memory/resolve` 响应（profile → context）

每条 item 除 `memory_id`、`content`、`network_type` 外，另有：`keywords[]`、`temporal_state`、`preference_polarity`、`source_message_id`（见 `repo/platform/contracts/internal/context-service.yaml`）。

### smoke 内嵌断言（Phase A + Phase B）

`smoke-chat-memory-profile.sh` 在打印 JSON 后会校验：

- `facts` 非空；`traits` 含 **键** `location_label`（允许 `null`）；兴趣/目标/沟通偏好三轴非空；至少出现一种 `fact_type`。  
- **Phase B / 可追溯性**：至少一条 fact 含 **非空** `source_memory_id`；**每条** fact 的 `confidence` 存在且 ∈ `[0,1]`（不断言具体数值）。**不**断言 `source_message_id`（依赖上游是否写入）。**不**硬编码某条 `value` 全文。  
- `completion`：`required_dimensions` 排序后与五维一致；`filled` 含 `interest_tags`、`connection_goals`、`communication_preferences`；`completion_rate` 位于 `[0,1]`，且 `filled + missing == required`。

## benchmark v1（固定跑法）

最小 benchmark v1 入口：

```bash
cd OneLink/repo
bash scripts/benchmark-asmr-lite-v1.sh
```

或使用编排脚本：

```bash
bash scripts/local/run-chat-memory-profile-slice.sh benchmark-v1
```

当前固定包含两组样本：
- **成功样本**：`我在上海做AI产品`
  - 通过条件：`artifact_count > 0`
  - 通过条件：`summary_count > 0`
  - 通过条件：`entity_count > 0`
  - 通过条件：`checkpoint_count > 0`
  - 通过条件：`routing.last_observation.executed_route == "L1"`
  - 通过条件：`routing.last_observation.candidate_route == "L1"`
  - 通过条件：成功样本写入后，后续读取 `profile/me` 时可看到投影结果继续可见
- **升级样本**：`我之前在北京，现在在上海，后来改为远程办公`
  - 通过条件：`routing.last_observation.candidate_route == "L3"`
  - 通过条件：`routing.last_observation.executed_route == "L1"`
  - 通过条件：`recent_failures[].category` 中出现 `route_escalation_deferred`
  - 通过条件：对应 failure record 的 `trace_id` 非空
  - 通过条件：执行完整脚本后 `profile.headline` 出现“记忆已同步”
  - 通过条件：`ai-chat` relay 观测面仍可读，且 `total_failures == 0`

说明：
- 这是 **benchmark v1 最小入口**，不是完整 ASMR-Lite 评测矩阵。
- 当前“失败/升级样本”以 **deferred route escalation** 为主，不要求人为打断服务制造网络故障。
- `artifact_count` / `summary_count` / `checkpoint_count` 等：在 **无 `DATABASE_URL`** 时为**进程内累计**；在 **有 `DATABASE_URL`** 时为 **DB 聚合（全表行数视角）**，重复跑全库脚本时总数可能高于单次会话预期。`total_failures` 仍为进程内。重复运行时请结合模式看 `last_observation`、`recent_failures` 与字段是否出现，而不是僵化要求总数回到某一固定值。
- 当前 `token_budget` 主要以 `memory_limit` / `summary_limit` 的 MVP 条数上限落地，不代表完整 token 级裁剪。
- `POST /internal/memory/consolidate` 若传 `artifact_ids=[]`，或传入后一个都解析不到有效 artifact，当前 MVP 返回 `accepted=false`，且**不占用** `event_id` 幂等索引；允许后续同一 `event_id` 带有效 `artifact_ids` 重试。
- 若某次 consolidate 已成功写入 summary，则同一 `event_id` 的重复请求应返回成功，但不会重复写入新 summary。
- `ai-chat-service` 的 `created_at` / `last_message_at` 当前为固定开发态时间戳，占位用于跑通纵切面；不要用它们做严格时序断言。

## benchmark v2（固定小数据集）

新增入口：

```bash
cd OneLink/repo
bash scripts/benchmark-asmr-lite-v2.sh
```

或：

```bash
bash scripts/local/run-chat-memory-profile-slice.sh benchmark-v2
```

当前固定包含：

- `Memory QA` 数据集：`repo/tests/integration/asmr_benchmark_v2/memory_qa.json`
- `Temporal & Update` 数据集：`repo/tests/integration/asmr_benchmark_v2/temporal_update.json`

runner 当前至少会打印：

- `Baseline-A`（= **Lexical-FullTranscript**，全量 setup 词法规则）
- `Baseline-B`（= **Lexical-LatestMessage**，仅末条 setup）
- `OneLink-L1`
- `candidate_route` / `executed_route`
- `memory_context` / `task_context`
- 每 case 一行 **`>>> VERDICT:`**（WIN/LOSE）

当前通过口径：

- `benchmark v2` 不覆盖 `v1`
- `OneLink-L1` 在两类数据集上都必须可跑
- `executed_route` 仍允许固定为 `L1`
- `L2/L3` 当前仍只作为 `candidate_route` 与埋点/失败样本口径出现

## benchmark v2.1（歧视样本 + entity_hits）

文档：`tests/integration/ASMR_LITE_BENCHMARK_V2.1.md`  
数据：`tests/integration/asmr_benchmark_v2_1/*.json`

```bash
cd OneLink/repo
bash scripts/benchmark-asmr-lite-v2.1.sh
```

或：

```bash
bash scripts/local/run-chat-memory-profile-slice.sh benchmark-v2.1
```

用途简述：

- 至少一组样本要求 **OneLink-L1 通过且两路 lexical scaffold 按设计失败**（避免「全员答对」）。
- 至少一组样本 **断言 `routing.last_observation.entity_hits >= 1`**（与当前 L1 entity-aware 实现一致）。
- **`query_polarity_open.json`**：断言 **`query_preference_polarity`**（开放式问句为 `neutral`，「是否不喜欢…」为 `negative`），与 `l1_policy::query_preference_polarity` 对齐。

## Rust 测试壳

```bash
cd OneLink/repo
cargo test -p integration_chat_memory_profile_slice
```

在六服务已就绪时（**Phase A/B** chat 纵切，`smoke-chat-memory-profile.sh`）：

```bash
RUN_SLICE_HTTP_SMOKE=1 cargo test -p integration_chat_memory_profile_slice slice_http_smoke -- --exact --nocapture
```

在 **七服务**已就绪时（含 **question-service 8086** 与 BFF `QUESTION_SERVICE_BASE_URL`，**Phase C** `smoke-questionnaire-profile.sh`）：

```bash
RUN_SLICE_QUESTIONNAIRE_SMOKE=1 cargo test -p integration_chat_memory_profile_slice slice_questionnaire_http_smoke -- --exact --nocapture
```

## 日志与后续 Benchmark 接入

- 主链相关 crate 使用 `tracing`；建议 `RUST_LOG=info`（或 `debug`）启动。
- `context-service` 额外提供 `GET /internal/observability/asmr-lite`：`routing` / `recent_failures` 为当前进程内观测；`artifact_count` 等与 `DATABASE_URL` 是否存在时的读数来源见上文 **§8** 与 `context-service.yaml`（双模式）。**`context_logs` 行不在该响应中暴露**；Postgres 模式下由 **`POST /internal/context/build`** 侧路写入，验证见上文 **`context_logs` 写入门禁** 节。
- `ai-chat-service` 额外提供 `GET /internal/observability/chat-relay`，用于查看 `chat.user_message.created.v1 -> context-service` 的最小失败记录。
- `profile-service /internal/events/receive` 在 `memory/resolve` 失败时会返回非 2xx，让 `context -> profile` relay 可以重试或记录失败，不再把失败伪装成接受成功。
- `profile-service` 的 Bearer 校验错误语义已与 `ai-chat-service` 对齐：identity `5xx` / 不可达返回 `502`，无效或过期 token 返回 `401`。
- 冻结事件名（用于 grep / 未来 benchmark 埋点对齐）：  
  `chat.user_message.created.v1`、`context.memory.extracted.v1`、`context.memory.summary.updated.v1`、`profile.memory_projection.requested.v1`  
  完整清单与验收见：`OneLink/docs/archive/rules-legacy-2026-05-15/Rules-V2/EXECUTION/asmr-lite-benchmark-and-acceptance-checklist.md`。  
- **当前仓库脚本不运行完整 ASMR-Lite benchmark**；后续可在同一 smoke 之后叠加 checklist 中的用例与指标采集，无需改事件 schema。

## OpenAPI

本轮对外契约：`platform/contracts/openapi/identity-service.yaml`、`profile-service.yaml`、`bff.yaml`、`ai-chat-service.yaml`、**`question-service.yaml`**（与 `OneLink/docs/archive/rules-legacy-2026-05-15/Rules/15` 字段名对齐）。  
**`/internal/*` 服务间路径** 不放入公开 OpenAPI，避免误当作前端 API；细节见各服务 README。

## 已知契约/实现尾差（若存在）

- `repo/data-platform/event-schemas/question.answered.v1.json` 中部分 id 字段标注 `format: uuid`，若 identity 在 dev 下返回非 UUID 形 `user_id`，与 schema 严格校验器可能不完全一致；**运行态以 Rust 实现与 OpenAPI 描述为准**。
- `OneLink/docs/archive/rules-legacy-2026-05-15/Rules/15` 部分章节描述通用响应 **envelope**（`data` / `error`）；当前 Rust MVP 对上述路径多返回 **裸 JSON 体**。OpenAPI 已按**实际响应体**描述；统一 envelope 需单独迭代（交 GPT 5.4 / 主实现）。
- **`memory_highlights`**：Profile 内部用于派生 `headline`/`bio` 等，**当前** `GET /api/v1/profile/me` **不**返回该字段；文档与 `profile-service.yaml` 已按此收敛。若产品要对外展示要点列表，需主实现增字段后再改 OpenAPI / smoke。
- **`ProfileTraitSnapshot.location_label`**：OpenAPI 中为 **required + nullable**（无地点时 JSON `null`），与实现一致。
