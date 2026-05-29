# OpenAPI（草案 + 纵切面尾差）

- 当前裁决顺序为：运行代码与测试 > `repo/platform/contracts/` > `repo/data-platform/` > `OneLink/rules/` > `OneLink/docs/archive/`。
- 路径历史来源于 `OneLink/docs/archive/rules-legacy-2026-05-15/Rules/15-MVP-OPENAPI-DRAFT.md`，但当前以本目录中的现行 OpenAPI 与 `OneLink/rules/04-DATA-EVENT-CONTRACTS.md` 为准。
- **已按最小可联调同步（Composer 2 fast）**：`identity-service.yaml`、`profile-service.yaml`、`bff.yaml`、`ai-chat-service.yaml` 中本轮用到的路径带请求/响应 schema；其余路径可仍为 `501` 占位。
- `servers.url` 使用各服务**直连默认端口**（如 identity **8081**、bff **8083**、ai-chat **8085**），与 api-gateway **8080** 区分。
- **内部 relay**：`x-internal-token` / `INTERNAL_SHARED_SECRET` 为 **dev-only** 服务间约定，**不**在公开 OpenAPI 中展开 `/internal/*`（见各服务 README）。
