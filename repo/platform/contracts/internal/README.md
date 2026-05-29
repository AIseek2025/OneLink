# 内部契约

- 当前裁决顺序为：运行代码与测试 > `repo/platform/contracts/` > `repo/data-platform/` > `OneLink/rules/` > `OneLink/docs/archive/`。
- `model-gateway.yaml`：`model-gateway` 能力（不对浏览器暴露）。
- `context-service.yaml`：`context-service` 内部接口（Memory Compute Layer，历史来源于 `Rules/15` §9）。

当前规范见 `OneLink/rules/03-SERVICE-BOUNDARIES.md`、`OneLink/rules/04-DATA-EVENT-CONTRACTS.md`。  
历史依据见 `OneLink/docs/archive/rules-legacy-2026-05-15/Rules/15-MVP-OPENAPI-DRAFT.md` 与 `OneLink/docs/archive/rules-legacy-2026-05-15/Rules-V2/CONTRACTS/context-service-contract.md`。
