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

## 文档来源
`OneLink/Rules/10-SERVICE-BOUNDARIES.md`, `OneLink/Rules/04-MATCHING-SAFETY-GOVERNANCE.md`
