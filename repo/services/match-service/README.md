# match-service

## 服务职责
找人请求、推荐结果集与卡片；**统一写入** `recommendation_feedbacks`（其他服务只发事件）。

## 拥有的数据
`find_requests`, `recommendation_result_sets`, `recommendation_cards`, `recommendation_feedbacks`。

## 对外接口
`/api/v1/match/*` — 见 `OneLink/repo/platform/contracts/openapi/match-service.yaml`。

## 依赖
profile（索引/向量）、safety（评估）、model-gateway（解析/排序等）。

## 不负责
用户私信内容存储、画像事实主写。

## 文档来源
`OneLink/Rules/10-SERVICE-BOUNDARIES.md`
