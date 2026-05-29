# BFF Client Contract Freeze

## 1. 目的

本文件用于冻结 App/Web 开工前必须稳定的客户端契约边界。

目标不是一次性穷尽所有字段，而是保证：

- App 与 Web 不各自发明协议
- BFF 成为唯一客户端聚合入口
- 关键主链路可以并行开发
- 契约变更有明确 owner 和升级规则

## 2. 冻结范围

本文件冻结的是“接口主题和语义边界”，不是每个字段的最终实现细节。

Phase A0-A1 必须冻结：

- 认证与会话
- Lumi 聊天
- 画像确认
- 基础设置

Phase A2 必须追加冻结：

- 找人请求
- 推荐列表与推荐详情
- 推荐反馈

Phase A3 必须追加冻结：

- 首条私信
- 举报 / 拉黑 / 申诉
- 系统消息与审核反馈

## 3. 客户端调用原则

- 客户端只调用 `BFF`
- 客户端不直接调用内部 owner service
- 客户端不拼接内部 trace、internal auth 或内部 schema
- 所有客户端响应必须能映射到稳定的 UI 状态，而不是内部技术状态

## 4. 契约主题

## 4.1 认证与会话

必须冻结：

- 注册
- 登录
- 刷新会话
- 获取当前会话用户摘要
- 退出登录

必须统一：

- 错误码
- 会话过期策略
- 多设备策略文案
- 首登引导 flag

## 4.2 Lumi 聊天

必须冻结：

- 获取会话列表
- 创建 / 继续会话
- 发送消息
- 获取消息流
- 重试失败消息

必须统一：

- conversation identity
- streaming / polling 语义
- degraded / fallback 表达
- 系统提示格式

## 4.3 画像确认

必须冻结：

- 获取待确认画像事实
- 接受 / 拒绝 / 稍后处理
- 提交轻量纠错
- 获取事实来源说明摘要

必须统一：

- fact identity
- source / confidence 解释口径
- 接受与拒绝事件语义

## 4.4 找人请求

必须冻结：

- 创建找人请求
- 获取请求状态
- 提交澄清答案
- 获取推荐结果是否就绪

必须统一：

- request status 枚举
- clarification question 结构
- empty result 语义

## 4.5 推荐与反馈

必须冻结：

- 获取推荐列表
- 获取推荐详情
- 获取推荐解释摘要
- 提交反馈
- 触发连接动作

必须统一：

- recommendation identity
- explanation 版本
- feedback type
- connection eligibility

## 4.6 私信与安全

必须冻结：

- 创建首条私信草稿
- 提交首条私信
- 查询审核状态
- 举报
- 拉黑
- 申诉状态查询

必须统一：

- safety decision 枚举
- report / block / appeal identity
- 用户可见提示语义

## 4.7 设置与语言

必须冻结：

- 获取当前设置
- 更新语言与地区
- 更新通知语言
- 更新通知开关
- 获取隐私与合规入口摘要

必须统一：

- `locale`
- `region`
- `timezone`
- `content_language`
- `notification_language`

## 5. 统一响应规则

客户端可见响应必须包含：

- 稳定业务 id
- 请求时间或版本戳
- 可选 `trace_id`
- 明确状态枚举
- 可选用户可见 message key

禁止：

- 直接暴露内部 `x-internal-token` 语义
- 暴露 owner service 内部错误细节
- 一个接口同时返回多种互相矛盾的状态表达

## 6. 统一错误规则

错误必须至少分成：

- `auth_error`
- `network_error`
- `validation_error`
- `rate_limited`
- `safety_blocked`
- `temporarily_unavailable`
- `retryable`

客户端必须按 message key + code 渲染，不自行硬编码业务逻辑文案。

## 7. 变更治理

契约变更必须遵守：

- 新增字段优先，不破坏旧字段
- 删除字段必须先标记废弃
- App/Web 同步评审后再切换
- OpenAPI、mock、前端 DTO、测试说明同步更新

以下变更必须开评审：

- 枚举新增或重命名
- 登录态变化
- 推荐反馈语义变化
- 安全决策语义变化
- i18n / locale 字段变化

## 8. 开发前冻结清单

开工前必须至少冻结以下资源：

- `BFF OpenAPI`
- DTO / TS types
- mock response
- analytics event map
- error code map
- locale field map

## 9. 验收标准

若满足以下条件，才视为契约冻结完成：

- App/Web 都只依赖 BFF
- 所有主链路都有 mock 示例
- App/Web 共享错误码和状态枚举
- OpenAPI、mock、测试说明和页面状态流一致

## 10. 与其他规则的关系

- App 页面和状态流见 `rules/16-APP-IA-AND-STATE-FLOWS.md`
- App 总规划见 `rules/15-APP-DELIVERY-PLAN.md`
- Phase 2B 闭环见 `rules/18-PHASE2B-CLOSURE-CHECKLIST.md`
