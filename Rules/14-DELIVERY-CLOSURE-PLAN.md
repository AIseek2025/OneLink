# Delivery Closure Plan

## 1. 目的

本文件用于承接 OneLink 在 iteration 1-60 审计后的剩余工作，回答一个具体问题：

- 在 autopilot 已完成定义 phase 之后，哪些工作仍未达到“整体计划开发到位”标准
- 团队应该按什么顺序继续开发和修复
- 每一块工作的验收标准是什么

本文件是 `rules/08-ROADMAP.md`、`rules/09-NEXT-DEVELOPMENT-PLAN.md`、`rules/12-CAPACITY-AND-SLO.md`、`rules/13-GLOBAL-I18N-AND-COMPLIANCE.md` 的收口版执行规划。

## 2. 审计后的总判断

OneLink 当前状态不是“未完成任何主线”，而是：

- phase 审计链已完成
- 规则体系和工程事实源已基本收口
- 核心 Rust 服务、Web 基础壳、BFF、契约、部分 capacity/compliance 骨架已成型

但以下四个方向仍未达到“整体计划开发到位”的标准：

1. `CodeMaster` 控制面回归未完全全绿
2. App + Web 双端目标未完整落地，App 仍缺位
3. Phase 2B 的用户闭环未完成，Find/Admin 等仍有占位能力
4. capacity/SLO 与 global/i18n/compliance 仍主要停留在规则、契约和骨架层

## 3. 交付铁律

后续所有收口开发必须遵守：

- 不再以“已有 phase 已完成”为理由跳过剩余缺口治理
- 不再新增大而散的新方向，先把已承诺的 App、闭环、容量、全球化做完
- 任何新增规划都必须落到 `repo/` 内代码、契约、脚本、测试和运行门禁
- 文档、代码、测试、报告四者必须同步，不允许继续出现“代码修了但证据没回写”的情况

## 4. 剩余工作总清单

### 4.1 Track A：控制面收口

目标：让 `CodeMaster` 控制面对 OneLink 的完成态管理、回归测试和恢复面完全一致。

必须完成：

- 修正 `test_autopilot_step_marks_finished_when_final_phase_is_approved` 的旧断言
- 确保 `phased_autopilot.py`、`phased_autopilot_supervisor.py`、`project_fleet.py`、`project_fleet_supervisor.py` 的完成态字段和 fallback 契约一致
- 明确 `last_audited_iteration`、`last_audit_result`、`last_audit_decision`、`last_completion` 的对外读取口径
- 重新跑全量相关测试并把结果写回控制面报告

验收标准：

- `tests/test_phased_autopilot.py`
- `tests/test_phased_autopilot_supervisor.py`
- `tests/test_project_fleet.py`
- `tests/test_project_fleet_runtime.py`

以上测试全部通过，且 OneLink 现场状态满足：

- `state.json` 保留完成态审计字段
- `loop_heartbeat.json` 为 terminal 状态
- `supervisor_heartbeat.json` 为 terminal 状态
- `project_state.json` 为 `active=false`
- `project_recovery_state.json` 不再尝试恢复已完成项目

### 4.2 Track B：App + Web 双端补齐

目标：把 `Phase 2A / P1` 从“只有 Web + 占位 App 策略”补成真正的双端产品基座。

必须完成：

- 确立 App 技术栈和 monorepo 目录
- 落地独立 App 工程
- 冻结 App/Web 共享 BFF 契约、埋点和设计 token
- 实现 App 核心闭环：注册、登录、Lumi 聊天、画像确认、找人输入壳、推荐壳、设置基础页
- 保证 Web 与 App 的能力边界清晰：Web 承担长表单与后台，App 承担主用户链路

详细方案见 `rules/15-APP-DELIVERY-PLAN.md`。

执行级资料见：

- `rules/16-APP-IA-AND-STATE-FLOWS.md`
- `rules/17-BFF-CLIENT-CONTRACT-FREEZE.md`

验收标准：

- `repo/apps/` 下存在正式 App 工程，而不是 README 占位
- App/Web 共享 BFF 契约和埋点规范
- 真实可演示的双端路径：注册 -> 登录 -> 聊天 -> 画像确认 -> 找人意图输入
- App 的构建、lint、test、preview 文档可重复执行

### 4.3 Track C：Phase 2B 闭环收口

目标：把 `Find`、`Recommend`、`DM`、`Safety`、`Admin` 从占位或半成品状态补成可用闭环。

必须完成：

- `FindPage` 从“后续版本上线”改为真实候选与解释路径
- `AdminPage` 从占位页升级为最小可运营后台
- `match-service`、`dm-service`、`safety-service` 完成真实功能接线，而不仅是 contract 或内存态样例
- 完成“推荐 -> 解释 -> 首条私信 -> 风控 -> 举报/拉黑 -> 反馈回流”的闭环

验收标准：

- App/Web 用户都能完成推荐浏览、发起连接、首条私信、举报/拉黑
- 管理后台至少能查看审核队列、风控结果、举报申诉
- `Phase 2B` contract test、smoke、BFF 路由、前端页面三者一致

执行级清单见 `rules/18-PHASE2B-CLOSURE-CHECKLIST.md`。

### 4.4 Track D：Capacity / SLO 落地

目标：把 `rules/12-CAPACITY-AND-SLO.md` 从原则文件变成发布门禁。

必须完成：

- 为关键服务补齐 `health`、`ready`、`metrics` 或明确的等价观测面
- 建立注册、登录、聊天、context build、画像读取、推荐请求的压测脚本
- 建立 provider timeout、rate limit、fallback、cache miss 等故障注入脚本
- 形成压测报告模板：`p50/p95/p99/error rate/resource usage/cost`
- 把压测结果回写到 `rules/12-CAPACITY-AND-SLO.md` 或服务 README

验收标准：

- 共享环境发布前必须有压测结果和回滚说明
- 新增高成本模型能力必须声明预算、fallback 和 kill switch
- 核心链路达到阶段 SLO 基线，否则阻塞发布

执行级手册见 `rules/19-CAPACITY-SLO-EXECUTION-HANDBOOK.md`。

### 4.5 Track E：Global / I18n / Compliance 落地

目标：把 `rules/13-GLOBAL-I18N-AND-COMPLIANCE.md` 从策略说明推进到产品和工程落地。

必须完成：

- 清理 App/Web/BFF 的硬编码用户可见文案
- 建立中英文资源文件、术语表和文案发布流程
- 为用户语言、地区、时区、内容语言、通知语言建立独立字段和落库路径
- 建立用户查看/导出/纠正/删除关键画像与记忆事实的真实接口
- 明确区域路由、数据驻留、跨境说明和降级策略

验收标准：

- App/Web 可切换中文和英文
- 高风险拒绝、举报、隐私、申诉文案均经过审核
- 用户数据权利接口可演示
- 区域与数据驻留策略进入工程说明和部署文档

执行级手册见 `rules/20-GLOBAL-I18N-COMPLIANCE-EXECUTION-HANDBOOK.md`。

## 5. 开发顺序

后续执行顺序固定为：

1. `Track A` 控制面收口
2. `Track B` App + Web 双端补齐
3. `Track C` Phase 2B 闭环收口
4. `Track D` Capacity / SLO 落地
5. `Track E` Global / I18n / Compliance 落地

不建议把 `Track D/E` 提前到 `Track B/C` 之前做成孤立大工程，因为它们依赖真实 App/Web/闭环链路。

## 6. 里程碑

### Milestone 1：控制面与规则基线收口

范围：

- `Track A`
- 文档只做必要回写，不新扩散主题

出口条件：

- 控制面相关完整回归全绿
- 完成态与恢复态字段一致
- 新修复有独立报告和运行证据

### Milestone 2：App MVP 与 Web 对齐

范围：

- `Track B`
- `Track C` 的前置接口和 BFF 接线

出口条件：

- App 工程上线到仓库
- App/Web 共享契约、共享埋点、共享设计 token
- App 可完成核心主旅程

### Milestone 3：连接闭环

范围：

- `Track C`

出口条件：

- 匹配、推荐、私信、安全、反馈、后台串成最小闭环
- 前端页面不再保留“后续版本上线”占位描述

### Milestone 4：规模与全球化门禁

范围：

- `Track D`
- `Track E`

出口条件：

- 标准化可观测面、压测门禁、故障注入可执行
- 中英文资源化完成
- 合规接口和区域策略可验证

## 7. 角色与 owner

| Track | 主 owner | 协作 owner | 主要产物 |
|------|-----------|-------------|----------|
| A 控制面收口 | 平台 / 自动化 | QA | Python runtime 代码、测试、完成态报告 |
| B App + Web | 移动端负责人 | Web、BFF、设计 | App 工程、双端契约、埋点、设计系统、演示链路 |
| C 闭环收口 | Rust 后端 | App/Web、运营后台 | match/dm/safety/BFF/后台真实闭环 |
| D Capacity / SLO | 平台 / Rust 后端 | QA、数据 | ready/metrics、压测脚本、故障注入、SLO 报告 |
| E Global / Compliance | 产品 + 后端 | App/Web、法务/运营 | i18n 资源、合规接口、区域策略、审核文案 |

## 8. 文档回写要求

每完成一个 Track，必须同步回写：

- `rules/08-ROADMAP.md`：更新阶段完成度
- `rules/09-NEXT-DEVELOPMENT-PLAN.md`：更新执行优先级和批次状态
- 对应工程 README、OpenAPI、event schema、脚本 README
- 对应 work report、audit report、closeout 报告

不允许再出现以下情况：

- 代码已完成，但规则或 README 仍写“后续版本上线”
- work report 声称已完成，但 audit 或原始证据不支持
- 运行态已变更，但 state/recovery/heartbeat 未同步

## 9. 本文件与其他规则的关系

- App 详细开发计划见 `rules/15-APP-DELIVERY-PLAN.md`
- 路线图主轴仍以 `rules/08-ROADMAP.md` 为准
- 当前执行顺序仍以 `rules/09-NEXT-DEVELOPMENT-PLAN.md` 为准
- 容量与发布门禁仍以 `rules/12-CAPACITY-AND-SLO.md` 为准
- 全球化与合规门禁仍以 `rules/13-GLOBAL-I18N-AND-COMPLIANCE.md` 为准
