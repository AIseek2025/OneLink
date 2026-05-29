# contracts

- 当前裁决顺序为：运行代码与测试 > `repo/platform/contracts/` > `repo/data-platform/` > `OneLink/rules/` > `OneLink/docs/archive/`。
- **`openapi/`** — 对外（经 BFF / 领域服务）HTTP 契约骨架；历史来源于 `OneLink/docs/archive/rules-legacy-2026-05-15/Rules/15-MVP-OPENAPI-DRAFT.md`。
- **`internal/`** — 不暴露给前端的内部能力（如 `model-gateway`）。

当前规范见 `OneLink/rules/03-SERVICE-BOUNDARIES.md`、`OneLink/rules/04-DATA-EVENT-CONTRACTS.md`。  
历史依据见 `OneLink/docs/archive/rules-legacy-2026-05-15/Rules/10-SERVICE-BOUNDARIES.md`。
