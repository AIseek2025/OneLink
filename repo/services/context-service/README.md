# context-service

**Memory Compute Layer**（记忆计算层）— MVP 骨架，现已具备最小 ASMR-Lite 闭环：异步结构化提取、summary/consolidation、L1 检索、路由埋点与失败样本观测。

## 定位（冻结口径）

- **负责**：memory extraction、memory distillation、context assembly。
- **不负责**：原始聊天主存、画像最终写入、推荐结果、社交关系主存。

## Memory persistence（`DATABASE_URL`）

实现为**双模式**（与 `OneLink/docs/archive/rules-legacy-2026-05-15/Rules-V2/EXECUTION/composer-2-memory-persistence-brief.md` 一致）：

| 条件 | 行为 |
|------|------|
| **未设置**或**空** `DATABASE_URL` | **In-memory fallback**：核心记忆与 checkpoint 仅存进程内 `HashMap`；**重启即丢**。旧 smoke / benchmark（无 DB）仍走此路径。 |
| **已设置** `DATABASE_URL` | **Postgres**：对下列对象做真实读写（`tokio-postgres` + `deadpool-postgres` 连接池）；**重启后可恢复**（表须事先用草案 SQL 建好，**服务内不做 auto-migrate**）。 |

**已在 Postgres 路径持久化（对齐 `repo/data-platform/db-schema/drafts/003_context.sql`）**

- `memory_artifacts`、`memory_summaries`、`memory_entities`、`memory_entity_links`
- `agent_runtime_checkpoints`
- **`context_logs`**：每次 **`POST /internal/context/build`** 在**成功返回 200 前**尝试 **INSERT 一行**（`user_id`、`conversation_id`（空串落库为 `NULL`）、`selected_summary_ids` / `selected_memory_ids`（`UUID[]`）、`retrieval_modes`（`TEXT[]`）、`task_type`、**`token_budget_json`**（由请求体 **`max_tokens` / `memory_limit` / `summary_limit`** 组 JSON，满足列 **`NOT NULL`**）、`input_ref_id` / `model_context_size` 当前为 `NULL`）。插入失败仅 **`tracing::warn`**，**不改变** HTTP 状态码与 `ContextBuildResponse`（best-effort）。
- 幂等辅助表（`repo/data-platform/db-schema/drafts/003_context_idempotency.sql`）：`context_checkpoint_dedupe`、`context_memory_consolidate_dedupe`（保证与原先进程内 `checkpoint_request_index` / `consolidate_event_index` 等价语义）

Rust 侧记录与 SQL 列**并非一一字面同构**时：多出的运行字段写入 **`content_structured` / `key_points_json` / `source_message_range`** 等 JSONB；`memory_entity_links` 在库中为 `source_entity_id`/`target_entity_id`，实现侧用 **`_memory_pointer` 占位实体** + 真实实体做最小映射（见主实现 `store/postgres.rs`），**不**因此扩展新业务语义。

**Routing / failure 观测：进程内聚合 + 可选 DB 快照（`011_runtime_observability.sql`）**

- **`routing.total_requests` / `l1_requests` / `l2_candidates` / `l3_candidates` / `degraded_requests` / `total_conflicts`**：无论是否 Postgres，均为 **当前进程** Mutex 累加，**重启归零**（未做跨重启持久化）。
- **`context_routing_observations` / `context_failure_events`**：在 **已设置 `DATABASE_URL`** 且已执行 **`repo/data-platform/db-schema/drafts/011_runtime_observability.sql`** 时，build 路径 **best-effort** 写入快照行；**`GET /internal/observability/asmr-lite`** 在 Postgres 且能读到最新快照时，用 DB **覆盖**响应中的 **`routing.last_observation`**、**`recent_failures`**、根级 **`total_failures`**（跨重启可读**最近一次**快照视角）。**无 011 或未配置 DB** 时上述字段仍为进程内池。

**`forgetting_decisions`（在线写入）**

- **`POST /internal/memory/forgetting/decide`**：Postgres 模式下 **INSERT** 一行（列与 `003_context.sql` 对齐，含 **`reason_codes` / `policy_version` / `cold_storage_ref`** 等）。**无 `DATABASE_URL`** 时返回 **`accepted: false`**、**`persistence: noop_no_database`**（不写行）。

**`DATABASE_URL` 缺失（in-memory fallback）与 `context_logs`**

- **不写** `context_logs`：无进程内模拟表；观测仍以 **`GET /internal/observability/asmr-lite`** 为准。

**`policy_version` / `retrieval_modes` 观测口径（不回退）**

- 写入 summary 的 **`policy_version`** 仍与 **`PolicyConfigStore::policy_version_label`** 对齐。
- **`GET /internal/observability/asmr-lite`** 暴露 **`policy_version_label`**、**`latest_summary_policy_version`**、**`routing.last_observation.retrieval_modes`**（与 `context-service.yaml` 一致）。
- **计数语义**：**`artifact_count` / `summary_count` / `entity_count` / `link_count` / `checkpoint_count`** 在 Postgres 模式下为 **DB 聚合**，否则为进程内 HashMap。**`routing` 聚合计数**见上（始终进程内）；**`last_observation` / `recent_failures` / `total_failures`** 在 Postgres+011 下可取 DB 快照。

**Policy 表：只读、非主写；`graph` / `rerank` 由策略开关驱动**

- **`policy_configs` 等**仍为 **optimization-layer** 主写边界；本服务 **不**主写 `policy_*`。
- **启动时**：若 **`policy_configs`** 表存在且 SELECT 可达，**只读**合并 **`graph_enabled` / `rerank_enabled` / `enabled_retrieval_modes`**、策略版本标签等到 **`PolicyConfigStore`**；**表不存在或探测失败**则回落内置默认并 **不**因缺表崩溃。
- **`enabled_retrieval_modes` 的 `current_value`（与 `store/postgres.rs` 一致）**：优先按 **JSON 字符串数组**解析（如 `["structured","graph"]`）；**空数组 `[]` 视为无有效覆盖**（**不**合并进进程内允许列表，与「未配置该键」等价），**不会**再退回到按逗号拆字面量。非 JSON 时按逗号/空白拆分为模式名；拆完仍为空则忽略。**`graph_enabled` / `rerank_enabled`**：仅接受宽松布尔字面（`true`/`false`/`1`/`0`/`yes`/`on` 等），无法解析则**跳过该键**（保留合并前的默认值）。
- **`graph`（真实图扩展）**：在策略允许时，于 L1 首轮候选之后，从命中实体沿 **`memory_entity_links`** 遍历邻居，将 **原先不在首轮候选集内** 的相关 artifact/summary **拉入**候选再评分（区别于仅对已有候选做 entity-link 加权）。
- **`rerank`（真实二阶段）**：在策略允许时，在证据收集后调用 **独立** `rerank_second_pass`，使用首轮 **`score_match` 未使用** 的重排信号；与「在同一函数里微调公式」区分。

**跨重启验证（可选，依赖 DB）**

```bash
cd OneLink/repo
export DATABASE_URL='postgres://...'   # 且已执行 001_identity、003_context、003_context_idempotency
bash scripts/smoke-chat-memory-persistence.sh
# 或：bash scripts/local/run-chat-memory-profile-slice.sh smoke-persistence
```

**Routing / failure 跨重启 asmr-lite（依赖 DB + 011）**

```bash
cd OneLink/repo
export DATABASE_URL='postgres://...'   # 001_identity、003_context、011_runtime_observability.sql
bash scripts/smoke-chat-memory-runtime-observability.sh
# 或：bash scripts/local/run-chat-memory-profile-slice.sh smoke-runtime-obs
```

**`forgetting_decisions` 写入门禁（依赖 DB + users FK）**

```bash
cd OneLink/repo
export DATABASE_URL='postgres://...'   # 001_identity、003_context（含 forgetting_decisions）
bash scripts/smoke-chat-memory-forgetting-decisions.sh
# 或：bash scripts/local/run-chat-memory-profile-slice.sh smoke-forgetting
```

**`context_logs` 写入门禁（依赖 DB，不断言跨重启）**

```bash
cd OneLink/repo
export DATABASE_URL='postgres://...'   # 且已执行 001_identity、003_context（含 context_logs）
bash scripts/smoke-chat-memory-context-log.sh
# 或：bash scripts/local/run-chat-memory-profile-slice.sh smoke-context-log
```

脚本会 **`INSERT users … ON CONFLICT DO NOTHING`**、检查 **`context_logs` 表存在**、真实 **`POST /internal/context/build`**，并用 `psql` 断言该 `user_id` 下行数增加；**不**将「kill 再起后仍能查到该行」作为硬性门禁（与 persistence smoke 分工不同）。

详见 `scripts/README.md` 与 `tests/integration/CHAT_MEMORY_PROFILE_SLICE.md`。

## Activation Scoring（Elo-like Phase 1）

- `importance_score` 现在在写入 `memory_artifacts` 时就会赋初值：
  - `questionnaire` -> `0.9`
  - `supersedes_previous = true` -> `0.8`
  - `chat` 且 `confidence >= 0.7` -> `0.7`
  - `chat` 且 `confidence < 0.7` -> `0.5`
  - `behavior` -> `0.6`
  - 其它 -> `importance_score_default`
- `last_accessed_at` / `access_count` 属于 `memory_artifacts` 的动态状态字段；在 **Postgres 模式**下，`POST /internal/context/build` 命中 artifact 后会 **best-effort** 更新它们，失败只 `warn`，不改变 HTTP 返回。
- `score_match` 仍以既有词法 / 意图 / 极性 / 实体信号为主，activation 只是**附加加分项**：

```text
activation_factor =
  importance_score
  * ((1 + elapsed_hours) ^ (-activation_decay_rate))
  * log2(2 + access_count)
```

- 当前策略参数：
  - `activation_decay_rate`：默认 `0.3`
  - `score_activation_weight`：默认 `0.12`
  - `importance_score_default`：默认 `0.5`
- in-memory fallback 也保留这些字段，但不会做数据库写回。

### Activation smoke（依赖 Postgres）

```bash
cd OneLink/repo
export DATABASE_URL='postgres://...'
bash scripts/smoke-chat-memory-activation.sh
# 或：bash scripts/local/run-chat-memory-profile-slice.sh smoke-activation
```

## 内部逻辑模块（单进程）

MVP 阶段以下模块仅作为本进程内 `module`/crate 组织，**不得**拆成独立微服务：

- memory-extractor  
- memory-distiller  
- context-builder  
- memory-store  
- vector-index  
- task-router（logical only）

## 契约与数据

| 类型 | 路径 |
|------|------|
| 内部 OpenAPI | `repo/platform/contracts/internal/context-service.yaml` |
| SQL 草案 | `repo/data-platform/db-schema/drafts/003_context.sql`、**`003_context_activation.sql`**（Phase 1 activation 扩展）、**`011_runtime_observability.sql`**（routing/failure 观测快照，可选）、**`010_optimization.sql`**（`policy_*` 占位，§7 第 10 步） |
| 事件 schema | `repo/data-platform/event-schemas/context.memory.*.v1.json`、`profile.memory_projection.requested.v1.json` |

当前规范裁决顺序见 `OneLink/rules/10-MIGRATION-NOTES.md` §3：运行代码与测试 > `repo/platform/contracts/` > `repo/data-platform/` > `OneLink/rules/` > `OneLink/docs/archive/`。

规则历史来源：`OneLink/docs/archive/rules-legacy-2026-05-15/Rules/19`、`11`、`14`、`15`、`16`、`18`，以及 `OneLink/docs/archive/rules-legacy-2026-05-15/Rules-V2/DATA/data-model.md`、`OneLink/docs/archive/rules-legacy-2026-05-15/Rules-V2/CONTRACTS/context-service-contract.md`。以上 archive 资料仅用于追溯字段来源与演化背景，当前实现与现行规则以 `OneLink/rules/`、`repo/platform/contracts/`、`repo/data-platform/` 和代码为准。

### 持久化草案字段（本服务 owner 表，与 `003_context.sql` / Rules/11、14 对齐）

- **`memory_summaries`**：除 `summary_type`、`summary_text`、`key_points_json`、`source_message_range`、`token_count`、`updated_at` 外，草案含可选 **`policy_version`**（生成该摘要时生效的策略版本标签，便于重放与对账）。摘要写入路径会填入与 **`PolicyConfigStore::policy_version_label`** 一致的单值（当前为 `memory_policy_version`）；**若启用 `DATABASE_URL`**，该字段随行写入 Postgres；否则仅存进程内。以契约与 SQL 草案为准。
- **`context_logs`**：草案含 **`retrieval_modes`**（`TEXT[]`）等列。每次 **`/internal/context/build`** 仍将策略过滤后的模式记入 **`routing.last_observation.retrieval_modes`**（`GET /internal/observability/asmr-lite`）。**此外**，在 **Postgres 模式**下同一请求会 **best-effort 追加一行 `context_logs`**（字段与 `003_context.sql` 对齐；**`token_budget_json` 非空**，来自 build 请求体）。**In-memory 模式不写 `context_logs` 行**。

### `POST /internal/context/build` 与 `retrieval_modes`

- 请求体中的 **`retrieval_modes`**：常见为 **`structured`、`semantic`、`temporal`**；在 **`PolicyConfigStore`** 允许时还可包含 **`graph`、`rerank`**（与 `enabled_retrieval_modes` / 进程默认对齐）。调用方声明的模式经策略过滤后进入 **`retrieval_used`**；未允许的模式丢弃并可能 **`degraded: true`**。
- 契约说明见 `context-service.yaml` 中 `ContextBuildRequest.retrieval_modes` 的 `description`；详述见 `OneLink/docs/archive/rules-legacy-2026-05-15/Rules/15-MVP-OPENAPI-DRAFT.md` §9.1。
- 降级时须 **`degraded: true`**（与 `OneLink/docs/archive/rules-legacy-2026-05-15/Rules-V2/CONTRACTS/context-service-contract.md` §4.4 一致）。

### Runtime baseline hardening（证明抓手，非新平台能力）

`Runtime Baseline Hardening` 波次在叙述层视为**已通过终验**；角色任务书已标为历史归档，见 `OneLink/docs/archive/rules-legacy-2026-05-15/Rules-V2/EXECUTION/runtime-baseline-hardening-dispatch-sheet.md` 顶部说明。下列验证抓手仍描述**当前 repo 中的可跑证明**（不扩展产品边界），与历史任务书 `OneLink/docs/archive/rules-legacy-2026-05-15/Rules-V2/EXECUTION/composer-2-runtime-baseline-hardening-brief.md` 对齐：

- **crate 单测**（`cargo test -p context-service`）：**进程内 routing 累加**、**in-memory 无 DB 快照**、`PolicyConfigStore` 合并与 **`filter_retrieval_modes`**、**`policy_configs` 值解析**（含 `[]` / 非法布尔）、**graph 拉入非首轮候选**、**rerank second-pass 可改序**。
- **集成测**：`tests/forgetting_decide_no_db.rs` — 无 **`DATABASE_URL`** 时 **`POST /internal/memory/forgetting/decide`** → **`noop_no_database`**。
- **Shell 门禁脚本**（仍与旧入口并列）：**`smoke-chat-memory-runtime-observability.sh`**、**`smoke-chat-memory-forgetting-decisions.sh`**、**`benchmark-asmr-lite-v2.2.sh`** 约定退出码：**0** 成功；**1** 实现/断言失败；**2** 环境或 SQL 前置缺失（便于区分「缺库/缺表」与「实现坏了」）。

### Policy Config Store（只读边界）

- **`policy_configs` / `policy_experiments` / `policy_rollouts`** 的 **owner 为 `optimization-layer`**，见 `OneLink/docs/archive/rules-legacy-2026-05-15/Rules/14` §3.3A、`OneLink/docs/archive/rules-legacy-2026-05-15/Rules-V2/DATA/data-model.md` §5、`OneLink/docs/archive/rules-legacy-2026-05-15/Rules/17` §2.2A。
- **本服务（context-service）**：**启动时**若 `policy_configs` 表存在则 **只读**合并到进程内 store；否则仅用内置默认。**永不**主写 `policy_*`。
- **禁止**：把本服务写成 Policy 主写方；**`AutoResearch` 不得**修改 DDL / OpenAPI / 事件 schema / 宪法（`OneLink/docs/archive/rules-legacy-2026-05-15/Rules-V2/00-CONSTITUTION.md` 铁律 7）。可调参数须先进入 Policy Config Store 注册后，才由优化层变更。
- **`policy_experiments` / `policy_rollouts`**：DDL 占位仍在 **`010_optimization.sql`**；**本轮**不在 **`context-service` 在线主链**上消费或写入，与 **`policy_configs` 启动只读**分工不同。

> **说明**：`policy_*` DDL 与 `OneLink/docs/archive/rules-legacy-2026-05-15/Rules/14` §3.3A 一致，已独立落盘为 **`repo/data-platform/db-schema/drafts/010_optimization.sql`**；全库建表顺序见 **`drafts/README.md`** 与 `OneLink/docs/archive/rules-legacy-2026-05-15/Rules/14` §7（在 `009_model_gateway.sql` **之前**应用）。

## `/internal/context/build` 降级

向量检索超时或 Qdrant 不可用时，须降级为 **working memory + 最近 N 条 memory_summaries**，跳过 persistent 向量召回（详见 OneLink/docs/archive/rules-legacy-2026-05-15/Rules/19）。

## V2 最小实现状态

当前骨架已补入：

- `POST /internal/context/build`
- `POST /internal/session/checkpoint`
- `POST /internal/memory/write`
- `GET /internal/memory/search`
- `POST /internal/memory/consolidate`
- `POST /internal/memory/resolve`（profile 解析记忆工件；响应除 `memory_id`/`content`/`network_type` 外，Phase A 起附带 **`keywords`**、**`temporal_state`**、**`preference_polarity`**、**`source_message_id`**，供 profile **启发式映射**使用；**不在此接口判定** `fact_type`）  
  - **与 profile Phase B 对齐**：`preference_polarity` 仍用于本服务 L1/观测面；**profile-service 不把极性直接映射为 `communication_preference`**，沟通偏好仅来自用户文本中的 **显式**沟通措辞（见 `profile-service` `projection.rs`）。
- `GET /internal/observability/asmr-lite`（路由计数、失败样本、artifact/summary/entity 数；Postgres+011 下部分 routing 字段可取 DB 快照）
- `POST /internal/memory/forgetting/decide`（Postgres 下写入 **`forgetting_decisions`**；无 DB 则 noop）
- `POST /internal/events/receive`（dev-only envelope relay，消费 `chat.user_message.created.v1`、**`question.answered.v1`** 等）

**内部鉴权（与实现一致，dev-only）**  
以下路径在 handler 内统一校验 **`x-internal-token`** == **`INTERNAL_SHARED_SECRET`**（默认 **`onelink-dev-internal-token`**，须与 **ai-chat、profile** 一致）：

- `POST /internal/context/build`
- `POST /internal/session/checkpoint`
- `POST /internal/memory/write`
- `GET /internal/memory/search`
- `POST /internal/memory/consolidate`
- `POST /internal/events/receive`
- `POST /internal/memory/resolve`
- `POST /internal/memory/forgetting/decide`
- `GET /internal/observability/asmr-lite`

出站调用（实现已接线）：向 **ai-chat** 拉消息、向 **profile** 投递事件时，会携带 **`x-internal-token`**。  
入站调用：`ai-chat -> context` 的 `POST /internal/context/build` 也按同一口令保护。

## 当前 ASMR-Lite 最小能力

- **异步结构化提取**：`chat.user_message.created.v1` 进入后，先拉用户正文，再生成 `memory_artifacts` 风格的内存态对象。`memory_extractor::heuristic_extract` 对含分号的长句会**额外**切出与 profile Phase B 对齐的**显式沟通偏好子句**（如「沟通上不喜欢拐弯抹角，希望直接一点」），避免仅产出短摘要 artifact 时 profile 侧无法映射 `communication_preference`。
- **问卷（Phase C）**：`question.answered.v1` 由 **question-service** relay；本服务将抽取文本写入工件时标记 **`source_type = questionnaire`**（`summary_type` 同为 `questionnaire`），再走与聊天一致的 summary / **`profile.memory_projection.requested.v1`** relay。**question-service 不直写 profile**。  
  - 前端冷启动拉问卷进度可走 **BFF `GET /api/v1/bff/onboarding`**（聚合 identity + pending + **问卷域** `progress`）；与 **聊天进线**用的 **`GET /api/v1/bff/chat/init`**（多一个 ai-chat 会话）分流，**不**改变本服务消费事件链。
- **consolidation / summary**：同一条用户消息会同步形成一条 `working_memory` summary，并把关键词、实体、链接、时间态写入 **store**（Postgres 或进程内）；`POST /internal/memory/consolidate` 也会真正写入 summary。
- **L1 检索**：`/internal/context/build` 与 `/internal/memory/search` 走确定性匹配，不额外调用 LLM；首轮参考关键词、实体与时间态等 rule-first 信号；策略允许 **`graph`** 时做 **沿 `memory_entity_links` 的邻居扩展**，把**非首轮候选** artifact/summary 拉入；允许 **`rerank`** 时经 **`rerank_second_pass`** 二阶段重排。`preference_polarity` 写入 artifact/summary 后参与排序与 `memory_context` / observability。
- **路由埋点**：当前只执行 `L1`，但会记录 `candidate_route`、升级原因、`upgraded`、证据数、`summary_hits` / `artifact_hits` / `entity_hits`、冲突数、近似 `route_confidence`、`estimated_llm_calls`、`estimated_tokens`、`query_preview`、**`retrieval_modes`**（与本次 build 的 **`retrieval_used`** 一致）、`query_preference_polarity`、`evidence_preference_polarity` 与耗时；若上游传入 `trace_id`，`route_escalation_deferred` 等 failure sample 会带上同一 `trace_id`。
- **失败样本池**：当 message fetch、事件归属对账、projection relay、路由升级延后等情况发生时，会写入 in-memory failure cases，并记录 `stage / trace_id / retryable / attempt_count`。
- **最小真实 internal 行为**：`POST /internal/session/checkpoint` 会写入 checkpoint（Postgres 或进程内），并对相同请求体按 canonical JSON 生成稳定 dedupe key（DB 模式下由 `context_checkpoint_dedupe` 保证跨重启幂等）。`POST /internal/memory/write` 会直接生成 artifact + summary。
- **consolidate 边界**：`POST /internal/memory/consolidate` 在 `artifact_ids=[]` 或传入后一个都解析不到有效 artifact 时返回 `accepted=false`，且不占用 `event_id` 幂等索引，允许后续同一 `event_id` 带有效 `artifact_ids` 重试。
- **consolidate 成功重放**：若某次已成功写入 summary，则同一 `event_id` 的重复请求返回成功，但不会重复写入新的 summary。
- **事件生产口径**：`profile.memory_projection.requested.v1` 会真实通过 HTTP relay 投递；payload 可选带 **`projection_inputs_version`**（与 `OneLink/docs/archive/rules-legacy-2026-05-15/Rules/16`、`repo/data-platform/event-schemas/profile.memory_projection.requested.v1.json` 对齐，供 resolve/投影启发式演进对账）。`context.memory.extracted.v1` 与 `context.memory.summary.updated.v1` 当前仍以日志级自记录为主。

### `preference_polarity`（与实现对齐，勿过度解读）

- **来源**：`memory_distiller` 对用户消息做 **启发式** 极性分类，取值 **`positive` | `negative` | `neutral`**（与 `l1_policy` 中偏好 marker 一致）。
- **落库**：写入每条 **artifact** 与对应 **summary**；`POST /internal/memory/consolidate` 对选中 artifact 做合并（**negative 优先于 positive 优于 neutral**）。
- **用途（MVP）**：参与 L1 `score_match`（查询侧由 `query_preference_polarity` 推断后与证据极性对齐/互斥微调）；出现在 **`memory_context`**（`query_polarity_hint` / `pref_top`）、**`task_context`**、**`routing.last_observation`**（`query_preference_polarity` / `evidence_preference_polarity`）、**`GET /internal/memory/search`** 的 `items[].preference_polarity`。
- **不是**：细粒度情感分析、稳定跨语言 NLI、或已接入学习的用户长期偏好模型。

#### `query_preference_polarity`（查询侧，与 `preference_polarity` 区分）

- **仅**在查询命中 `l1_policy` 中偏好相关意图时，才用查询文本做 **query-side** 极性（供 `score_match` 与埋点）；未命中时实现仍写入 **`neutral`**（与 `Option` 在 `task_context` 中展开一致）。
- **开放式偏好问句**（实现：`is_preference_open_question`，如含「什么样」「哪种」「喜欢什么…」等）→ **`neutral`**，避免把「用户喜欢什么样的沟通方式？」误判成 query 自己在表达 **`positive`**。
- **嵌入立场**（如「是否不喜欢 X」）→ 可保留 **`negative` / `positive`**，与证据上的 `preference_polarity` 对齐加分。
- **排查**：`l1_policy::query_mentions_preference_intent` 可用于扫 benchmark 中哪些 query 会进入偏好意图分支。
- **验收**：`repo/tests/integration/asmr_benchmark_v2_1/query_polarity_open.json` + `scripts/benchmark-asmr-lite-v2.1.sh`。

## 运行（本地）

```bash
cd OneLink/repo
export RUST_LOG=info
export INTERNAL_SHARED_SECRET="${INTERNAL_SHARED_SECRET:-onelink-dev-internal-token}"
export AI_CHAT_SERVICE_BASE_URL="${AI_CHAT_SERVICE_BASE_URL:-http://127.0.0.1:8085}"
export PROFILE_SERVICE_BASE_URL="${PROFILE_SERVICE_BASE_URL:-http://127.0.0.1:8082}"
# 可选：export DATABASE_URL='postgres://...'   # 启用持久化（须已建表，见上文）
PORT=8089 cargo run -p context-service
```

- `GET /health`
- 上述 internal 路由（见代码与 `repo/platform/contracts/internal/context-service.yaml` 草案）

观察 ASMR-Lite 最小指标：

```bash
curl -sS "http://127.0.0.1:8089/internal/observability/asmr-lite" \
  -H "x-internal-token: ${INTERNAL_SHARED_SECRET:-onelink-dev-internal-token}"
```

返回中可重点关注：
- `checkpoint_count`
- `routing.last_observation`（含 **`retrieval_modes`**）
- **`policy_version_label`**（与写入 summary 的 **`policy_version`** 同源单值）
- **`latest_summary_policy_version`**（有 summary 时应对齐上述标签；无 summary 时为 `null`）
- `recent_failures[*].stage`
- `recent_failures[*].retryable`
- `recent_failures[*].attempt_count`

最小 benchmark v2 可直接运行：

```bash
cd OneLink/repo
bash scripts/benchmark-asmr-lite-v2.sh
```

**benchmark v2.1**（歧视性样本 + `entity_hits` 断言，可与 Opus 验收对齐）：

```bash
bash scripts/benchmark-asmr-lite-v2.1.sh
```

说明见 `repo/tests/integration/ASMR_LITE_BENCHMARK_V2.1.md`。

**benchmark v2.2**（Postgres + **`policy_configs`**：`graph`/`rerank` 进入 **`retrieval_used`** 的门禁脚本，**不**替代 v2 / v2.1）：

```bash
cd OneLink/repo
export DATABASE_URL='postgres://...'   # 001_identity、003_context、010_optimization.sql
bash scripts/benchmark-asmr-lite-v2.2.sh
```

或：`bash scripts/local/run-chat-memory-profile-slice.sh benchmark-v2.2`

`benchmark v2` / `v2.1` 当前冻结：

- 数据集：v2 为 `Memory QA`、`Temporal & Update`；v2.1 为 `asmr_benchmark_v2_1/*`
- baseline：**Baseline-A** = Lexical-FullTranscript，**Baseline-B** = Lexical-LatestMessage（本地 shell scaffold），**OneLink-L1** = 本服务 `context/build`
- runner 输出：route、memory/task context、查询级 observability 字段；v2/v2.1 均打印 **`>>> VERDICT:`**（WIN/LOSE）
- 边界：`executed_route` 仍固定为 `L1`，`L2/L3` 仍只作为 candidate 与埋点口径

最小 benchmark v1 可直接运行：

```bash
cd OneLink/repo
bash scripts/benchmark-asmr-lite-v1.sh
```

## 技术栈

与 workspace 一致：`tokio`、`axum`、`tracing`、`serde`（见根 `Cargo.toml`）。
