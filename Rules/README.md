# OneLink Rules

生成日期：2026-05-15

本目录是 OneLink 后续开发的唯一规则入口，合并自旧 `Rules/`、`Rules-V2/`、`docs/0327` 有价值研究资料与 `AUDIT_INVENTORY_REPORT_2026-05-15.md`。

旧资料已归档到：`docs/archive/rules-legacy-2026-05-15/`。

## 阅读顺序

1. `00-CONSTITUTION.md`：不可破坏的产品与架构铁律。
2. `01-PRODUCT-PLATFORM.md`：产品定义、用户旅程、三端平台策略。
3. `02-SYSTEM-ARCHITECTURE.md`：Rust 为主、Go 配合的系统架构。
4. `03-SERVICE-BOUNDARIES.md`：服务主责、主写权和拆分节奏。
5. `04-DATA-EVENT-CONTRACTS.md`：数据、事件、契约和可回放约束。
6. `05-MEMORY-AGENT-CONTEXT.md`：AI 好朋友、长期记忆、上下文、Skill 与选择性遗忘。
7. `06-MATCHING-SAFETY-GOVERNANCE.md`：找人、匹配、安全、风控、提示词防火墙。
8. `07-ENGINEERING-QUALITY.md`：工程规则、测试、CI、部署与生产门禁。
9. `08-ROADMAP.md`：阶段路线图。
10. `09-NEXT-DEVELOPMENT-PLAN.md`：从当前代码状态出发的下一步开发计划。
11. `10-MIGRATION-NOTES.md`：规则迁移、archive 用法与 Git 收口说明。
12. `11-GIT-SETTLEMENT-CHECKLIST.md`：Git 层目录迁移与规则收口执行清单。
13. `12-CAPACITY-AND-SLO.md`：容量目标、SLO、成本预算和压测门禁。
14. `13-GLOBAL-I18N-AND-COMPLIANCE.md`：全球化、多语言、区域合规和数据驻留策略。
15. `14-DELIVERY-CLOSURE-PLAN.md`：基于 iteration 1-60 审计的剩余工作总收口规划。
16. `15-APP-DELIVERY-PLAN.md`：App 作为核心产品载体的专项开发总规划。
17. `16-APP-IA-AND-STATE-FLOWS.md`：App 信息架构、页面树与核心状态流。
18. `17-BFF-CLIENT-CONTRACT-FREEZE.md`：App/Web 共用客户端契约冻结范围与治理规则。
19. `18-PHASE2B-CLOSURE-CHECKLIST.md`：匹配、私信、安全、后台闭环收口清单。
20. `19-CAPACITY-SLO-EXECUTION-HANDBOOK.md`：容量、SLO、压测和故障注入执行手册。
21. `20-GLOBAL-I18N-COMPLIANCE-EXECUTION-HANDBOOK.md`：全球化、多语言与合规执行手册。
22. `21-APP-SCREEN-SPECS.md`：App 逐页面规格说明。
23. `22-APP-BFF-API-MATRIX.md`：App 页面到 BFF 接口矩阵。
24. `23-PHASE2B-TASK-BREAKDOWN.md`：Phase 2B 任务拆分与 sprint 建议。
25. `24-CAPACITY-TEST-PLAN.md`：容量与 SLO 测试计划模板。
26. `25-I18N-COMPLIANCE-CHECKLIST.md`：国际化与合规上线检查表。
27. `26-WEB-APP-COMPLETION-PLAN.md`：Web 与 App 双端开发完工、统一验收与上线门禁执行计划。
28. `27-PRE-LAUNCH-TASK-BOOK.md`：上线前准备全套任务规划书，覆盖预发、门禁、发布、回滚、告警与放行决策。

## 权威性

- 当前代码与可运行联调结果优先于历史规划。
- 本目录优先于归档目录中的旧 `Rules/` 与 `Rules-V2/`。
- `repo/` 内实际契约、SQL、事件 schema 与测试文档是工程事实源；规则变更必须级联到这些实现资产。
- `docs/` 下研究资料只作为背景输入，不能直接当当前交付状态。
- 当前规范裁决顺序为：运行代码与测试 > `repo/platform/contracts/` > `repo/data-platform/` > `rules/` > `docs/archive/`。
- `docs/archive/` 仅用于历史追溯、字段来源核对与审计说明，不作为团队默认执行入口，也不得单独作为新变更依据。

## 合并判断

本次没有在旧 `Rules-V2` 上原地继续追加，而是新建 `rules/`：

- 旧 `Rules` 是 V1 总体规划，产品、服务和路线图仍有价值，但部分前端优先级与当前工程状态已过时。
- 旧 `Rules-V2` 是 V2 架构升级，宪法、memory/session/optimization/persona/data/contract 更适合作为新主干。
- 旧 `Rules-V2/EXECUTION` 含大量历史波次、brief、review 和 closeout，不应继续混在团队日常真相源中。
- 新 `rules/` 只保留稳定规则与下一步计划；历史任务书统一归档。

## 旧规则映射

为降低迁移成本，旧规则主题与当前文档的主要对应关系如下：

| 历史资料 | 当前入口 |
|------|------|
| `Rules/10-SERVICE-BOUNDARIES.md` | `rules/03-SERVICE-BOUNDARIES.md` |
| `Rules/11-DATA-EVENT-MODEL.md` | `rules/04-DATA-EVENT-CONTRACTS.md` |
| `Rules/14-MVP-SQL-SCHEMA-DRAFT.md` | `rules/04-DATA-EVENT-CONTRACTS.md` + `repo/data-platform/db-schema/` |
| `Rules/15-MVP-OPENAPI-DRAFT.md` | `rules/04-DATA-EVENT-CONTRACTS.md` + `repo/platform/contracts/` |
| `Rules/16-MVP-EVENT-SCHEMAS-DRAFT.md` | `rules/04-DATA-EVENT-CONTRACTS.md` + `repo/data-platform/event-schemas/` |
| `Rules/19-CONTEXT-MEMORY-ARCHITECTURE.md` | `rules/05-MEMORY-AGENT-CONTEXT.md` |
| `Rules/20-FIRST-RUNNABLE-VERTICAL-SLICE-BRIEF.md` | `rules/08-ROADMAP.md` + `rules/09-NEXT-DEVELOPMENT-PLAN.md` + `repo/tests/integration/CHAT_MEMORY_PROFILE_SLICE.md` |
| `Rules-V2/ARCHITECTURE/system-overview.md` | `rules/02-SYSTEM-ARCHITECTURE.md` |
| `Rules-V2/ARCHITECTURE/session-layer.md` | `rules/05-MEMORY-AGENT-CONTEXT.md` |
| `Rules-V2/ARCHITECTURE/memory-layer.md` | `rules/05-MEMORY-AGENT-CONTEXT.md` |
| `Rules-V2/DATA/data-model.md` | `rules/04-DATA-EVENT-CONTRACTS.md` |
| `Rules-V2/CONTRACTS/context-service-contract.md` | `rules/04-DATA-EVENT-CONTRACTS.md` + `repo/platform/contracts/internal/context-service.yaml` |

## 归档使用边界

- `docs/archive/rules-legacy-2026-05-15/Rules-V2/EXECUTION/` 中的 dispatch、brief、review、closeout 全部视为历史任务资料，不再作为当前默认发单入口。
- 若需要追溯某个字段、事件或阶段性验收基线，应先在 `rules/` 中定位当前章节，再按需回查 archive。
- 若 archive 与 `rules/`、代码或工程契约冲突，以当前代码、工程资产与 `rules/` 为准。

## 当前波次工单出口

`rules/` 只承载稳定规则、路线图和下一步计划，不承载频繁变化的日常 dispatch、brief、review 或 closeout。

当前波次工单默认放入 `docs/execution/<yyyymmdd>-<topic>/` 或团队 issue tracker。若某个工单结论影响产品、架构、契约、容量、安全或工程门禁，必须在 closeout 后回写到 `rules/`、`repo/platform/contracts/` 或 `repo/data-platform/`，不能只停留在执行记录里。
