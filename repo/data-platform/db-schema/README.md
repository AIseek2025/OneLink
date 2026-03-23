# db-schema

- **`drafts/`** — DDL 草案，与 `OneLink/Rules/14-MVP-SQL-SCHEMA-DRAFT.md` 对齐，可按序 `psql -f` 本地建表（见各文件头注释）。
- **`migrations/`** — 正式 migration 占位；生产演进由工程师在此添加工具链（如 sqlx / refinery），**勿**将 `drafts/` 误当作已发布 migration。

推荐执行顺序：`001` → `008`（见 `drafts/` 内 Prerequisite 说明）。
