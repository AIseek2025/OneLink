# OneLink V2 Context Service Contract

## 1. 文档目标

把 `context-service` 在 V2 中的内部接口边界、请求字段、响应字段与行为约束先冻结下来，为后续 OpenAPI 级联更新提供单一来源。

---

## 2. 服务定位

`context-service` 是 `Memory Compute Layer`，内部逻辑上包含两个域：

- `session domain`
- `memory domain`

物理上 MVP 仍保持单进程服务。

---

## 3. 内部接口清单

### 3.1 同步核心接口

1. `POST /internal/context/build`
2. `POST /internal/session/checkpoint`

### 3.2 异步或内部辅助接口

1. `POST /internal/memory/write`
2. `POST /internal/memory/consolidate`
3. `GET /internal/memory/search`
4. `POST /internal/memory/resolve`
5. `POST /internal/events/receive`
6. `GET /internal/observability/asmr-lite`

说明：

- `build` 是实时链路必须接口
- 其余接口可先作为内部服务能力，不要求全部对外可调

---

## 4. `POST /internal/context/build`

### 4.1 调用方

主要调用方：

- `ai-chat-service`

后续可复用方：

- `match-service`
- `question-service`

### 4.2 请求体

```json
{
  "user_id": "uuid",
  "agent_id": "uuid",
  "conversation_id": "uuid",
  "input": "string",
  "task_type": "chat",
  "max_tokens": 8000,
  "memory_limit": 20,
  "summary_limit": 3,
  "reply_style": "brief",
  "trace_id": "uuid",
  "retrieval_modes": ["structured", "semantic", "temporal"]
}
```

### 4.3 响应体

```json
{
  "system_prompt": "string",
  "user_context": "string",
  "memory_context": "string",
  "task_context": "string",
  "selected_summary_ids": ["uuid"],
  "selected_memory_ids": ["uuid"],
  "retrieval_used": ["structured", "semantic"],
  "degraded": false,
  "token_budget": {
    "max_tokens": 8000,
    "memory_limit": 20,
    "summary_limit": 3
  }
}
```

### 4.4 行为约束

- 必须显式执行 token budget
- 不允许无上限拼接原始历史
- 向量检索失败时必须降级
- 降级时必须显式返回 `degraded = true`
- 当前 MVP 的 `semantic` / `temporal` 仍按 **deterministic L1** 子集落地，不代表已接入独立向量检索或完整时间推理引擎
- 当前 MVP 中，若 `retrieval_used` 为空或结构化 evidence 为空，也允许进入 `degraded = true` 路径
- `memory_context` 在 L1 证据非空时为 **单字符串摘要**（非 JSON），其中可包含以下 **键=值式片段**（实现保证前缀稳定，便于 benchmark 与日志 grep；具体标点以代码 `format!` 为准）：
  - `summary_hits` / `artifact_hits` / `entity_hits` / `top_confidence`
  - `query_polarity_hint`：来自查询的偏好极性推断，非偏好类查询为 `none`
  - `pref_top`：当前最高分证据上的 `preference_polarity`（`positive` \| `negative` \| `neutral`）
  - `summaries=` / `artifacts=` 的短预览
- `task_context` 追加 `query_preference_polarity` 与 `evidence_preference_polarity`（与 observability 中 `last_observation` 同口径；值为 `positive` \| `negative` \| `neutral`）
- **artifact / summary 字段**：进程内 `MemoryArtifactRecord` / `MemorySummaryRecord` 含 **`preference_polarity`**，与 distiller 输出一致；`GET /internal/memory/search` 的 `MemorySearchItem` **必须**返回同名字段（见 OpenAPI `context-service.yaml`）

---

## 5. `POST /internal/session/checkpoint`

### 5.1 用途

保存当前 runtime 的可恢复状态。

### 5.2 请求体

```json
{
  "agent_id": "uuid",
  "user_id": "uuid",
  "conversation_id": "uuid",
  "schema_version": 1,
  "working_summary_ref": "uuid",
  "runtime_state_blob": {
    "active_task": "chat",
    "last_tool_result": null
  },
  "policy_versions": {
    "memory_policy": "v1",
    "session_policy": "v1",
    "retrieval_policy": "v1"
  }
}
```

### 5.3 响应体

```json
{
  "accepted": true,
  "checkpoint_id": "uuid"
}
```

### 5.4 行为约束

- 必须写入带 `schema_version` 的 checkpoint
- 失败不得阻塞主响应
- 当前 MVP 需对**相同请求体**支持幂等重试，重复请求返回同一 `checkpoint_id`
- `runtime_state_blob` 与 `policy_versions` 的去重键应按 **canonical JSON** 生成，不能依赖隐含 map 输出顺序

---

## 6. `POST /internal/memory/write`

### 6.1 用途

承接来自事件消费者或内部调用的记忆候选写入。

### 6.2 请求体

```json
{
  "event_id": "uuid",
  "user_id": "uuid",
  "source_type": "chat",
  "source_service": "ai-chat-service",
  "source_ref_id": "uuid",
  "raw_text": "string",
  "memory_value_score": 0.81
}
```

`memory_value_score` 是请求层统一候选分值，不直接等于表字段；MVP 由 `context-service` 在写入或整合阶段把它映射到 `importance_score` 等内部评分字段。

### 6.3 响应体

```json
{
  "accepted": true,
  "write_mode": "direct_internal",
  "trace_id": "uuid"
}
```

### 6.4 行为约束

- 以异步事件消费为主
- 不允许业务服务把它当成主同步写入口
- 当前 MVP 若 `raw_text` 为空，可返回 `accepted = false` 与 `write_mode = \"noop_missing_raw_text\"`

---

## 7. `POST /internal/memory/consolidate`

### 7.1 用途

触发记忆整合、去重、冲突标记与 summary 更新。

### 7.2 请求体

```json
{
  "event_id": "uuid",
  "user_id": "uuid",
  "artifact_ids": ["uuid"],
  "mode": "incremental"
}
```

### 7.3 响应体

```json
{
  "accepted": true,
  "replayable": true
}
```

### 7.4 行为约束

- 必须以 `event_id` 幂等
- 必须可重放
- 不得因整合失败破坏原始候选保留
- 当前 MVP 对重复 `event_id` 应返回成功，但不重复写入新 summary
- 若 `artifact_ids` 为空，或传入后一个都解析不到有效 artifact，当前 MVP 返回 `accepted = false`，且**不占用** `event_id` 幂等索引；允许后续同一 `event_id` 以有效 `artifact_ids` 重试

---

## 8. `GET /internal/memory/search`

### 8.1 用途

用于内部检索与诊断，不直接暴露给前端。

### 8.2 查询参数

- `user_id`
- `query`
- `limit`

当前 MVP 不提供 `network_types` 与 `cursor`，统一返回 `next_cursor = null`。

### 8.3 响应体

```json
{
  "items": [
    {
      "memory_id": "uuid",
      "network_type": "opinion",
      "memory_level": "persistent",
      "content": "用户偏好直接沟通",
      "confidence": 0.82,
      "preference_polarity": "neutral"
    }
  ],
  "next_cursor": null
}
```

---

## 9. 事件关系

`context-service` 必须生产或消费以下关键事件：

### 9.1 消费

- `chat.user_message.created.v1`
- `question.answered.v1`
- 行为反馈相关事件

### 9.2 生产

- `context.memory.extracted.v1`
- `context.memory.summary.updated.v1`
- `profile.memory_projection.requested.v1`

当前 MVP 说明：

- `profile.memory_projection.requested.v1` 由 `context-service` 通过 dev-only HTTP relay 真实投递到 `profile-service`
- `context.memory.extracted.v1` 与 `context.memory.summary.updated.v1` 当前以**日志级自我记录**为主，用于观测与后续事件总线对接，不应误读为已具备可靠下游投递

---

## 10. 补充内部接口

### 10.1 `POST /internal/memory/resolve`

- 用于 `profile-service` 按 `memory_ids` 拉取投影所需文本
- 需 `x-internal-token`
- 当前 MVP 返回 `items[] = { memory_id, content, network_type }`

### 10.2 `POST /internal/events/receive`

- dev-only HTTP envelope relay 入口
- 当前 MVP 重点消费 `chat.user_message.created.v1`
- 当前返回 `202 Accepted` 仅表示**已接受异步处理**，不等于下游 fetch / projection 已完成

### 10.3 `GET /internal/observability/asmr-lite`

- 用于查看 `artifact_count`、`summary_count`、`checkpoint_count`
- 用于查看 `routing.last_observation`
- `routing.last_observation` 当前至少应包含：`upgraded`、`summary_hits`、`artifact_hits`、`entity_hits`、`route_confidence`、`estimated_llm_calls`、`estimated_tokens`、`query_preview`，以及偏好口径字段 `query_preference_polarity`、`evidence_preference_polarity`（`positive` \| `negative` \| `neutral`）
- 用于查看 `recent_failures[*].stage / trace_id / retryable / attempt_count`
- 用于辅助判断异步链路是否已完成；不能把 `POST /internal/events/receive = 202` 误读为“下游处理已完成”

---

## 11. 与 V1 契约的差异

### 11.1 保留

- `/internal/context/build` 继续是核心入口
- token budget 仍为必填

当前 MVP 中，`token_budget` 的显式执行主要体现在 `memory_limit` / `summary_limit` 条数上限与返回字段贯通；
尚未承诺完整 token 级精确裁剪。

### 11.2 升级

- 引入 `agent_id`
- 显式返回 `retrieval_used`
- 引入 `session/checkpoint`
- 引入 `memory/consolidate`

---

## 12. 一句话定义

> `context-service` 的内部契约必须先冻结“做什么”和“不能做什么”，再去写 OpenAPI 细节，否则 V2 仍会重演 V1 的契约漂移问题。
