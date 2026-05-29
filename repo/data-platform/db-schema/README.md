# db-schema

- 当前裁决顺序为：运行代码与测试 > `repo/platform/contracts/` > `repo/data-platform/` > `OneLink/rules/` > `OneLink/docs/archive/`。
- **`drafts/`** — DDL 草案，历史来源于 `OneLink/docs/archive/rules-legacy-2026-05-15/Rules/14-MVP-SQL-SCHEMA-DRAFT.md`，可按序 `psql -f` 本地建表（见各文件头注释）。
- **`migrations/`** — 正式 migration 占位；生产演进由工程师在此添加工具链（如 sqlx / refinery），**勿**将 `drafts/` 误当作已发布 migration。

推荐执行顺序：`001` → `008`（见 `drafts/` 内 Prerequisite 说明）。

当前规范见 `OneLink/rules/04-DATA-EVENT-CONTRACTS.md` 与各域 owner service README。archive 只用于字段来源核对与历史追溯，不单独作为当前建模依据。
