# shared-libs

- 当前裁决顺序为：运行代码与测试 > `repo/platform/contracts/` > `repo/data-platform/` > `OneLink/rules/` > `OneLink/docs/archive/`。
- **`onelink-event-envelope`**：通用事件 envelope 的共享 serde 类型，供 dev-only HTTP relay 与各服务对齐字段；历史来源于 `OneLink/docs/archive/rules-legacy-2026-05-15/Rules/16`。

其他跨服务 crate（tracing 扩展、错误码等）可按需增量添加。

当前规范见 `OneLink/rules/04-DATA-EVENT-CONTRACTS.md` 与 `repo/data-platform/event-schemas/`。若 archive 历史口径与当前共享类型或 schema 冲突，以当前代码和 schema 为准。
