# OpenAPI（草案 + 纵切面尾差）

- 路径与 `OneLink/Rules/15-MVP-OPENAPI-DRAFT.md` 对齐。
- **已按最小可联调同步（Composer 2 fast）**：`identity-service.yaml`、`profile-service.yaml`、`bff.yaml`、`ai-chat-service.yaml` 中本轮用到的路径带请求/响应 schema；其余路径可仍为 `501` 占位。
- `servers.url` 使用各服务**直连默认端口**（如 identity **8081**、bff **8083**、ai-chat **8085**），与 api-gateway **8080** 区分。
- **内部 relay**：`x-internal-token` / `INTERNAL_SHARED_SECRET` 为 **dev-only** 服务间约定，**不**在公开 OpenAPI 中展开 `/internal/*`（见各服务 README）。
