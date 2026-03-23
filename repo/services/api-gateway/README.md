# api-gateway

## 服务职责

统一入口：路由、鉴权校验、限流等（MVP 骨架阶段仅占位）。

## 拥有的数据

无自有 OLTP 表；可缓存会话校验结果等（未实现）。

## 对外接口

- `GET /health`
- 权威业务 API 契约见 `OneLink/repo/platform/contracts/openapi/` 与 `OneLink/Rules/15-MVP-OPENAPI-DRAFT.md` 的 `/api/v1` 入口约定。
- `api-gateway` 作为统一入口与转发层，不单独作为权威业务 OpenAPI 来源。

## 依赖

- 下游：各领域服务（BFF / identity / profile / …）
- 不直接调用 `model-gateway`（由业务服务调用）

## 不负责

- 业务聚合（由 `bff`）、画像写入、推荐逻辑等。

## 文档来源

- `OneLink/Rules/10-SERVICE-BOUNDARIES.md`
- `OneLink/Rules/02-TECH-ARCHITECTURE.md`
- `OneLink/Rules/15-MVP-OPENAPI-DRAFT.md`
