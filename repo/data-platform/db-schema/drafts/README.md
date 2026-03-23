# SQL drafts

来源：`OneLink/Rules/14-MVP-SQL-SCHEMA-DRAFT.md`（按域拆分）。

本地一次性建表示例（按序）：

```bash
psql "$DATABASE_URL" -f 001_identity.sql
psql "$DATABASE_URL" -f 002_profile.sql
psql "$DATABASE_URL" -f 003_context.sql
psql "$DATABASE_URL" -f 004_ai_chat.sql
psql "$DATABASE_URL" -f 005_dm.sql
psql "$DATABASE_URL" -f 006_question.sql
psql "$DATABASE_URL" -f 007_match.sql
psql "$DATABASE_URL" -f 008_safety.sql
psql "$DATABASE_URL" -f 009_model_gateway.sql
```
