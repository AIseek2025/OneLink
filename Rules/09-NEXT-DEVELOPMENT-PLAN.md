# Next Development Plan

## 1. 总判断

OneLink 下一步开发必须从“强规划型工程”转向“可持续交付型工程”。优先把已存在的核心纵切面变成安全、持久、可测试、可部署的产品基座，再推进 App/Web 双端和匹配闭环。

基于 iteration 1-60 的最新审计，剩余工作总收口方案见 `rules/14-DELIVERY-CLOSURE-PLAN.md`，App 专项开发方案见 `rules/15-APP-DELIVERY-PLAN.md`。

## 1.1 Phase / P 对照

本文使用 `P` 表示当前执行批次，和 `rules/08-ROADMAP.md` 的 `Phase` 对应如下：

| 本文批次 | Roadmap 阶段 | 说明 |
|----------|--------------|------|
| P0-docs | Phase 0 | 真相源收敛、目录迁移、入口联动、Git 收口 |
| P0-engineering | Phase 1 | 安全、持久化、CI、纵切面生产化 |
| P1 | Phase 2A | App + Web 基础壳与聊天/画像链路 |
| P2 | Phase 2B | 匹配、私信、安全闭环接入 |
| P3 | Phase 3 | AI 记忆与上下文增强 |
| P4 | Phase 4 | 高并发、成本、容量与规模化准备 |
| P5 | Phase 5 | 全球化、多语言、合规与多区域 |
| P6 | Phase 6 | 桌面客户端与深度 Agent |

## 2. 立即执行的 P0 工作

### P0-1：安全基线

- 非开发环境禁止默认 `INTERNAL_SHARED_SECRET`。
- 为内部接口增加启动期配置校验。
- identity-service 引入服务端密码哈希。
- 会话持久化、撤销、过期和多实例共享。
- 内部观测接口加网络与鉴权保护。

### P0-2：持久化基线

- identity 用户/会话持久化。
- ai-chat conversation/message 持久化。
- question delivery/answer 持久化。
- context memory/checkpoint/context_logs 在 DB 模式下成为共享环境默认。
- 明确本地内存 fallback 只用于 dev 和 smoke。

### P0-3：自动化质量基线

- 建立 CI：fmt、clippy、test、schema 校验。
- 把 chat -> memory -> profile smoke 纳入可重复脚本。
- 补最小 contract test，覆盖 BFF、ai-chat、context、profile。
- 为内部鉴权和默认密钥风险加测试。

### P0-4：文档入口更新（已完成）

- 顶层 README 指向 `rules/`。
- `docs/AGENT_MEMORY_BRIEF.md` 指向 `rules/`。
- `repo/README.md` 纠正旧 `Rules-V2` canonical 入口。

状态：已在 2026-05-15 至 2026-05-16 的规则收口批次完成，后续只需保持同步维护。

### P0-5：Git 层目录收口

- 按 `rules/11-GIT-SETTLEMENT-CHECKLIST.md` 执行三类提交：目录迁移、规则内容、入口联动。
- 处理旧 `Rules/`、`Rules-V2/` 显示为删除、新 `rules/` 与 archive 显示为未跟踪的问题。
- 清理 `.DS_Store` 追踪噪音，确认 `.gitignore` 已覆盖 `.DS_Store`。
- 不把无关业务代码、历史研究资料改动和规则迁移混在同一个提交里。

## 3. P1：App + Web 基础壳开工

### 3.1 共用准备

- 冻结 BFF 面向 App/Web 的 OpenAPI。
- 建立设计系统 token、组件命名、空状态、错误状态。
- 建立埋点事件：注册、聊天、画像确认、找人、推荐曝光、私信、举报。
- 建立 API mock 与前端本地开发环境。

### 3.2 App 优先页面

- 启动 / 登录 / 注册。
- Lumi 聊天。
- 基础画像确认卡。
- 找人意图输入壳。
- 推荐占位与反馈入口。
- 设置 / 个人基础页。

### 3.3 Web 优先页面

- 登录注册。
- Lumi 聊天宽屏版。
- 完整资料与偏好编辑。
- 问卷与画像纠错。
- 找人请求配置。
- 推荐占位与解释壳。
- 管理后台骨架。

P1 的目标是让 App/Web 双端与 BFF 契约、设计系统、埋点、聊天与画像链路先稳定下来，不提前承诺完整匹配、私信与安全闭环。

如果仓库中仍缺 App 工程，或 App 仅停留在 README / 规划层，则 P1 不得视为完成，必须按 `rules/15-APP-DELIVERY-PLAN.md` 补齐。

## 4. P2：匹配与安全闭环

- match-service 从 placeholder 转正。
- safety-service 从 placeholder 转正。
- dm-service 从 placeholder 转正。
- 实现 graph_first + rule_filter + optional_vector 的候选路径。
- 实现陌生人首条私信安全审查。
- 推荐反馈入事件骨干并反哺画像和策略。

P2 完成后，App/Web 中的推荐名片、真实私信、举报 / 拉黑 / 风控结果才从占位能力升级为真实用户功能。

若 `Find`、`Recommend`、`Admin`、`DM`、`Safety` 中任一主链路仍是占位或半成品，则继续按 `rules/14-DELIVERY-CLOSURE-PLAN.md` 的 `Track C` 收口。

## 5. P3：AI 记忆与上下文增强

- 完善 `memory_artifacts`、`memory_summaries`、`memory_entities`、`memory_entity_links`。
- 增加 query-aware context build。
- 引入可关闭的 LLMLingua 类压缩评测。
- 设计 `.onelink/` 逻辑个人记忆容器。
- Skill 先做轨迹日志和人工审核发布，不直接自动上线。

## 6. P4：高并发与成本控制

- Prompt Gateway PoC：规则过滤、格式校验、重复请求缓存。
- model-gateway 能力舱壁：chat、match、safety 分预算和熔断。
- Redis 缓存策略：session、context summary、推荐结果短缓存。
- 压测核心链路：注册、聊天、context build、推荐。
- 建立成本指标：每会话 token、每成功连接成本、缓存命中率。

P4 的容量、SLO、压测和成本门禁以 `rules/12-CAPACITY-AND-SLO.md` 为准。

仅有 contract、budget、capacity status 或 cost metrics 骨架，不代表 P4 完成；必须具备标准化观测面、压测脚本、故障注入和回写报告。

## 7. 团队建议

最小推进配置：

- Rust 后端 2-3 人。
- App/Web 前端 2 人。
- 产品/设计 1-2 人。
- AI/数据 1 人。
- QA/平台 1 人。

短期不要把资源分散到桌面客户端、完整自研模型、大规模 AutoResearch 在线优化或复杂多区域部署。

## 8. 岗位 owner 与首月交付清单

| 角色 | 主要 owner | 首月可交付 |
|------|------------|------------|
| Rust 后端 | identity、ai-chat、context、profile、BFF、contract test | P0 安全/持久化改造、核心纵切面 smoke、OpenAPI/internal contract 对齐 |
| App/Web 前端 | App/Web 基础壳、设计系统、API mock、埋点 | Phase 2A 注册/登录/Lumi 聊天/画像确认 demo，框架选型建议与风险表 |
| 产品/设计 | Lumi 用户旅程、画像确认、找人意图、空状态与错误状态 | Phase 2A 页面流、设计 token、文案边界、i18n 术语表初版 |
| AI/数据 | memory、context build、评测集、成本指标 | query-aware context 评测、压缩开关实验、token/cost 指标定义 |
| QA/平台 | CI、smoke、contract test、环境模板 | fmt/clippy/test/schema/smoke CI 基线、环境变量校验、发布回滚清单 |

## 9. 近期验收清单

- `cargo fmt/clippy/test` 在 CI 中可运行。
- 本地一键启动核心纵切面。
- 默认开发密钥不能进入非 dev。
- 用户注册、登录、聊天、记忆抽取、画像读取可重复验证。
- App/Web 至少一个端到端 demo 跑通注册到聊天。
- 新团队成员只读 `README.md`、`docs/AGENT_MEMORY_BRIEF.md`、`rules/README.md` 就能知道从哪里开始。
