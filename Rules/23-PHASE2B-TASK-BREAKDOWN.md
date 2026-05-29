# Phase 2B Task Breakdown

## 1. 目的

本文件把 `rules/18-PHASE2B-CLOSURE-CHECKLIST.md` 转成任务分解文档，供团队按 sprint 或 issue 拆分。

## 2. Epic 列表

Phase 2B 拆成 6 个 Epic：

1. `Epic A`：Find 请求真实化
2. `Epic B`：Recommend 真实化
3. `Epic C`：DM 闭环
4. `Epic D`：Safety 闭环
5. `Epic E`：Admin 最小运营后台
6. `Epic F`：BFF / 测试 / 回写收口

## 3. Epic A：Find 请求真实化

任务：

- 定义找人请求最终状态枚举
- BFF 接入真实请求提交
- 前端接入请求状态查询
- 澄清问题链路接通
- 空结果恢复路径接通

交付物：

- OpenAPI
- BFF 路由
- App/Web 页面联调
- contract test

完成判定：

- 用户可真实提交请求并查看状态

## 4. Epic B：Recommend 真实化

任务：

- 推荐列表接真实服务
- 推荐详情接真实解释摘要
- 推荐反馈写回事件
- 空结果与降级态设计收口

交付物：

- recommendation API
- feedback 事件
- App/Web 联调

完成判定：

- 推荐卡不再是静态样例

## 5. Epic C：DM 闭环

任务：

- 首条私信草稿创建
- 首条私信提交审核
- 审核通过后进入线程
- 审核失败提示接通
- 后续消息线程接通

交付物：

- dm API
- safety 联动
- App/Web 消息页

完成判定：

- 用户可完成最小私信路径

## 6. Epic D：Safety 闭环

任务：

- 举报提交接口接通
- 拉黑接口接通
- 申诉状态查询接通
- 高风险提示文案收口
- 安全结果进入后台队列

交付物：

- safety API
- 审核文案 key
- Admin 联调

完成判定：

- 举报 / 拉黑 / 审核 / 申诉可回流

## 7. Epic E：Admin 最小运营后台

任务：

- 举报队列页面
- 审核结果查看页
- 申诉队列页面
- 最小审核动作按钮

交付物：

- admin routes
- admin 页面
- 审核运营说明

完成判定：

- Admin 不再是占位页

## 8. Epic F：BFF / 测试 / 回写收口

任务：

- BFF 聚合响应统一
- mock 更新
- contract test 更新
- smoke 更新
- README / rules 回写

交付物：

- BFF routes
- tests
- 文档回写

完成判定：

- 页面、BFF、服务、测试、文档一致

## 9. 推荐执行顺序

1. Epic A
2. Epic B
3. Epic C
4. Epic D
5. Epic E
6. Epic F

## 10. Sprint 拆分建议

### Sprint 1

- Epic A
- Epic B 前半

### Sprint 2

- Epic B 后半
- Epic C

### Sprint 3

- Epic D
- Epic E
- Epic F

## 11. 验收旅程

必须至少演示一次：

1. 提交找人请求
2. 收到推荐
3. 查看解释
4. 发起首条私信
5. 被安全审查放行或拦截
6. 举报或拉黑
7. 后台查看并处理

## 12. 与其他规则的关系

- 收口清单见 `rules/18-PHASE2B-CLOSURE-CHECKLIST.md`
- App 页面规格见 `rules/21-APP-SCREEN-SPECS.md`
- App/BFF 矩阵见 `rules/22-APP-BFF-API-MATRIX.md`
