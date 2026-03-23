# ai-chat-service

## 服务职责
用户与 AI 好友的对话、会话与消息持久化；模型调用经 `model-gateway`，长期上下文经 `context-service`。

## 拥有的数据
`ai_conversations`, `ai_messages`, `ai_message_contents`（规划态；当前 MVP 为内存实现）。

## 对外接口
`/api/v1/chat/*` — 见 `repo/platform/contracts/openapi/ai-chat-service.yaml`。

**公开 API 鉴权（与实现一致）**  
以下路径需 **`Authorization: Bearer <opaque token>`**（identity `GET /me`）：

- `POST /api/v1/chat/conversations`

以下路径除 Bearer 外，还会校验 **会话属主**（非本人 `conversation_id` → 403/404）：

- `GET|POST /api/v1/chat/conversations/{conversationId}/messages`
- `GET /api/v1/chat/conversations/{conversationId}/context`

**内部接口（不面向浏览器）**  
`GET /internal/chat/conversations/{id}/messages/{messageId}` 供 **context-service** 拉取用户消息正文；须 **`x-internal-token: <INTERNAL_SHARED_SECRET>`**。  
本路径 **未** 写入公开 OpenAPI，避免误用。

另有 `GET /internal/observability/chat-relay` 供开发态查看 `ai-chat -> context` 的 relay 失败记录；同样要求 `x-internal-token`，不对前端公开。

## 依赖
`identity-service`（Bearer 校验）、`context-service`、`model-gateway`；异步向 `context-service` relay `chat.user_message.created.v1`，后续 `context -> profile` 由 `context-service` 继续投递。

## 不负责
用户间私信（dm-service）、问卷主写（question-service）。

## 文档来源
`OneLink/Rules/10-SERVICE-BOUNDARIES.md`

## V2 最小实现状态

- 内存态 conversation / messages / context snapshot；重启丢失。
- 同步链路边界：`POST /internal/context/build`、`model-gateway` 调用等仍按骨架落地。
- `POST /internal/context/build` 当前会把上游 chat `trace_id` 一并传给 `context-service`，用于把 `route_escalation_deferred` 等 failure sample 关联回具体请求。
- `POST /internal/context/build` 现已携带 **`x-internal-token`**，与 `context-service` 的全量 internal 守卫保持一致。
- `chat.user_message.created.v1 -> context-service` 当前带最小重试；若 3 次内仍失败，会落到 `GET /internal/observability/chat-relay` 可查询的 in-memory 失败记录。
- `idempotency_key` 当前仅作为前向兼容字段保留；**MVP 不执行 conversation/message 去重**，重试保护不能依赖它。
- `created_at` / `last_message_at` 当前使用固定开发态时间戳占位，用于稳定跑通纵切面；**不能**当作真实消息时间做时序判断。
- **Dev-only 服务间口令**：环境变量 **`INTERNAL_SHARED_SECRET`**（默认 **`onelink-dev-internal-token`**），与 **context-service、profile-service** 必须一致；**不是**生产级零信任。

## 运行（本地）

```bash
cd OneLink/repo
export RUST_LOG=info
export INTERNAL_SHARED_SECRET="${INTERNAL_SHARED_SECRET:-onelink-dev-internal-token}"
export IDENTITY_SERVICE_BASE_URL="${IDENTITY_SERVICE_BASE_URL:-http://127.0.0.1:8081}"
export CONTEXT_SERVICE_BASE_URL="${CONTEXT_SERVICE_BASE_URL:-http://127.0.0.1:8089}"
export MODEL_GATEWAY_BASE_URL="${MODEL_GATEWAY_BASE_URL:-http://127.0.0.1:8090}"
PORT=8085 cargo run -p ai-chat-service
```

## 联调纵切面
`scripts/local/run-chat-memory-profile-slice.sh`、`repo/tests/integration/CHAT_MEMORY_PROFILE_SLICE.md`。
