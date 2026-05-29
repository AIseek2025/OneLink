# OneLink

## 当前真相入口

- **统一规则入口**：[`rules/`](rules/)（合并旧 `Rules/`、`Rules-V2/` 与 2026-05-15 审计结论后的当前真相源）。
- **Agent 轻量记忆（Cursor / 新会话优先读）**：[`docs/AGENT_MEMORY_BRIEF.md`](docs/AGENT_MEMORY_BRIEF.md)（产品主线、工程入口、端口与密钥、排障指针；与 `rules/` 或代码冲突时以当前代码和 `rules/` 为准。always-apply 规则：`.cursor/rules/read-agent-memory-brief.mdc`。）
- **历史规则归档**：[`docs/archive/rules-legacy-2026-05-15/`](docs/archive/rules-legacy-2026-05-15/)（仅用于历史追溯、字段来源核对与审计说明，不作为当前默认开发入口）。
- **模型命名注意事项**：`OneLink` 当前 `coding` 主位配置里的 `astron-code-latest`，在本项目语境中按 **`glm-5.1` 企业版编码入口** 理解；讨论“主编程师 = `glm-5.1`”时，不应把它误判成另一条不同模型路线。

## 仓库结构

- **工程代码**：[`repo/`](repo/)
- **产品 / 工程规则**：[`rules/`](rules/)
- **历史规则归档**：[`docs/archive/rules-legacy-2026-05-15/`](docs/archive/rules-legacy-2026-05-15/)
