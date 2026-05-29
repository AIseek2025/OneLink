# SQL drafts

当前裁决顺序为：运行代码与测试 > `repo/platform/contracts/` > `repo/data-platform/` > `OneLink/rules/` > `OneLink/docs/archive/`。

来源：`OneLink/docs/archive/rules-legacy-2026-05-15/Rules/14-MVP-SQL-SCHEMA-DRAFT.md`（按域拆分）。

**权威建表顺序**以 **`OneLink/docs/archive/rules-legacy-2026-05-15/Rules/14-MVP-SQL-SCHEMA-DRAFT.md` §7** 为准；下列脚本顺序与之对齐。与 §7 步骤对应关系概览：

| §7 步骤（摘要） | 本目录文件 |
|-----------------|------------|
| 1–3：`users` 及 identity/profile 相关 | `001_identity.sql`、`002_profile.sql` |
| 4：context-service 记忆域 | `003_context.sql`；其后可执行 **`003_context_activation.sql`**（Phase 1 activation 动态状态扩展）、**`003_context_idempotency.sql`**（checkpoint / consolidate 幂等辅助表，供 `context-service` 在配置 `DATABASE_URL` 时使用）；可选 **`011_runtime_observability.sql`**（routing / failure 追加表，asmr-lite 跨重启读） |
| 5–9：ai-chat、dm、question、match、safety | `004_ai_chat.sql` … `008_safety.sql` |
| 10：`policy_configs` → `policy_experiments` → `policy_rollouts` | `010_optimization.sql` |
| 11：`model_invocation_logs` | `009_model_gateway.sql` |

### Policy 占位表（§7 第 10 步）

`policy_*` 的 DDL 见 **`010_optimization.sql`**（与 `OneLink/docs/archive/rules-legacy-2026-05-15/Rules/14` §3.3A 一致）。应用顺序：**在 `008_safety.sql` 之后、`009_model_gateway.sql` 之前**。

本地一次性建表示例（按序）：

```bash
psql "$DATABASE_URL" -f 001_identity.sql
psql "$DATABASE_URL" -f 002_profile.sql
psql "$DATABASE_URL" -f 003_context.sql
psql "$DATABASE_URL" -f 003_context_activation.sql
psql "$DATABASE_URL" -f 003_context_idempotency.sql
psql "$DATABASE_URL" -f 011_runtime_observability.sql
psql "$DATABASE_URL" -f 004_ai_chat.sql
psql "$DATABASE_URL" -f 005_dm.sql
psql "$DATABASE_URL" -f 006_question.sql
psql "$DATABASE_URL" -f 007_match.sql
psql "$DATABASE_URL" -f 008_safety.sql
psql "$DATABASE_URL" -f 010_optimization.sql
psql "$DATABASE_URL" -f 009_model_gateway.sql
```

当前规范见 `OneLink/rules/04-DATA-EVENT-CONTRACTS.md`。若 archive 历史说明与本目录实际 SQL 文件冲突，以当前 SQL 文件和 owner service 实现为准。
