# Capacity And SLO

## 1. 目的

OneLink 的高并发目标不能只停留在“面向全球几十亿用户”的口号层。容量、SLO、成本和压测必须成为架构决策、Model Gateway 预算、缓存策略、数据分区和发布门禁的共同输入。

本文件定义阶段性目标，不代表 MVP 一开始就支撑全球流量；任何阶段升级都必须通过压测、观测和回滚门禁。

## 2. 阶段容量目标

| 阶段 | 用户规模目标 | 在线与请求目标 | 重点 |
|------|--------------|----------------|------|
| Phase 1 / P0-engineering | 内部测试与小规模邀请 | 单区域、低并发、可重复 smoke | 安全、持久化、CI、纵切面稳定 |
| Phase 2A / P1 | 早期内测用户 | App/Web 注册、聊天、画像确认链路稳定 | 前端壳、BFF 契约、埋点、API mock |
| Phase 2B / P2 | 小规模真实连接闭环 | 推荐、私信、安全审查具备最小闭环 | match/dm/safety 转正 |
| Phase 4 / P4 | 百万到千万级用户准备 | 核心链路可横向扩展，缓存和事件总线可压测 | 多实例、Redis/Kafka/Postgres/向量索引容量治理 |
| Phase 5 / P5 | 跨区域增长 | 多区域路由、数据驻留、跨语检索 | 区域合规、故障隔离、全球化 |

进入下一阶段前，必须把真实压测结果写回本文件或对应工程 README，不能只凭架构假设推进。

## 3. SLO 基线

MVP 前不承诺生产 SLA，但内部测试环境应建立以下 SLO 基线：

| 链路 | 初始目标 | 说明 |
|------|----------|------|
| 注册 / 登录 | p95 < 500ms（不含第三方验证） | identity-service 不得依赖进程内会话作为共享环境主状态 |
| Lumi 聊天首包 | p95 < 3s（不含外部模型极端波动） | 必须记录 model latency、context build latency、fallback reason |
| context build | p95 < 800ms（不含远端向量库冷启动） | 返回 selected ids、retrieval modes、degraded、token budget |
| 画像读取 | p95 < 300ms | profile-service 应能从持久化读取核心画像 |
| 推荐请求 | P2 初始 p95 < 1.5s | 先结构化召回和规则过滤，再语义补充 |
| 内部 smoke | 成功率 100% | 失败即阻塞共享环境发布 |

这些目标是起点，不是最终值。每次压测后应根据实际瓶颈更新。

## 4. 成本预算

成本指标必须和产品指标一起看：

- 每会话 token：按 user session、conversation、model provider 维度聚合。
- 每成功连接成本：推荐、解释、私信安全审查和后续反馈共同计入。
- 缓存命中率：Prompt Gateway、model-gateway、context summary、推荐结果短缓存分别记录。
- fallback rate：按 provider、能力域、错误类型统计。
- 无价值请求拦截率：Prompt Gateway 拦截、澄清、缓存命中必须可审计。

超过预算时优先降级：减少上下文、使用摘要、命中缓存、改用小模型、请求澄清；不得绕过 safety 或审计。

## 5. 压测门禁

进入 Phase 4 前必须具备：

- 注册、登录、聊天、context build、画像读取、推荐请求的压测脚本。
- 对 Postgres、Redis、Kafka 或兼容事件总线、向量索引的容量基线。
- 模型 provider 超时、限流、错误、fallback 的故障注入脚本。
- trace id 串联 API Gateway / BFF / service / model-gateway / event consumer。
- 压测报告包含 p50、p95、p99、错误率、资源利用率、成本指标和瓶颈判断。

## 6. 发布门禁

任何面向真实用户的共享环境发布必须满足：

- 配置不使用默认开发密钥。
- 核心状态不只存在进程内。
- health、ready、metrics 或等价观测面可用。
- 可回滚，且回滚步骤写入发布说明。
- 新增高成本模型路径时必须说明预算、fallback 和关闭开关。
