# Chat → Memory Projection → Profile 可见 — 验证说明

> **配套文档**（Composer 2 fast）。主链路由与事件消费由 **Composer 2** 在各服务 crate 内实现；本文只描述如何联调与观察结果。  
> 验收基线：`OneLink/Rules/20-FIRST-RUNNABLE-VERTICAL-SLICE-BRIEF.md`。  
> 事件口径：`OneLink/Rules/16-MVP-EVENT-SCHEMAS-DRAFT.md`（勿改事件名 / producer）。
> V2 约束：`OneLink/Rules-V2/00-CONSTITUTION.md`、`Rules-V2/CONTRACTS/context-service-contract.md`、`Rules-V2/EXECUTION/asmr-lite-benchmark-and-acceptance-checklist.md` 用于约束本轮 MVP 边界与下一轮验收方向。

## 端口矩阵（冻结）

| 服务 | 端口 |
|------|------|
| api-gateway | 8080（本轮纵切面可不启） |
| identity-service | 8081 |
| profile-service | 8082 |
| bff | 8083 |
| ai-chat-service | 8085 |
| context-service | 8089 |
| model-gateway | 8090 |

## 鉴权与内部口令（本轮纵切面）

### Bearer（用户态）

- **ai-chat** 的 **`GET …/messages`**、**`GET …/context`** 与写接口一样，必须带 **`Authorization: Bearer <token>`**，且 **会话须属于当前用户**（否则 403/404）。
- **profile** 的 **`GET /profile/me`**、**`GET /profile/me/completion`** 需要 Bearer。
- **BFF** `GET /api/v1/bff/chat/init` 仅 **透传** `Authorization` 到 identity / ai-chat，不在 BFF 侧解析 token。

### `INTERNAL_SHARED_SECRET` + `x-internal-token`（dev-only 服务间）

- **Header 名**：`x-internal-token`（实现按此名读取）。
- **值**：与各服务环境变量 **`INTERNAL_SHARED_SECRET`** 相同；未设置时实现默认 **`onelink-dev-internal-token`**。
- **须一致的服务进程**：**ai-chat-service**、**context-service**、**profile-service**（本地 `start-bg` 或 `print-start` 时请统一 export）。
- **典型调用链**（由实现发起，**不要**在浏览器或 smoke 里伪造）：
  - ai-chat → context：`POST /internal/context/build`（带 `x-internal-token`）
  - context → ai-chat：`GET /internal/chat/.../messages/{id}`（带 `x-internal-token`）
  - context → profile：`POST /internal/events/receive`（带 `x-internal-token`）
  - ai-chat → context：`POST /internal/events/receive`（带 `x-internal-token`）
  - profile → context：`POST /internal/memory/resolve`（带 `x-internal-token`）
- **说明**：这是 **开发态 relay 共享口令**，**不是**生产级零信任 / 服务身份方案。

### Identity `expires_at` 与 smoke

- `session.expires_at` 为 **真实过期时间**（当前实现约 **30 天**）。`GET /api/v1/identity/me` 过期返回 **401**。
- 若 smoke 失败且返回 401，先确认是否用了**旧 token** 或长时间挂机后重跑；**重新 register** 即可排除「过期误判为链路坏了」。

## 最小验证顺序

1. **注册**（拿 `session.token`）  
   `POST http://127.0.0.1:8081/api/v1/identity/register`  
   Body 字段见 `Rules/15` §2.1 与 `platform/contracts/openapi/identity-service.yaml`。

2. **（可选）登录** — 与注册二选一用于拿 token；若已注册则用 login。

3. **BFF 聊天初始化** — 透传 `Authorization: Bearer <token>`  
   `GET http://127.0.0.1:8083/api/v1/bff/chat/init`  
   期望 JSON：`user`（identity.me）、`conversation`、`pending_questions`（本轮为空数组）。

4. **发用户消息** — 需同一 Bearer  
   `POST http://127.0.0.1:8085/api/v1/chat/conversations/{conversation_id}/messages`  
   Body：`content_type`, `content_text`, `idempotency_key`（见 `Rules/15` §4）。
   说明：`idempotency_key` 当前仅作前向兼容字段保留，**MVP 不执行请求去重**；不要把它当作真实重试保护。

5. **（推荐）列消息** — 验证读路径鉴权：  
   `GET http://127.0.0.1:8085/api/v1/chat/conversations/{conversation_id}/messages`  
   Header：`Authorization: Bearer <token>`（与 `scripts/smoke-chat-memory-profile.sh` 一致）。

6. **等待异步链路** — dev-only HTTP relay：`ai-chat` → `context` → `profile`；建议 `sleep 1`～`2`。  
   若画像长期不更新，检查三服务 **`INTERNAL_SHARED_SECRET`** 是否一致（见上）。

7. **读画像**  
   - `GET http://127.0.0.1:8082/api/v1/profile/me`  
   - `GET http://127.0.0.1:8082/api/v1/profile/me/completion`  

   期望：`headline` / `bio` 或 `completion` 维度在记忆投影后出现可感知变化（具体文案依赖主实现启发式）。

8. **（推荐）查看 ASMR-Lite 观测面**  
   `GET http://127.0.0.1:8089/internal/observability/asmr-lite`  
   Header：`x-internal-token: ${INTERNAL_SHARED_SECRET:-onelink-dev-internal-token}`  
   期望：
   - `artifact_count` / `summary_count` / `entity_count` 大于 0
   - `checkpoint_count` 可用于确认 checkpoint 内部接口已具备真实 in-memory 行为
   - `routing.total_requests` 递增
   - `routing.last_observation.executed_route == "L1"`
   - `routing.last_observation.summary_hits` / `artifact_hits` / `entity_hits` 可读（「问城市但未点名」时 `entity_hits` 可对齐用户已抽取的 `location` 实体数）
   - `routing.last_observation.route_confidence` / `estimated_llm_calls` / `estimated_tokens` / `query_preview` 可读
   - `routing.last_observation.query_preference_polarity` / `evidence_preference_polarity` 可读（偏好类查询为 `positive|negative|neutral`）
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

## 一键 Shell Smoke

依赖：`curl`、`jq`、六服务已监听上述端口。

```bash
cd OneLink/repo
bash scripts/smoke-chat-memory-profile.sh
```

编排入口（含后台启动提示）：

```bash
bash scripts/local/run-chat-memory-profile-slice.sh print-start   # 打印 cargo run 命令
bash scripts/local/run-chat-memory-profile-slice.sh start-bg       # 可选：后台起服务
sleep 12
bash scripts/local/run-chat-memory-profile-slice.sh smoke
```

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
- `artifact_count` / `summary_count` / `total_failures` 为**进程内累计值**；重复运行时请主要看 `last_observation`、`recent_failures` 与字段是否出现，而不是要求总数回到固定值。
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

## Rust 测试壳

```bash
cd OneLink/repo
cargo test -p integration_chat_memory_profile_slice
```

在六服务已就绪时：

```bash
RUN_SLICE_HTTP_SMOKE=1 cargo test -p integration_chat_memory_profile_slice slice_http_smoke -- --exact --nocapture
```

## 日志与后续 Benchmark 接入

- 主链相关 crate 使用 `tracing`；建议 `RUST_LOG=info`（或 `debug`）启动。
- `context-service` 额外提供 `GET /internal/observability/asmr-lite`，可直接读取当前 in-memory 的 route / failure / evidence 计数，作为 benchmark v1 的最小观察面。
- `ai-chat-service` 额外提供 `GET /internal/observability/chat-relay`，用于查看 `chat.user_message.created.v1 -> context-service` 的最小失败记录。
- `profile-service /internal/events/receive` 在 `memory/resolve` 失败时会返回非 2xx，让 `context -> profile` relay 可以重试或记录失败，不再把失败伪装成接受成功。
- `profile-service` 的 Bearer 校验错误语义已与 `ai-chat-service` 对齐：identity `5xx` / 不可达返回 `502`，无效或过期 token 返回 `401`。
- 冻结事件名（用于 grep / 未来 benchmark 埋点对齐）：  
  `chat.user_message.created.v1`、`context.memory.extracted.v1`、`context.memory.summary.updated.v1`、`profile.memory_projection.requested.v1`  
  完整清单与验收见：`Rules-V2/EXECUTION/asmr-lite-benchmark-and-acceptance-checklist.md`。  
- **当前仓库脚本不运行完整 ASMR-Lite benchmark**；后续可在同一 smoke 之后叠加 checklist 中的用例与指标采集，无需改事件 schema。

## OpenAPI

本轮对外契约：`platform/contracts/openapi/identity-service.yaml`、`profile-service.yaml`、`bff.yaml`、`ai-chat-service.yaml`（与 `Rules/15` 字段名对齐）。  
**`/internal/*` 服务间路径** 不放入公开 OpenAPI，避免误当作前端 API；细节见各服务 README。

## 已知契约/实现尾差（若存在）

- `Rules/15` 部分章节描述通用响应 **envelope**（`data` / `error`）；当前 Rust MVP 对上述路径多返回 **裸 JSON 体**。OpenAPI 已按**实际响应体**描述；统一 envelope 需单独迭代（交 GPT 5.4 / 主实现）。
