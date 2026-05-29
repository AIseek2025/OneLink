# safety-service

## 服务职责
风险研判、举报工单、处罚与申诉；输出轻量信誉/风险信号（MVP 无独立 trust-service）。

## 拥有的数据
`risk_assessments`, `report_tickets`, `moderation_actions`, `appeal_cases`, `user_blocks`。

## 对外接口
`/api/v1/safety/*` — 见 `OneLink/repo/platform/contracts/openapi/safety-service.yaml`。

## 依赖
可经 model-gateway 做语义审核；消费 match/dm 相关事件（后续）。

## 不负责
推荐主排序、AI 对话上下文存储。

## 当前规范入口
当前裁决顺序为：运行代码与测试 > `repo/platform/contracts/` > `repo/data-platform/` > `OneLink/rules/` > `OneLink/docs/archive/`。

## 文档来源
当前规范：`OneLink/rules/03-SERVICE-BOUNDARIES.md`、`OneLink/rules/06-MATCHING-SAFETY-GOVERNANCE.md`、`OneLink/rules/09-NEXT-DEVELOPMENT-PLAN.md`。  
历史依据：`OneLink/docs/archive/rules-legacy-2026-05-15/Rules/10-SERVICE-BOUNDARIES.md`, `OneLink/docs/archive/rules-legacy-2026-05-15/Rules/04-MATCHING-SAFETY-GOVERNANCE.md`
