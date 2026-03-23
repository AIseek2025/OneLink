# OneLink Project Structure

## 1. 文档目标
- 定义 OneLink 的统一目录结构
- 明确规划文档区与代码仓库区的边界
- 为后续多人协作提供稳定、不返工的目录基线

## 2. 顶层原则
- 单一主仓库起步，但**规划文档**与**代码仓库**必须分层
- `OneLink/Rules/` 与 `OneLink/docs/` 是规划区，不与 `OneLink/repo/` 混在同一 Git 根语义里
- 代码仓库根固定为 `OneLink/repo/`
- 公共契约优先放共享目录，避免服务间复制粘贴
- MVP 先把目录骨架做对，再逐步补实现

## 3. 推荐目录结构

```text
OneLink/
├── Rules/
│   ├── 00-EXECUTIVE-BLUEPRINT.md
│   ├── 01-PRODUCT-SYSTEM.md
│   ├── 02-TECH-ARCHITECTURE.md
│   ├── 03-AI-PROFILE-QUESTIONNAIRE.md
│   ├── 04-MATCHING-SAFETY-GOVERNANCE.md
│   ├── 05-MODEL-PLATFORM-ROADMAP.md
│   ├── 06-AUTORESEARCH-PAPERCLIP-INTEGRATION.md
│   ├── 07-ENGINEERING-RULES.md
│   ├── 08-DELIVERY-ROADMAP.md
│   ├── 09-PROJECT-STRUCTURE.md
│   ├── 10-SERVICE-BOUNDARIES.md
│   ├── 11-DATA-EVENT-MODEL.md
│   ├── 12-ARCHITECTURE-REVIEW.md
│   ├── 13-COMPOSER-1.5-EXECUTION-BRIEF.md
│   ├── 14-MVP-SQL-SCHEMA-DRAFT.md
│   ├── 15-MVP-OPENAPI-DRAFT.md
│   ├── 16-MVP-EVENT-SCHEMAS-DRAFT.md
│   ├── 17-MVP-SERVICE-CONTRACTS.md
│   ├── 18-COMPOSER-2-FAST-EXECUTION-BRIEF.md
│   ├── 19-CONTEXT-MEMORY-ARCHITECTURE.md
│   ├── 20-FIRST-RUNNABLE-VERTICAL-SLICE-BRIEF.md
│   └── A1-FULL-AUDIT-REPORT.md
├── docs/
│   └── historical/
└── repo/
    ├── apps/
    │   ├── web/
    │   ├── admin/
    │   └── ops-console/
    ├── services/
    │   ├── api-gateway/
    │   ├── bff/
    │   ├── identity-service/
    │   ├── profile-service/
    │   ├── context-service/
    │   ├── ai-chat-service/
    │   ├── dm-service/
    │   ├── question-service/
    │   ├── match-service/
    │   ├── safety-service/
    │   ├── model-gateway/
    │   ├── ranking-service/
    │   ├── notification-service/
    │   ├── payment-service/
    │   └── media-service/
    ├── platform/
    │   ├── contracts/
    │   ├── shared-libs/
    │   ├── sdk/
    │   ├── auth/
    │   ├── observability/
    │   └── feature-flags/
    ├── data-platform/
    │   ├── event-schemas/
    │   ├── db-schema/
    │   ├── analytics/
    │   ├── feature-pipelines/
    │   ├── vector-jobs/
    │   └── retention-policies/
    ├── ai-platform/
    │   ├── prompts/
    │   ├── evaluators/
    │   ├── datasets/
    │   ├── training/
    │   ├── inference/
    │   ├── research-loop/
    │   └── safety-bench/
    ├── infra/
    │   ├── docker/
    │   ├── kubernetes/
    │   ├── terraform/
    │   ├── secrets-template/
    │   ├── ci/
    │   └── monitoring/
    ├── scripts/
    │   ├── local/
    │   ├── migration/
    │   ├── backfill/
    │   └── release/
    ├── tests/
    │   ├── contract/
    │   ├── integration/
    │   ├── e2e/
    │   ├── load/
    │   └── safety/
    └── tools/
        ├── generators/
        ├── linters/
        └── dev-cli/
```

## 4. 目录职责

### 4.1 `Rules/`
- OneLink 的唯一规划与执行规范
- 任何架构、边界、数据模型、任务书都先在这里冻结

### 4.2 `docs/`
- 早期想法、历史草稿、灵感输入
- 不再作为最终执行规范

### 4.3 `repo/apps/`
- 用户直接使用的前端应用
- `web/`：主站、聊天入口、主页、找人、名片、私信
- `admin/`：内容治理与风控后台
- `ops-console/`：运营与数据工作台（非 MVP）

### 4.4 `repo/services/`
- 在线业务服务
- 一服务一职责
- 不放训练代码，不放共享工具杂项

### 4.5 `repo/platform/`
- 全局平台能力
- 包括契约、共享库、SDK、鉴权、可观测性、feature flag

### 4.6 `repo/data-platform/`
- 数据契约、批流任务、特征产物、保留策略
- 是业务数据治理中心，不是模型训练仓库

### 4.7 `repo/ai-platform/`
- Prompt、评估器、训练代码、推理适配、研究循环
- 所有模型资产和实验资产从这里治理

### 4.8 `repo/infra/`
- 部署与基础设施即代码
- 不允许把环境临时脚本散落到其他目录

### 4.9 `repo/tests/`
- 按测试类型分层
- 不把测试混在业务目录根部

## 5. 语言落位

### 5.1 前端
- `repo/apps/web`
- `repo/apps/admin`
- `repo/apps/ops-console`
- 主语言：`TypeScript`

### 5.2 Rust
- `repo/services/api-gateway`
- `repo/services/bff`
- `repo/services/identity-service`
- `repo/services/profile-service`
- `repo/services/context-service`
- `repo/services/ai-chat-service`
- `repo/services/dm-service`
- `repo/services/question-service`
- `repo/services/model-gateway`
- `repo/services/match-service`
- `repo/services/ranking-service`
- `repo/services/safety-service`
- MVP 核心后端默认全部采用 `Rust`

### 5.3 Go
- `repo/services/notification-service`
- `repo/services/payment-service`
- `repo/services/media-service`
- 非 MVP 辅助服务或平台工具

### 5.4 Python
- `repo/ai-platform/*`
- `repo/data-platform/feature-pipelines`
- `repo/data-platform/vector-jobs`

## 6. MVP 最小子集

MVP 不必一次性建完所有未来目录，先落地以下最小子集：

```text
OneLink/
├── Rules/
├── docs/
└── repo/
    ├── apps/
    │   ├── web/
    │   └── admin/
    ├── services/
    │   ├── api-gateway/
    │   ├── bff/
    │   ├── identity-service/
    │   ├── profile-service/
    │   ├── context-service/
    │   ├── ai-chat-service/
    │   ├── dm-service/
    │   ├── question-service/
    │   ├── match-service/
    │   ├── safety-service/
    │   └── model-gateway/
    ├── platform/
    │   ├── contracts/
    │   ├── shared-libs/
    │   └── observability/
    ├── data-platform/
    │   ├── event-schemas/
    │   └── db-schema/
    ├── ai-platform/
    │   ├── prompts/
    │   ├── evaluators/
    │   └── inference/
    ├── infra/
    ├── tests/
    └── scripts/
```

## 7. 协作规则
- 任何服务新增接口，先更新 `repo/platform/contracts`
- 任何表结构变更，先更新 `repo/data-platform/db-schema`
- 任何事件新增，先更新 `repo/data-platform/event-schemas`
- 任何 Prompt 变更，先更新 `repo/ai-platform/prompts`
- 任何模型评估变更，先更新 `repo/ai-platform/evaluators`
- 规划修改先改 `Rules/`，代码修改先改 `repo/`

## 8. 不建议的做法
- 不要把前端、服务端、训练脚本混在一个 `src/` 目录里
- 不要在服务目录里私自放一套共享库
- 不要把历史设想文档继续和执行文档放在同一层
- 不要让 `Rules/` 与 `repo/` 的目录口径长期分叉
- 不要一开始就拆成十几个独立仓库

## 9. 这一阶段后最适合谁接手
- 当前目录规范冻结后，下一步最适合让 `Composer 1.5` 基于现有骨架制定第一条可运行闭环的实现任务书
- `Opus 4.6` 更适合在首条闭环实现后做高标准架构复审
