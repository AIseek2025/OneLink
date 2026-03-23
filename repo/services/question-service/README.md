# question-service

## 服务职责
结构化问卷、必填题包、投放与答案；驱动冷启动画像密度。

## 拥有的数据
`question_templates`, `question_variants`, `question_deliveries`, `question_answers`。

## 对外接口
`/api/v1/questions/*` — 见 `OneLink/repo/platform/contracts/openapi/question-service.yaml`。

## 依赖
identity；答题后发事件供 profile 写事实。

## 不负责
自然语言对话内追问的实现细节可部分在 ai-chat，但题库与投放归本服务。

## 文档来源
`OneLink/Rules/03-AI-PROFILE-QUESTIONNAIRE.md`, `OneLink/Rules/10-SERVICE-BOUNDARIES.md`
