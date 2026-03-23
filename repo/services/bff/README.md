# bff

## 服务职责
面向 Web/Admin 的薄聚合层：编排多领域接口，减少前端扇出。

## 拥有的数据
无核心 OLTP；可有聚合缓存（未实现）。

## 对外接口
见 `repo/platform/contracts/openapi/bff.yaml` 与 `OneLink/Rules/15-MVP-OPENAPI-DRAFT.md` §10（`/api/v1/bff/*`）。

**本轮纵切面（主实现：Composer 2）已接线：** `GET /api/v1/bff/chat/init`。  
行为：**不解析** token，仅 **透传** `Authorization` 到 `identity-service` 与 `ai-chat-service`。  
下游 **ai-chat** 的会话/消息 **读接口** 同样需要有效 Bearer；BFF **不**携带、也 **不应**暴露 `INTERNAL_SHARED_SECRET`（内部 secret 仅服务间使用）。

**本地联调提示**  
若异步 relay（chat → context → profile）失败，请检查 **ai-chat / context / profile** 是否以 **相同** `INTERNAL_SHARED_SECRET` 启动（见 `scripts/local/run-chat-memory-profile-slice.sh` 说明）。此为 **dev-only** 共享口令，**不是**生产级零信任方案。

## 本地运行（默认端口 **8083**）

```bash
cd repo
RUST_LOG=info PORT=8083 cargo run -p bff
```

环境变量：
- `IDENTITY_SERVICE_BASE_URL`（默认 `http://127.0.0.1:8081`）  
- `AI_CHAT_SERVICE_BASE_URL`（默认 `http://127.0.0.1:8085`）

## 依赖
下游：identity、ai-chat（本轮）；未来 profile、dm、question、match、safety 等。

## 不负责
领域主写、画像事实写入、模型直连（须经 model-gateway）。**不是**业务真相 owner。

## 联调纵切面
`scripts/local/run-chat-memory-profile-slice.sh`、`repo/tests/integration/CHAT_MEMORY_PROFILE_SLICE.md`。

## 文档来源
`OneLink/Rules/10-SERVICE-BOUNDARIES.md`, `OneLink/Rules/15-MVP-OPENAPI-DRAFT.md`
