# App Screen Specs

## 1. 目的

本文件把 `rules/15-APP-DELIVERY-PLAN.md` 与 `rules/16-APP-IA-AND-STATE-FLOWS.md` 继续拆到页面规格层。

每个页面至少定义：

- 页面目标
- 核心用户任务
- 关键状态
- 依赖数据
- 埋点
- 风险与边界

本文件优先覆盖 App Phase A0-A3。

## 2. 启动页

目标：

- 快速完成品牌识别、环境初始化和会话恢复判断

核心任务：

- 检查本地登录态
- 决定进入登录页、引导页还是会话首页

关键状态：

- `booting`
- `restoring_session`
- `session_restored`
- `no_session`
- `boot_failed`

依赖数据：

- 本地 token
- 远端 session refresh 结果

埋点：

- `app_boot_started`
- `app_boot_finished`
- `app_boot_failed`

风险与边界：

- 启动页停留时间不能过长
- 出错后必须给用户可恢复入口

## 3. 登录页

目标：

- 让回访用户稳定进入系统

核心任务：

- 输入账号
- 输入密码或验证码
- 登录

关键状态：

- `idle`
- `submitting`
- `login_success`
- `login_failed`
- `session_expired`

依赖数据：

- 登录接口
- 错误码映射

埋点：

- `login_submit`
- `login_success`
- `login_failed`

风险与边界：

- 不展示内部错误细节
- 需要弱网与超时文案

## 4. 注册页

目标：

- 让新用户最短路径完成账号创建

核心任务：

- 输入注册信息
- 同意必要条款
- 完成注册

关键状态：

- `idle`
- `validating`
- `submitting`
- `registered`
- `requires_verification`
- `failed`

依赖数据：

- 注册接口
- 首登 flag

埋点：

- `register_submit`
- `register_success`
- `register_failed`

风险与边界：

- 不在注册页堆过多资料字段
- 合规条款链接必须可见

## 5. 会话列表页

目标：

- 展示 Lumi 关系入口和最近对话

核心任务：

- 查看最近会话
- 进入已有会话
- 发起新会话

关键状态：

- `loading_list`
- `empty_list`
- `list_ready`
- `load_failed`

依赖数据：

- conversation list
- 最近消息摘要

埋点：

- `conversation_list_view`
- `conversation_open`
- `conversation_create`

风险与边界：

- 空态要引导用户开始第一轮对话

## 6. 聊天页

目标：

- 成为用户与 Lumi 的主交互界面

核心任务：

- 发送消息
- 接收回复
- 查看系统提示
- 重试失败消息

关键状态：

- `ready`
- `sending`
- `reply_loading`
- `reply_streaming`
- `reply_completed`
- `degraded`
- `failed`

依赖数据：

- chat send
- conversation fetch
- fallback reason

埋点：

- `chat_view`
- `chat_send`
- `chat_reply_received`
- `chat_retry`

风险与边界：

- 降级与失败状态要可辨识
- 不允许静默丢消息

## 7. 画像确认卡

目标：

- 让用户以最低负担确认关键事实

核心任务：

- 查看候选画像事实
- 接受 / 拒绝 / 稍后
- 进入深度编辑说明

关键状态：

- `hidden`
- `pending`
- `accepted`
- `rejected`
- `snoozed`

依赖数据：

- pending facts
- confidence/source 摘要

埋点：

- `profile_fact_exposed`
- `profile_fact_accept`
- `profile_fact_reject`
- `profile_fact_snooze`

风险与边界：

- 不能让用户误以为系统已永久锁定事实

## 8. 找人输入页

目标：

- 让用户快速表达连接目标

核心任务：

- 输入找人意图
- 提交请求

关键状态：

- `draft`
- `submitting`
- `submitted`
- `clarification_needed`
- `failed`

依赖数据：

- find request create
- clarification question

埋点：

- `find_request_started`
- `find_request_submitted`
- `find_request_failed`

风险与边界：

- 不能默认把请求当作已成功检索

## 9. 找人条件补充页

目标：

- 在用户负担最小的前提下补齐必要条件

核心任务：

- 回答澄清问题
- 补充条件
- 再次提交

关键状态：

- `loading_questions`
- `editing_answers`
- `submitting_answers`
- `completed`

依赖数据：

- clarification schema
- existing draft

埋点：

- `clarification_view`
- `clarification_submit`

风险与边界：

- 条件补充不能变成长表单问卷

## 10. 推荐列表页

目标：

- 让用户高效消费推荐结果

核心任务：

- 浏览卡片
- 查看解释摘要
- 做出反馈

关键状态：

- `waiting_results`
- `results_ready`
- `empty_result`
- `failed`

依赖数据：

- recommendation list
- request status

埋点：

- `recommendation_list_view`
- `recommendation_card_exposed`

风险与边界：

- 空结果要给出真实恢复路径

## 11. 推荐详情页

目标：

- 让用户理解为什么被推荐给自己

核心任务：

- 查看候选详情
- 查看解释
- 发起连接

关键状态：

- `loading_detail`
- `detail_ready`
- `action_disabled`
- `failed`

依赖数据：

- recommendation detail
- explanation summary
- connection eligibility

埋点：

- `recommendation_detail_view`
- `recommendation_explanation_view`
- `recommendation_connect_start`

风险与边界：

- 解释必须是可审计摘要，不泄露内部敏感打分逻辑

## 12. 推荐反馈弹层

目标：

- 收集用户对推荐的低摩擦反馈

核心任务：

- 喜欢
- 跳过
- 稍后看

关键状态：

- `opened`
- `submitting`
- `submitted`
- `failed`

依赖数据：

- recommendation feedback

埋点：

- `recommendation_feedback_open`
- `recommendation_feedback_submit`

风险与边界：

- 提交失败不能吞掉

## 13. 首条私信页

目标：

- 帮助用户发起第一次安全连接

核心任务：

- 编辑首条消息
- 提交审核
- 查看审核结果

关键状态：

- `draft`
- `under_review`
- `approved`
- `blocked`
- `sent`

依赖数据：

- dm draft
- safety decision

埋点：

- `dm_first_message_submit`
- `dm_first_message_approved`
- `dm_first_message_blocked`

风险与边界：

- 必须突出风险解释和后续动作

## 14. 举报页

目标：

- 让用户快速完成举报而不是放弃

核心任务：

- 选择举报原因
- 补充说明
- 提交举报

关键状态：

- `editing`
- `submitting`
- `submitted`
- `failed`

依赖数据：

- report categories
- report submit

埋点：

- `report_open`
- `report_submit`

风险与边界：

- 高风险文案必须走审核版

## 15. 设置首页

目标：

- 成为用户管理偏好、语言、通知和隐私的入口

核心任务：

- 查看设置分组
- 进入语言 / 通知 / 隐私 / 数据权利页

关键状态：

- `loading`
- `ready`
- `failed`

依赖数据：

- settings summary

埋点：

- `settings_view`
- `settings_section_open`

风险与边界：

- 设置项不能只在前端变更，必须能追溯到落库路径

## 16. 语言与地区页

目标：

- 让用户独立管理 `locale`、`region`、`timezone`、`notification_language`

核心任务：

- 修改语言
- 修改地区
- 修改时区
- 修改通知语言

关键状态：

- `loading`
- `editing`
- `saving`
- `saved`
- `failed`

依赖数据：

- settings fetch/update
- locale registry

埋点：

- `locale_setting_view`
- `locale_setting_save`

风险与边界：

- 字段不能混用
- 保存后要影响后续请求行为

## 17. 数据权利入口页

目标：

- 给用户稳定、可信的数据导出/删除/纠正入口

核心任务：

- 进入查看/导出/删除/纠正流程
- 查看申请状态

关键状态：

- `ready`
- `request_submitting`
- `request_submitted`
- `failed`

依赖数据：

- compliance summary
- export/delete/correction actions

埋点：

- `data_rights_view`
- `data_export_request`
- `data_delete_request`
- `data_correction_request`

风险与边界：

- 不允许只有说明页没有动作入口

## 18. 评审门禁

每个页面进入开发前，必须补齐：

- 页面草图
- 状态流
- BFF 接口
- 埋点
- 错误态
- 合规文案要求

## 19. 与其他规则的关系

- App 总规划见 `rules/15-APP-DELIVERY-PLAN.md`
- IA 和状态流见 `rules/16-APP-IA-AND-STATE-FLOWS.md`
- BFF 契约冻结见 `rules/17-BFF-CLIENT-CONTRACT-FREEZE.md`
