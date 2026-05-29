# Rules Migration Notes

## 1. 目的

本文件说明：

- 为什么 OneLink 不再继续并行维护旧 `Rules/`、`Rules-V2/`
- 为什么当前唯一规则入口是 `rules/`
- 团队如何正确引用 archive、现行规则与工程事实源
- Git 层应如何完成目录收口，避免 macOS 与跨平台协作混乱

## 2. 为什么不继续沿用 `Rules-V2/`

`Rules-V2/` 的结构比旧 `Rules/` 更先进，尤其在宪法、架构、数据、契约与执行范式上更适合作为主骨架。

但它不适合继续直接作为当前唯一入口，原因有三点：

- `Rules-V2/EXECUTION/` 中包含大量历史 dispatch、brief、review、closeout，极易被误认为当前发单入口。
- 旧 `Rules/` 中仍保留了 MVP 产品边界、服务职责、SQL/OpenAPI/Event 冻结草案等有效信息，单纯“只看 V2”会丢失不少上下文。
- 当前工程事实已从“纯规划态”进入“有纵切面实现态”，需要一个把 `Rules/`、`Rules-V2/`、`docs/0327`、审计结果统一吸收后的新入口，而不是继续在历史目录上叠加。

因此，本次采用：

- 新建 `rules/` 作为当前唯一规则入口
- 将旧 `Rules/` 与 `Rules-V2/` 归档到 `docs/archive/rules-legacy-2026-05-15/`

## 3. 当前文档裁决顺序

当前规范裁决顺序固定为：

1. 运行代码与测试
2. `repo/platform/contracts/`
3. `repo/data-platform/`
4. `rules/`
5. `docs/archive/`

含义如下：

- 代码与测试描述当前真实行为
- contracts / schemas 是工程冻结事实源
- `rules/` 负责产品、架构、边界、路线图和开发计划
- archive 只做追溯、核对与审计，不再作为默认开发入口

## 4. archive 的正确用法

允许：

- 追溯某条字段、事件、接口最初从哪份规则演化而来
- 查历史验收基线、brief、review 结论
- 做规划再审查、审计和历史责任链核对

不允许：

- 直接从 archive 派生当前新开发任务
- 把 archive 里的字段口径覆盖当前 `rules/` 或工程 contracts
- 在 archive 中追加新的现行规则、当前发单入口或新的冻结契约

## 5. 推荐引用方式

在 README、测试说明、服务文档中，推荐统一采用如下格式：

```text
当前规范：OneLink/rules/xx-...
工程事实：repo/platform/contracts/... 或 repo/data-platform/...
历史依据：OneLink/docs/archive/rules-legacy-2026-05-15/...
```

这样可以保证：

- 团队先看到当前规范
- 工程实现能直接找到 contract / schema
- 历史资料仍可追溯，但不会形成并行真相源

## 6. Git 收口建议

当前仓库可能出现以下状态：

- 旧 `Rules/`、`Rules-V2/` 在 Git 中显示为删除
- 新 `rules/` 显示为未跟踪
- macOS 本地由于大小写不敏感，阅读时不容易察觉问题

为减少跨平台协作风险，建议单独完成一次“规则目录收口提交”：

1. 确认当前唯一现行目录名固定为 `rules/`
2. 确认 archive 中保留 `Rules/` 与 `Rules-V2/` 历史快照
3. 将目录迁移与规则内容修订分成两个提交
4. 在提交说明中明确：
   - `rules/` 为当前唯一入口
   - `docs/archive/...` 为历史归档
   - 不再从 `Rules-V2/EXECUTION/` 发当前任务

推荐提交拆分：

- 提交 A：目录迁移与归档
- 提交 B：规则内容修订与 README/README-like 文档联动

## 7. 后续维护建议

- 新增规则主题时，优先补到 `rules/`
- 如需追溯来源，在 `rules/README.md` 的映射表中补条目
- 任何进入 `repo/` 的契约/README 更新，都应明确“当前规范”与“历史依据”
- 当前波次工单、dispatch、brief、review、closeout 不写入 `rules/` 根目录；默认放入 `docs/execution/<yyyymmdd>-<topic>/` 或团队 issue tracker
- 当前波次工单 closeout 后，只有稳定结论才回写到 `rules/`、`repo/platform/contracts/` 或 `repo/data-platform/`
- 每次大范围规划调整后，至少同步检查：
  - `README.md`
  - `docs/AGENT_MEMORY_BRIEF.md`
  - `repo/README.md`
  - `rules/README.md`
  - `repo/platform/contracts/`
  - `repo/data-platform/`

## 8. 目录结构触发条件

当前 `rules/` 采用按序号扁平结构，短期有利于 onboarding 和快速检索；不立即恢复 `Rules-V2` 的职能子目录结构。

若出现以下任一条件，启动按职能子目录重组评估：

- `rules/*.md` 文件数超过 15。
- 任一规则文件超过 200 行。
- 出现第二处补丁式章节编号，例如 `3A`、`Annex`、`NA`。
- 团队规模达到 8 人以上，且 2 人以上经常同时修改 `rules/`。
- 团队决定建立长期当前波次工单目录。

预案映射如下：

| 推荐子目录 | 承接现行文件 | 未来增量主题 |
|------------|--------------|--------------|
| `rules/principles/` | `00-CONSTITUTION.md` | 安全底线、合规底线 |
| `rules/product/` | `01-PRODUCT-PLATFORM.md`、`13-GLOBAL-I18N-AND-COMPLIANCE.md` | 多端策略、i18n、UX 系统 |
| `rules/architecture/` | `02-SYSTEM-ARCHITECTURE.md`、`03-SERVICE-BOUNDARIES.md`、`04-DATA-EVENT-CONTRACTS.md`、`12-CAPACITY-AND-SLO.md` | SLO、容量、Model Gateway 成本、Prompt Gateway |
| `rules/ai-memory/` | `05-MEMORY-AGENT-CONTEXT.md` | Skill、Forgetting、个人记忆容器 |
| `rules/matching-safety/` | `06-MATCHING-SAFETY-GOVERNANCE.md` | 多区域风控、申诉、未成年人保护 |
| `rules/engineering/` | `07-ENGINEERING-QUALITY.md` | CI/CD、SRE、发布、Observability |
| `rules/roadmap/` | `08-ROADMAP.md`、`09-NEXT-DEVELOPMENT-PLAN.md` | 季度规划、OKR 对齐 |
| `rules/process/` | `10-MIGRATION-NOTES.md`、`11-GIT-SETTLEMENT-CHECKLIST.md` | 文档生命周期、评审流程 |

若未来建立 `rules/execution/`，必须带保留期和 closeout 规则；默认更推荐 `docs/execution/`，避免 `rules/` 再次混入历史任务噪音。

## 9. 最终目标

团队以后应形成如下阅读习惯：

- 先读 `README.md`
- 再读 `docs/AGENT_MEMORY_BRIEF.md`
- 再进入 `rules/README.md`
- 需要落地时再看 `repo/platform/contracts/`、`repo/data-platform/` 与服务 README
- 只有在追溯和审计时，才进入 `docs/archive/`

做到这一点，旧 `Rules/` 与 `Rules-V2/` 就不会再与新 `rules/` 形成三套并行真相源。
