# context-service

**Memory Compute Layer**（记忆计算层）— MVP 骨架，现已具备最小 ASMR-Lite 闭环：异步结构化提取、summary/consolidation、L1 检索、路由埋点与失败样本观测。

## 定位（冻结口径）

- **负责**：memory extraction、memory distillation、context assembly。
- **不负责**：原始聊天主存、画像最终写入、推荐结果、社交关系主存。

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
| SQL 草案 | `repo/data-platform/db-schema/drafts/003_context.sql` |
| 事件 schema | `repo/data-platform/event-schemas/context.memory.*.v1.json`、`profile.memory_projection.requested.v1.json` |

规则来源：`OneLink/Rules/19`、`11`、`14`、`15`、`16`、`18`。

## `/internal/context/build` 降级

向量检索超时或 Qdrant 不可用时，须降级为 **working memory + 最近 N 条 memory_summaries**，跳过 persistent 向量召回（详见 Rules/19）。

## V2 最小实现状态

当前骨架已补入：

- `POST /internal/context/build`
- `POST /internal/session/checkpoint`
- `POST /internal/memory/write`
- `GET /internal/memory/search`
- `POST /internal/memory/consolidate`
- `POST /internal/memory/resolve`（profile 解析记忆工件；响应除 `memory_id`/`content`/`network_type` 外，Phase A 起附带 **`keywords`**、**`temporal_state`**、**`preference_polarity`**、**`source_message_id`**，供 profile **启发式映射**使用；**不在此接口判定** `fact_type`）  
  - **与 profile Phase B 对齐**：`preference_polarity` 仍用于本服务 L1/观测面；**profile-service 不把极性直接映射为 `communication_preference`**，沟通偏好仅来自用户文本中的 **显式**沟通措辞（见 `profile-service` `projection.rs`）。
- `GET /internal/observability/asmr-lite`（查看路由计数、失败样本、artifact/summary/entity 数）
- `POST /internal/events/receive`（dev-only envelope relay，消费 `chat.user_message.created.v1` 等）

**内部鉴权（与实现一致，dev-only）**  
以下路径在 handler 内统一校验 **`x-internal-token`** == **`INTERNAL_SHARED_SECRET`**（默认 **`onelink-dev-internal-token`**，须与 **ai-chat、profile** 一致）：

- `POST /internal/context/build`
- `POST /internal/session/checkpoint`
- `POST /internal/memory/write`
- `GET /internal/memory/search`
- `POST /internal/memory/consolidate`
- `POST /internal/events/receive`
- `POST /internal/memory/resolve`
- `GET /internal/observability/asmr-lite`

出站调用（实现已接线）：向 **ai-chat** 拉消息、向 **profile** 投递事件时，会携带 **`x-internal-token`**。  
入站调用：`ai-chat -> context` 的 `POST /internal/context/build` 也按同一口令保护。

## 当前 ASMR-Lite 最小能力

- **异步结构化提取**：`chat.user_message.created.v1` 进入后，先拉用户正文，再生成 `memory_artifacts` 风格的内存态对象。`memory_extractor::heuristic_extract` 对含分号的长句会**额外**切出与 profile Phase B 对齐的**显式沟通偏好子句**（如「沟通上不喜欢拐弯抹角，希望直接一点」），避免仅产出短摘要 artifact 时 profile 侧无法映射 `communication_preference`。
- **consolidation / summary**：同一条用户消息会同步形成一条 `working_memory` summary，并把关键词、实体、链接、时间态保存在进程内；`POST /internal/memory/consolidate` 也会真正写入 summary。
- **L1 检索**：`/internal/context/build` 与 `/internal/memory/search` 走确定性匹配，不额外调用 LLM；当前会参考关键词、**实体链接**（含 summary 经 `memory_ids` 间接命中）、时间态、更新线索、偏好/地点/连接目标等 rule-first 信号；`preference_polarity` 写入 artifact/summary 后参与排序与 `memory_context` / observability。
- **路由埋点**：当前只执行 `L1`，但会记录 `candidate_route`、升级原因、`upgraded`、证据数、`summary_hits` / `artifact_hits` / `entity_hits`、冲突数、近似 `route_confidence`、`estimated_llm_calls`、`estimated_tokens`、`query_preview`、`query_preference_polarity`、`evidence_preference_polarity` 与耗时；若上游传入 `trace_id`，`route_escalation_deferred` 等 failure sample 会带上同一 `trace_id`。
- **失败样本池**：当 message fetch、事件归属对账、projection relay、路由升级延后等情况发生时，会写入 in-memory failure cases，并记录 `stage / trace_id / retryable / attempt_count`。
- **最小真实 internal 行为**：`POST /internal/session/checkpoint` 会写入 in-memory checkpoint，并对相同请求体按 canonical JSON 生成稳定 dedupe key；`POST /internal/memory/write` 会直接生成 artifact + summary，而不再只是占位返回。
- **consolidate 边界**：`POST /internal/memory/consolidate` 在 `artifact_ids=[]` 或传入后一个都解析不到有效 artifact 时返回 `accepted=false`，且不占用 `event_id` 幂等索引，允许后续同一 `event_id` 带有效 `artifact_ids` 重试。
- **consolidate 成功重放**：若某次已成功写入 summary，则同一 `event_id` 的重复请求返回成功，但不会重复写入新的 summary。
- **事件生产口径**：`profile.memory_projection.requested.v1` 会真实通过 HTTP relay 投递；`context.memory.extracted.v1` 与 `context.memory.summary.updated.v1` 当前仍以日志级自记录为主。

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
- `routing.last_observation`
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
