# dm-service

## 服务职责
用户间私信线程与消息；审核与安全与 safety-service 协同。

## 拥有的数据
`dm_threads`, `dm_participants`, `dm_messages`。

## 对外接口
`/api/v1/dm/*` — 见 `OneLink/repo/platform/contracts/openapi/dm-service.yaml`。

## 依赖
identity、safety（评估）；发事件供 match 反馈归并。

## 不负责
AI 对话、推荐卡片排序。

## 文档来源
`OneLink/Rules/10-SERVICE-BOUNDARIES.md`
