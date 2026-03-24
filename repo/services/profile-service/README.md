# profile-service

## 服务职责
资料、被找设置、关注关系；**唯一**拥有全部 `profile_*` 与 `follows` 写路径。

## 结构化画像投影 Phase A + Phase B（当前纵切面）

- **不是**完整画像引擎：无独立问卷链、无向量索引、无持久库；**in-memory** `HashMap` + **启发式**事实映射（`src/projection.rs`）。
- **事实层** `facts[]`：除 `fact_type`、`value` 外，Phase B 为每条事实写入 **`confidence`**（0~1 规则档位，**非**模型校准分）、**`source_memory_id`**（对齐 `context` `memory/resolve` 的 `memory_id`）；**`source_message_id`** 仅在上游记忆行非空时透传。以上为 **最小可信度/溯源**，不是完整 provenance / 审计产品。
- **trait 聚合** `traits`：`communication_preferences` **仅**来自显式 `communication_preference` 事实（**不**再把泛化 `preference_polarity` 包装成沟通偏好）；`location_label` 无值时为 JSON **`null`**，但键始终存在（见 OpenAPI）。
- **完成度** `GET /api/v1/profile/me/completion` 仍为 **五维**结构化覆盖率（维度集合与 Phase A 一致）。
- **`headline` / `bio`** 由事实与 traits **派生**（展示层）；**不作为** completion 主维度。
- **内部**仍维护 `memory_highlights` 以辅助派生，**当前**不随 `GET /me` JSON 暴露（与 `profile-service.yaml` 一致）。

## 拥有的数据
`profiles`, `discovery_preferences`, `follows`, `profile_facts`, `profile_fact_revisions`, `profile_traits`, `trait_supporting_facts`, `profile_embeddings`, `profile_summaries`。

## 对外接口
`/api/v1/profile/*` — 见 `repo/platform/contracts/openapi/profile-service.yaml`。

**本轮纵切面（主实现：Composer 2）已接线：** `GET /me`、`GET /me/completion`（均需 **`Authorization: Bearer`**，经 identity `GET /me`）；内部 `POST /internal/events/receive` 消费 `profile.memory_projection.requested.v1`（dev-only HTTP relay，**不**经 api-gateway）。其余 `PATCH` / follow 等仍为占位。

当前 Bearer 校验错误语义已与 `ai-chat-service` 对齐：
- identity `5xx` 或不可达：返回 `502`
- 非 `5xx` 且非成功（如无效/过期 token）：返回 `401`

**内部鉴权（dev-only，非生产零信任）**  
`POST /internal/events/receive` 须带 **`x-internal-token`**，值等于环境变量 **`INTERNAL_SHARED_SECRET`**。  
与 **ai-chat-service、context-service** 进程须配置 **同一 secret**；未设置时实现默认 **`onelink-dev-internal-token`**。

当前实现中，若 `profile.memory_projection.requested.v1` 后续调用 `context-service /internal/memory/resolve` 失败，`POST /internal/events/receive` 会返回非 2xx，供上游 relay 做重试或记录失败，而不再伪装成成功接受。

## 本地运行（默认端口 **8082**）

```bash
cd repo
RUST_LOG=info PORT=8082 cargo run -p profile-service
```

环境变量（默认值仅开发用）：
- `IDENTITY_SERVICE_BASE_URL`（默认 `http://127.0.0.1:8081`）— 校验 Bearer。  
- `CONTEXT_SERVICE_BASE_URL`（默认 `http://127.0.0.1:8089`）— 解析 `memory_ids` 文本（与主实现一致）。
- `INTERNAL_SHARED_SECRET`（默认 `onelink-dev-internal-token`）— 校验入站 `x-internal-token`；须与 ai-chat、context 一致。

- **in-memory** 画像存储；重启丢失。

## 依赖
identity（HTTP 校验）；context-service（HTTP 解析记忆工件，**非**直读聊天存储）。

## 不负责
AI 对话存储、私信、找人排序主逻辑。

## 联调纵切面
见 `repo/tests/integration/CHAT_MEMORY_PROFILE_SLICE.md`。后续 ASMR-Lite benchmark 见 `Rules-V2/EXECUTION/asmr-lite-benchmark-and-acceptance-checklist.md`（本服务仅预留日志/事件名观察，不跑完整 benchmark）。

## 文档来源
`OneLink/Rules/10-SERVICE-BOUNDARIES.md`, `OneLink/Rules/11-DATA-EVENT-MODEL.md`, `OneLink/Rules/15-MVP-OPENAPI-DRAFT.md`, `OneLink/Rules/16-MVP-EVENT-SCHEMAS-DRAFT.md`
