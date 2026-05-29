# Delivery Roadmap

## 阶段命名对照

`08-ROADMAP.md` 使用 `Phase` 作为对外路线图主轴；`09-NEXT-DEVELOPMENT-PLAN.md` 使用 `P` 作为当前执行批次别名。两者对应关系固定如下：

| Roadmap 阶段 | 执行别名 | 含义 |
|--------------|----------|------|
| Phase 0 | P0-docs | 真相源收敛、目录归档、入口更新、Git 收口 |
| Phase 1 | P0-engineering | 安全、持久化、CI、核心纵切面生产化 |
| Phase 2A | P1 | App + Web 基础壳、聊天、画像确认、问卷纠错 |
| Phase 2B | P2 | 匹配、私信、安全闭环接入 |
| Phase 3 | P3 | AI 记忆、上下文增强、数据飞轮 |
| Phase 4 | P4 | 高并发、容量、成本与规模化准备 |
| Phase 5 | P5 | 全球化、多语言、合规与多区域 |
| Phase 6 | P6 | 桌面客户端与深度 Agent |

## Phase 0：真相源收敛与工程基线

目标：让团队从同一套规则、同一条可运行链路、同一组质量门禁继续开发。

交付物：

- 新 `rules/` 目录。
- 旧 `Rules` / `Rules-V2` 归档。
- README 和 Agent 记忆入口更新。
- 当前纵切面 smoke 可重复运行。
- P0 风险拆成可执行任务。

## Phase 1：纵切面生产化

目标：把 `identity + bff + ai-chat + context + profile + question + model-gateway` 从开发态联调系统升级为测试态系统。

重点：

- 身份认证与会话生产化。
- 核心状态持久化。
- 内部服务鉴权强制化。
- context-service 可回放、可观测、可降级。
- 最小 CI、contract test、smoke。
- Web/App 基础入口同步开工。

验收：新环境可一键启动核心链路，自动化校验覆盖编译、核心 API 和 chat -> memory -> profile。

## Phase 2A：App + Web MVP 基础壳

目标：形成用户可用的登录、聊天、画像确认与前端开发基座，但不提前承诺完整匹配、私信、安全闭环。

范围：

- App：注册登录、Lumi 聊天、轻量画像确认、找人请求输入壳、推荐占位、反馈入口壳。
- Web：完整资料编辑、长表单、问卷与画像纠错、匹配配置、基础管理后台。
- BFF：统一双端 API。
- 设计系统、埋点协议、API mock 与本地开发环境。

验收：内部真实用户可以完成“注册 -> 登录 -> Lumi 聊天 -> 画像确认/纠错”的完整路径，且 App/Web 共享 BFF 契约与埋点规范。

剩余工作和收口策略见 `rules/14-DELIVERY-CLOSURE-PLAN.md`，App 专项方案见 `rules/15-APP-DELIVERY-PLAN.md`。

## Phase 2B：匹配、私信与安全闭环接入

目标：在 App/Web 基础壳之上，接入真实匹配、安全和私信能力，形成最小连接闭环。

重点：

- match-service 转正并接入真实候选路径。
- safety-service 转正并接入找人请求、陌生人首条私信风控。
- dm-service 转正并接入最小私信闭环。
- 推荐名片、解释、举报、拉黑、反馈从占位升级为真实功能。

验收：内部真实用户可以完成从注册到发起连接、收到推荐、发起私信、反馈结果的完整路径。

若仓库中仍存在推荐、管理后台、私信或安全能力的占位页、占位说明或仅骨架实现，不能视为本阶段完成，必须按 `rules/14-DELIVERY-CLOSURE-PLAN.md` 继续收口。

## Phase 3：匹配、安全与数据飞轮

目标：把匹配、反馈、问卷和安全形成闭环。

重点：

- 结构化召回 + 图筛 + 语义补充。
- 举报、处罚、申诉闭环。
- AutoResearch 只在 shadow/replay 中优化 Memory/Session/Retrieval policy。

## Phase 4：规模化准备

目标：支撑百万到千万级用户增长。

重点：

- 多实例无状态化。
- Redis/Kafka/Postgres/Qdrant 容量治理。
- Prompt Gateway 成本与安全防线。
- 灰度发布、回滚、统一观测。
- 自托管小模型承接高频压缩、分类、过滤、召回。

容量、SLO、压测和成本门禁见 `rules/12-CAPACITY-AND-SLO.md`。

若尚未形成标准化 `ready/metrics` 或等价观测面、压测脚本、故障注入和回写报告，不得宣称 Phase 4 已完成。

## Phase 5：全球化与多端扩展

目标：跨区域、多语言、合规化运营。

重点：

- 数据驻留与区域路由。
- 多语言 Lumi 风格一致性。
- 跨语检索与匹配。
- 桌面客户端立项。

全球化、多语言和区域合规门禁见 `rules/13-GLOBAL-I18N-AND-COMPLIANCE.md`。

若 App/Web 仍有硬编码用户可见文案、用户数据权利接口未落地、区域驻留仍停留在策略说明层，不得宣称 Phase 5 已完成。

## Phase 6：桌面客户端与深度 Agent

桌面客户端在正式上线并有一定用户量后开发。它面向长任务、常驻协作、浏览器自动化、职业/创作/研究等深度场景，不抢 MVP 资源。
