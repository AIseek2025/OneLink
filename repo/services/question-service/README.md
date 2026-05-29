# question-service

## 服务职责
结构化问卷、必填题包、投放与答案；驱动冷启动画像密度。

## 拥有的数据
`question_templates`, `question_variants`, `question_deliveries`, `question_answers`。

## 对外接口
`/api/v1/questions/*` — 见 `OneLink/repo/platform/contracts/openapi/question-service.yaml`。

## Phase C 最小实现（当前）
- 默认端口 **8086**（冻结；与 OpenAPI `servers` 一致）。starter 种子题 **in-memory**，非完整题库/持久化。
- `POST /answers` 在首次 `answered` 写入时向 `context-service` `POST /internal/events/receive` relay **`question.answered.v1`**（payload 最小字段与 `repo/data-platform/event-schemas/question.answered.v1.json` 一致），**不**直写 profile。
- **`GET /questions/status` 与 `GET /questions/completion`** 返回同一 JSON，表示**问卷域**进度（starter/profile/optional 计数）；与 **`profile-service` `GET /api/v1/profile/me/completion` 五维画像完成度无关、不联动**。
- **BFF**：`GET /api/v1/bff/chat/init` 只把 pending 的 **`items[]`** 填入 `pending_questions`；**`GET /api/v1/bff/onboarding`** 另把 **`GET /questions/completion`** 透传为响应字段 **`progress`**（失败时 BFF 返回带 `degraded` 的最小对象）。BFF **不**持有问卷真相，**不**把画像五维完成度写入 `progress`。
- 环境变量：`IDENTITY_SERVICE_BASE_URL`、`CONTEXT_SERVICE_BASE_URL`、`INTERNAL_SHARED_SECRET`（与 context 入站 relay 校验一致）。

## 依赖
identity；context（事件消费与记忆）；profile 仅经既有 `profile.memory_projection.requested.v1` 投影。

## 不负责
自然语言对话内追问的实现细节可部分在 ai-chat，但题库与投放归本服务。

## 当前规范入口
当前裁决顺序为：运行代码与测试 > `repo/platform/contracts/` > `repo/data-platform/` > `OneLink/rules/` > `OneLink/docs/archive/`。

## 文档来源
当前规范：`OneLink/rules/03-SERVICE-BOUNDARIES.md`、`OneLink/rules/04-DATA-EVENT-CONTRACTS.md`、`OneLink/rules/09-NEXT-DEVELOPMENT-PLAN.md`。  
历史依据：`OneLink/docs/archive/rules-legacy-2026-05-15/Rules/03-AI-PROFILE-QUESTIONNAIRE.md`, `OneLink/docs/archive/rules-legacy-2026-05-15/Rules/10-SERVICE-BOUNDARIES.md`
