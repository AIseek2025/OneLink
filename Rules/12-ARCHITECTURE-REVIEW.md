# OneLink Architecture Review — Service Boundaries & Data/Event Model

> **状态：本报告为首轮审查记录。其中多条建议已在后续迭代中被采纳或被更高级别决策覆盖，最终口径以 `10-SERVICE-BOUNDARIES.md` 和 `11-DATA-EVENT-MODEL.md` 为准。**

> 审查对象：`10-SERVICE-BOUNDARIES.md`、`11-DATA-EVENT-MODEL.md`
> 审查人：Opus 4.6
> 日期：2026-03-20
> 上下文：同时参照 `00` ~ `09` 全套规划文档

---

## 总体评价

两份文档的底层思路是对的：先少后多、一写权一主、读可跨服务写必须走拥有者。在 MVP 阶段不追求"完美微服务拆分"也是正确的。但在多个具体细节上存在 **缺口、隐性耦合和未来瓶颈**，如果不在 SQL/OpenAPI 阶段解决，后面的工程实现会踩到真实的坑。

以下逐项列出，每条都给出 **问题 → 影响 → 建议**。

---

## 一、服务拆分：太早 / 不够 / 不清晰

### 1.1 `bff` 在 MVP 阶段是多余的一跳

**问题**

MVP 只有 Web 一个客户端，后端只有 6 个核心服务。BFF 在这个阶段只是一个纯透传的聚合层，每个请求多经过一次序列化/反序列化和网络往返。

**影响**

- 增加 P99 延迟 5-15ms
- 增加一个需要运维、监控、部署的服务
- 聚合逻辑在 MVP 阶段极少（推荐名片的渲染已经在 match-service 出口组装好了）

**建议**

MVP 阶段把聚合逻辑做成 `api-gateway` 内部的一个 Go 模块或中间件。当出现第二个客户端（Admin、移动端）且数据拼装逻辑明显不同时再拆出独立 BFF。在 `10-SERVICE-BOUNDARIES.md` 中应标记 BFF 为"Phase 2 候选"而非 MVP 必须。

---

### 1.2 `chat-service` 承载了两个不同的领域

**问题**

`chat-service` 同时拥有：

| 子领域 | 特征 |
|--------|------|
| AI 对话（ai_conversations / ai_messages） | 用户 ↔ 平台 AI，同步为主，触发画像抽取，上下文窗口管理 |
| 私信（dm_threads / dm_messages） | 用户 ↔ 用户，异步为主，需要审核、送达状态、陌生人限制 |

两者的写入量级、一致性需求、审核链路和扩展路径完全不同。

**影响**

- AI 对话消息量在前期远超私信量，而 DM 在规模化时可能反超
- AI 对话的上下文窗口管理（摘要、token 预算）是 AI 平台问题；DM 的送达、已读、离线同步是 IM 工程问题
- 放在同一服务里，未来拆分时写路径、表结构、消息队列都会纠缠在一起

**建议**

MVP 可以暂时放在同一个服务进程里，但 **必须在代码层面做严格的包/模块隔离**（`chat-service/ai/` 和 `chat-service/dm/`），共享层只限于鉴权和连接管理。在 Phase 2 开始前完成拆分。在 `10-SERVICE-BOUNDARIES.md` 中应明确标注此拆分路径。

---

### 1.3 关注（Follow）没有数据拥有者

**问题**

`01-PRODUCT-SYSTEM.md` 定义了单向关注系统，`04-MATCHING-SAFETY-GOVERNANCE.md` 把关注作为推荐反馈信号。但：

- `10-SERVICE-BOUNDARIES.md` 没有任何服务声明拥有 follow 数据
- `11-DATA-EVENT-MODEL.md` 没有 `follows` 表
- 事件清单中没有 `social.follow.created.v1` 或 `social.follow.removed.v1`

**影响**

- 关注是社交图谱的基础，它同时影响：推荐信号、通知触发、主页展示、信任分计算
- 如果关注被临时塞进 `profile-service`，会引入和 match-service 的隐性耦合
- 如果关注被塞进 `match-service`，会让 match-service 变成混合了"社交关系"和"推荐"的双责服务

**建议**

在 MVP 阶段将 follow 数据归入 `profile-service`（因为 follow 本质上是用户之间的社交关系，和 profile/discovery 最近）。在数据模型中补充：

```
follows
├── id
├── follower_user_id
├── followee_user_id
├── source (recommendation / search / profile_visit)
├── created_at
└── unfollowed_at (nullable)
```

在事件模型中补充：`social.follow.created.v1`、`social.follow.removed.v1`。

---

### 1.4 画像事实管线（Fact Pipeline）在 MVP 中无主

**问题**

`10-SERVICE-BOUNDARIES.md` 第 12.1 节说"画像更新通过异步事实管线进入 `profile-service` 或特征层"。但：

- `profile-service` 的数据拥有范围只列了 profiles / visibility / discovery_preferences
- `profile_facts`、`profile_traits`、`profile_embeddings`、`profile_summaries` 在 MVP 表清单中，但没有服务声称拥有写权限
- `feature-service` 被标记为"不建议 MVP 过早拆出来"

**影响**

这是整个平台最核心的 AI 闭环（聊天 → 抽取 → 事实 → 特征 → 向量 → 推荐），但写路径没有明确主人。

**建议**

明确：在 MVP 阶段，`profile-service` 拥有所有 `profile_*` 表的写权限。事实抽取作为 profile-service 的异步消费者（监听 `chat.user_message.created.v1` 事件，调用 model-gateway 做抽取，然后写入 profile_facts）。在文档中显式写明这条完整路径，而不是用"或特征层"做模糊处理。

---

### 1.5 `model-gateway` 用 Go 的决策应被重新审视

**问题**

`model-gateway` 处于所有 AI 调用的关键路径上。chat-service、match-service、safety-service 都依赖它。在规模化阶段，它的 QPS 等于所有 AI 请求的总和。

**影响**

- Go 的 GC pause 在高并发长连接场景（等待外部模型 API 响应，典型 1-10 秒）下会消耗大量 goroutine 和内存
- 而 match-service 和 safety-service 已经选了 Rust，意味着团队有 Rust 能力
- model-gateway 的逻辑（路由、缓存、审计、计量）并不复杂，但对延迟和吞吐的要求极高

**建议**

MVP 用 Go 快速启动可以接受，但 `10-SERVICE-BOUNDARIES.md` 应标注 model-gateway 为"Phase 2/3 Rust 迁移候选"。同时在 MVP 阶段就要做好 per-capability 隔离（chat 调用和 safety 调用走不同的连接池和限流桶），避免一个慢模型拖垮全局。

---

### 1.6 Question 域在 MVP 中的归属缺失

**问题**

- `question-service` 被规划为 Phase 2 服务
- 但 `03-AI-PROFILE-QUESTIONNAIRE.md` 定义了"聊天中自然穿插问题"作为 MVP 能力
- `11-DATA-EVENT-MODEL.md` 在 MVP 表清单中没有 question 相关表，但画像事实的来源之一就是 questionnaire

**影响**

如果 MVP 真的要在聊天中穿插问题，必须有人管问题模板和答案记录。

**建议**

两条路选一条并在文档中明确：

- **路线 A**：MVP 的问题只是 AI 在聊天中的自然追问，不走独立题库，不记录独立的 `question_answers` 表。用户的回答直接作为聊天消息进入事实抽取。
- **路线 B**：MVP 就引入轻量题库（question_templates + question_answers），由 chat-service 内部模块管理，Phase 2 再拆成独立服务。

我的建议是 **路线 A**，因为它和"1 万问题系统"的演进路径不冲突，且避免了 MVP 阶段的额外复杂度。

---

## 二、写路径污染与数据拥有权冲突

### 2.1 `profile_facts` 的写入来源至少有 4 个

| 写入来源 | 触发方式 |
|----------|----------|
| AI 聊天事实抽取 | chat.user_message.created → 模型抽取 → 写入 |
| 问卷回答 | question.answered → 事实映射 → 写入 |
| 用户手动编辑 | profile-service API → 直接写入 |
| 矛盾确认 | 用户确认 pending_confirmation 事实 → 状态流转 |

如果这 4 条路径不统一走 `profile-service` 的写入 API，就会出现：
- 并发写入冲突（两条管线同时更新同一 fact_key）
- 置信度覆盖（低置信来源覆盖高置信来源）
- 审计链断裂（部分写入不经过统一日志）

**建议**

所有 `profile_facts` 写入必须走 `profile-service` 的内部 `UpsertFact` 接口。该接口必须：
1. 做 `fact_key + user_id` 级别的乐观锁
2. 有统一的矛盾检测逻辑
3. 每次写入都发出 `profile.fact.upserted.v1` 事件

---

### 2.2 `recommendation_feedbacks` 的写入分散在多个服务

**问题**

`recommendation_feedbacks` 记录的行为发生在不同服务的上下文中：

| feedback_type | 实际触发位置 |
|---------------|------------|
| impression | 前端上报 → api-gateway |
| click | 前端上报 → api-gateway |
| follow | 前端 → ??? (follow 无主) |
| dm_start | 前端 → chat-service |
| dm_reply | chat-service |
| dismiss | 前端上报 → api-gateway |
| block | 前端 → safety-service? |
| report | 前端 → safety-service |

**影响**

如果每个服务自己写 `recommendation_feedbacks`，就违反了"一类主数据只允许一个拥有者写"的原则。

**建议**

反馈写入应该统一为**事件驱动**：各服务发出各自领域的事件（`match.card.clicked.v1`、`social.follow.created.v1`、`chat.dm_message.created.v1`），由 match-service 的异步消费者监听这些事件并写入 `recommendation_feedbacks`。这样 match-service 保持唯一写权限，同时不需要其他服务知道 recommendation_feedbacks 的存在。

---

### 2.3 `risk_assessments` 的触发源过于分散

`risk_assessments` 的 `target_type` 包括 find_request、dm_message、profile。这意味着 safety-service 需要在三个不同的业务流中被调用。

**建议**

在服务边界文档中明确 safety-service 的调用模式：

- **同步调用**（阻塞性）：find_request 风险评估、陌生人首条 DM 审核
- **异步调用**（非阻塞）：profile 变更后的批量回溯审查、DM 已发消息的回扫

这两种模式对 safety-service 的接口设计、超时设置和降级策略有根本性差异。

---

## 三、表和事件的扩展瓶颈

### 3.1 `ai_messages` — 增长最快、最容易成为热点的表

**量级估算**

以 1M DAU、平均每日 10 条消息计算：
- 日增 1000 万行
- 月增 3 亿行
- 年增 36 亿行

字段 `content_text` 存储了用户消息和 AI 回复原文，单行可达数 KB。

**瓶颈**

- 无分区策略 → 单表 36 亿行，索引膨胀，vacuum 缓慢
- 冷热数据混合 → 99% 查询只访问最近 7 天的消息，但全量数据在同一个表
- `content_text` 和元数据混存 → 全表扫描和索引维护被 text 字段拖慢

**建议**

1. **必须在 SQL schema 阶段就定义分区策略**：按 `created_at` 月分区
2. **内容分离**：将 `content_text` 和 `content_metadata` 拆到独立的内容表（`ai_message_contents`），主表只保留元数据和外键
3. **定义冷数据归档策略**：超过 N 天的消息 → 冷存 → 删除热表行
4. **主键策略**：使用 ULID 或时间有序 UUID，避免随机 UUID 造成索引随机写

---

### 3.2 `recommendation_feedbacks` — 这不应该是 OLTP 表

**量级估算**

以 100K 日活找人、每次返回 10 张名片、每张名片 1 条 impression 计算：
- 日增 100 万 impression
- 加上 click/follow/dm_start 等交互，日增约 120-150 万行

到 10M DAU：日增 1.2-1.5 亿行。

**瓶颈**

- 这是一个追加写入、很少更新、主要供离线分析和模型训练使用的数据
- 放在 PostgreSQL 的 OLTP 表中会浪费写入带宽和存储
- `card_id` 的外键约束在高写入时会成为锁竞争热点

**建议**

1. MVP 阶段可以先用 PostgreSQL 表，但不加外键约束（只加索引）
2. 在 Phase 2 迁移到事件流（Kafka → 数据仓库），PostgreSQL 只保留最近 N 天的热数据供在线查询
3. 在 SQL schema 中标注此表为"未来迁移到事件存储"的候选

---

### 3.3 `profile_facts` + `profile_fact_revision` — 无上限增长

**问题**

- 每次 fact 更新都产生 revision
- `superseded` 状态的旧 fact 不会被删除
- `profile_traits.supporting_fact_ids` 是数组引用，如果用 JSON 数组存储，无法做外键约束；如果用关联表，会产生另一个高写入表

**建议**

1. 定义 fact 保留策略：超过 N 个版本的 superseded facts 归档
2. `supporting_fact_ids` 改为独立关联表 `trait_supporting_facts(trait_id, fact_id)`
3. 在 SQL schema 阶段就预设 `profile_facts` 的分区（按 user_id hash 分区或按时间分区）

---

### 3.4 `dm_messages` — 缺少参与者表导致查询瓶颈

**问题**

数据模型第 3.3 节列出了 `dm_participant` 实体，但核心表草案中没有对应表。

**影响**

- 查询"用户 X 的所有会话"需要扫描 `dm_threads` 全表或 `dm_messages` 全表
- 没有 participant 表意味着无法记录每个参与者的：已读位置、静音状态、最后可见消息时间
- 未来如果支持群聊，缺少这个表会导致重大重构

**建议**

补充 `dm_participants` 表：

```
dm_participants
├── id
├── thread_id
├── user_id
├── role (initiator / recipient)
├── status (active / muted / left)
├── last_read_message_id
├── joined_at
└── updated_at
```

同时以 `(user_id, updated_at DESC)` 为索引，支持高效的"我的会话列表"查询。

---

### 3.5 `find_requests` — `parsed_constraints_json` 是反模式

**问题**

`parsed_constraints_json` 用 JSON 存储结构化约束（时区、语言、地区等）。

**影响**

- 无法对约束字段建索引
- 无法做跨请求的统计分析（"有多少请求限定了中文？"需要全表 JSON 扫描）
- 不同模型版本的 JSON schema 不一致时，下游消费者会崩溃

**建议**

将高频使用的约束字段（语言、地区、时区、是否跨语言）提升为独立列。保留 `extra_constraints_json` 作为扩展字段但只存低频、非索引字段。

---

### 3.6 事件模型缺少 `discovery.preference.updated.v1`

**问题**

当用户修改被找偏好（关闭被找、修改接受的语言/地区/频率）时，匹配管线需要立即知道。

**影响**

如果没有这个事件，match-service 的召回层可能会继续把已经关闭被找的用户推荐给其他人，直到下一次全量索引重建。

**建议**

在事件清单的"资料域"中补充：
- `profile.discovery_preference.updated.v1`
- `profile.visibility_rule.updated.v1`

match-service 监听这些事件，实时更新召回索引中的可见性标记。

---

## 四、隐性耦合与一致性风险

### 4.1 聊天 → 画像管线是一个伪装的分布式事务

**完整路径**

```
用户发消息
  → chat-service 记录 ai_message
  → 发出 chat.user_message.created.v1
  → fact-extraction 消费者（profile-service 内）调用 model-gateway 抽取事实
  → 写入 profile_facts
  → 发出 profile.fact.upserted.v1
  → trait-builder 消费者更新 profile_traits
  → 发出 profile.trait.updated（目前缺失）
  → embedding-updater 重算 profile_embeddings
  → 发出 profile.embedding.updated.v1
  → summary-builder 更新 profile_summaries
  → 发出 profile.summary.updated.v1
```

**风险**

这是一条 6 步异步管线。任何一步失败都会导致画像不一致：

| 失败点 | 结果 |
|--------|------|
| 抽取失败 | 消息有了但画像没更新 — 可接受 |
| fact 写入成功但 trait 更新失败 | 事实和特征不一致 — 推荐质量下降 |
| trait 更新成功但 embedding 失败 | 特征是新的但向量是旧的 — 召回结果错误 |
| embedding 成功但 summary 失败 | 向量新但摘要旧 — 名片展示过时 |

**建议**

1. 每步都必须有重试机制和死信队列
2. 定义一个定时的 **一致性修复 job**：扫描 profile_facts 最新时间 vs profile_embeddings 最新时间，如果差距超过阈值则强制重算
3. 在 `profile_summaries` 和 `profile_embeddings` 上加 `source_fact_version`（最后处理的 fact id），用于判断是否需要重算
4. 事件模型补充 `profile.trait.updated.v1`（目前缺失）

---

### 4.2 `trust_score` 在 MVP 中没有落地位置

**问题**

`04-MATCHING-SAFETY-GOVERNANCE.md` 的精排层已经把 `trust_score` 作为排序因子。但：
- `trust-service` 被规划在 Phase 3
- `trust_score_snapshot` 表在数据模型中存在但没有进入 MVP 表清单
- 没有任何服务在 MVP 中负责计算和维护 trust_score

**建议**

两种方案选一种：

- **方案 A（推荐）**：MVP 阶段把 trust_score 简化为一个 `profile-service` 中的计算字段，基于 risk_assessments 和 moderation_actions 的简单统计（历史违规次数 × 权重）。不做独立服务，不做复杂模型。
- **方案 B**：MVP 精排暂时不使用 trust_score，用 safety-service 的最近一次 risk_level 替代。

无论选哪种，都需要在 `10-SERVICE-BOUNDARIES.md` 中明确说明。

---

### 4.3 `model-gateway` 是全局单点故障

**问题**

chat-service、match-service、safety-service 都同步依赖 model-gateway。model-gateway 又依赖外部模型 API。

**影响**

- 外部模型 API 出现一次大面积延迟（常见），model-gateway 的连接池被耗尽
- 所有下游服务同时超时
- 用户看到的是：聊天卡死、找人卡死、私信审核卡死

**建议**

1. model-gateway 内部必须按能力做**舱壁隔离**（bulkhead）：chat 调用、safety 调用、match 调用走独立的连接池和线程/goroutine 配额
2. 每种能力必须有独立的**熔断器**（circuit breaker）
3. safety-service 必须有**规则兜底模式**：当 model-gateway 不可用时，纯规则引擎接管，宁可误拦也不放行
4. chat-service 必须有**降级回复**：当模型不可用时，返回预设的延迟回复（"我正在思考中，稍等一会儿回复你"），而不是直接报错
5. 以上四点应写入 `10-SERVICE-BOUNDARIES.md` 的 model-gateway 职责部分

---

### 4.4 事件模型缺少关键的一致性保障定义

**问题**

`11-DATA-EVENT-MODEL.md` 定义了事件命名和字段标准，但没有定义：
- 事件投递语义（at-least-once / exactly-once）
- 分区策略（按 user_id？按 entity_id？）
- 幂等性要求（消费者如何处理重复事件？）
- 事件顺序保证（同一用户的事件是否保证有序？）

**影响**

- 如果 `profile.fact.upserted.v1` 乱序到达（先到新 fact，后到旧 fact），消费者可能用旧数据覆盖新数据
- 如果重复投递且消费者没有幂等处理，同一个 fact 可能被写入两次

**建议**

在事件模型中增加"事件投递规范"章节：

1. 投递保证：at-least-once（Kafka 默认）
2. 分区键：所有涉及用户的事件以 `actor_user_id` 或 `subject_user_id` 为分区键，保证同一用户的事件在同一分区内有序
3. 幂等键：每个事件的 `event_id` 必须全局唯一，消费者必须维护已处理 event_id 的窗口去重
4. 事件时间 vs 处理时间：以 `occurred_at` 为业务时间，消费者遇到乱序时以 `occurred_at` 更晚者为准

---

### 4.5 match-service → safety-service 存在潜在的循环依赖风险

**当前调用关系**

```
match-service → safety-service  （找人请求风险评估）
safety-service → model-gateway  （模型辅助审查）
```

**未来风险**

当 trust-service 在 Phase 3 引入后：
```
match-service → trust-service  （获取信任分）
trust-service → safety-service （获取风险历史）
safety-service → trust-service （获取信任分辅助判定）  ← 循环！
```

**建议**

在 `10-SERVICE-BOUNDARIES.md` 中为 trust-service 预先定义单向依赖规则：
- trust-service 只读取 safety-service 产生的**历史数据**（通过事件消费，不做同步调用）
- safety-service 读取 trust_score 时只读**缓存快照**（不同步调用 trust-service）
- 用事件驱动替代同步调用来打破潜在循环

---

## 五、数据模型补充建议

### 5.1 缺失的表

| 表名 | 归属服务 | 用途 |
|------|---------|------|
| `follows` | profile-service | 关注关系 |
| `dm_participants` | chat-service | 会话参与者、已读状态 |
| `user_blocks` | safety-service | 用户间拉黑关系 |
| `model_invocation_logs` | model-gateway | 模型调用日志（在边界文档中提到但无表定义） |

### 5.2 需要改动的字段

| 表 | 字段 | 建议 |
|----|------|------|
| `find_requests` | `parsed_constraints_json` | 高频约束提升为独立列 |
| `profile_traits` | `supporting_fact_ids` | 改为独立关联表 |
| `ai_messages` | `content_text` + `content_metadata` | 拆到独立内容表 |
| `profile_embeddings` | 新增 `source_fact_version` | 用于一致性校验 |
| `profile_summaries` | 新增 `source_fact_version` | 用于一致性校验 |

### 5.3 需要补充的事件

| 事件 | 用途 |
|------|------|
| `social.follow.created.v1` | 关注关系变更 → 推荐信号 + 通知 |
| `social.follow.removed.v1` | 取关 → 推荐信号更新 |
| `profile.discovery_preference.updated.v1` | 被找偏好变更 → 召回索引更新 |
| `profile.visibility_rule.updated.v1` | 可见性变更 → 名片展示更新 |
| `profile.trait.updated.v1` | 特征更新 → 触发 embedding 重算 |
| `safety.user_block.created.v1` | 拉黑 → 召回过滤 + 通知 |
| `model.invocation.completed.v1` | 模型调用完成 → 成本计量 + 审计 |

---

## 六、对三步联合开发安排的评价

### 第一步：Opus 4.6 审查 → 合适 ✓

这份审查报告就是第一步的产出。审查定位在"找问题、定边界"，不涉及具体代码，适合我的角色。

### 第二步：Composer 1.5 细化 SQL / OpenAPI / 事件 Schema → 合适 ✓

Composer 1.5 擅长基于明确规范产出大量结构化草案。但有一个前提：

> **必须先将本审查报告中的修改建议合并到 `10-SERVICE-BOUNDARIES.md` 和 `11-DATA-EVENT-MODEL.md` 中，再交给 Composer 1.5 执行。**

否则 Composer 1.5 会基于有缺口的文档产出 SQL，后面要大面积返工。

建议的执行顺序：
1. 你审阅本报告，确认接受哪些建议
2. 我（或你指定的角色）更新 `10-SERVICE-BOUNDARIES.md` 和 `11-DATA-EVENT-MODEL.md`
3. Composer 1.5 基于更新后的文档输出：
   - SQL schema 初稿（含分区、索引策略）
   - OpenAPI 接口草案（按服务分文件）
   - 事件 schema JSON 草案（含幂等键和分区键定义）
   - 服务间接口契约清单

### 第三步：Composer 2 Fast 铺目录和模板 → 合适 ✓

批量脚手架工作适合 Fast 模型。但建议在模板中预埋审查报告中提到的关键基础设施：
- 每个服务的 health check 端点模板
- 每个服务的 circuit breaker 配置模板
- 事件消费者的幂等处理模板
- 结构化日志和 trace 注入模板

### 总结：三步分工合理，但步骤间需要一个"修订合并"环节

```
第一步 Opus 审查
    ↓
 [修订合并] ← 你确认 + 更新文档
    ↓
第二步 Composer 1.5 细化
    ↓
 [快速复核] ← Opus 或你扫一眼 SQL 和 API 是否遵守边界
    ↓
第三步 Composer 2 Fast 铺骨架
```

---

## 七、对核心 AI 问题的建议

### 7.1 画像自我进化：关键在于闭环，不在于模型大小

当前规划的闭环路径是对的（聊天 → 抽取 → 事实 → 特征 → 推荐 → 反馈 → 权重调整）。我的补充建议：

- **画像完整度指标**必须在 MVP 就开始采集。定义每个用户的画像覆盖率（已填充维度 / 总维度），作为问题推送和匹配质量的基础信号。
- **事实衰减机制**：超过 N 个月未被用户确认或行为验证的事实，自动降低置信度。当前文档只说了"superseded"状态，没有时间衰减。
- **反馈信号的归因粒度**：当用户 dismiss 一张名片时，到底是画像错了、匹配逻辑错了、还是被推荐人不好？需要在 feedback 中记录 `dismiss_reason`（可选但有价值）。

### 7.2 1 万问题系统：MVP 别做题库，做"AI 自然追问"

1 万问题的工厂化生成系统是成长期能力。MVP 阶段应该聚焦于让 AI Companion 在对话中自然地补充追问。这和 03 文档的设想一致。关键改进：

- Prompt 中内嵌"当前画像缺口清单"，让模型自行决定何时追问
- 追问不要打断对话流，而是作为对话中的自然过渡
- 追问频率限制：每次会话最多 2 个画像补充问题

### 7.3 推荐匹配的自我进化：先做排序学习，再做端到端模型

MVP 用规则 + 向量召回 + 简单加权排序就足够。进化路径：

1. **Phase 1**：硬规则过滤 + embedding 召回 + 手工权重排序
2. **Phase 2**：引入 Learning to Rank（LTR），用 `recommendation_feedbacks` 训练 pointwise/pairwise 排序模型
3. **Phase 3**：引入多目标优化（同时优化 click、follow、dm_reply、report 等多个信号）
4. **Phase 4**：端到端 Deep Ranking 或 Reinforcement Learning

每一步的前提是：前一步产生了足够的标注数据。

### 7.4 审核处罚的自我进化：关键是负样本的积累

- MVP 的规则引擎 + 外部模型复核是合理的起步
- 真正的进化来自**生产中的真实 case**：被确认的举报、被改判的处罚、被漏判的事故
- 建议从 MVP 开始就记录每次审核决策的完整上下文（输入、规则命中、模型输出、最终决定、后续反馈），作为未来自研安全模型的训练数据
- Phase 2 引入"影子模式"：新模型在后台运行但不执行，和线上模型对比结果，验证准确率后再切换

### 7.5 Autoresearch 集成：当前的局部借鉴方案是对的

`06-AUTORESEARCH-PAPERCLIP-INTEGRATION.md` 把 autoresearch 定位为"离线研究闭环的实验执行器"是正确的判断。补充建议：

- 不要在 Phase 1-2 就投入精力构建完整的 research loop。先手动做几轮实验，建立评估基准后，再自动化。
- 自动化的第一优先级：安全模型的 regression bench（每次模型更新自动跑安全回归测试），这是最高 ROI 的自动实验。

### 7.6 外部模型 → 自研模型的节奏：关键是 Model Gateway 的能力抽象

当前规划的"能力接口 → 外部模型 → 领域小模型 → 中模型 → 组合"路径是对的。我的补充：

- **最先自研的应该是 Embedding 模型**：因为它是高频（每次画像更新都要调用）、低延迟要求、结构化输出、评估简单（recall@k）
- **第二自研的应该是安全分类器**：高频、结构化、负样本积累快、对延迟敏感
- **最晚自研的是对话模型**：对话质量极难评估，需要大量人工标注和 A/B 测试，且外部模型在持续快速进步

**关于架构不需推倒重来的保证**：当前 Model Gateway 的能力接口设计（`chat.respond`、`profile.extract_facts` 等）是正确的。只要业务层调用的永远是这些能力接口而不是具体的模型名称，底层换成自研模型只是 Gateway 内部的路由变更。这一点已经在 `05-MODEL-PLATFORM-ROADMAP.md` 中明确表述了。

---

## 八、总结：必须在进入第二步之前解决的 5 件事

| 优先级 | 事项 | 影响面 |
|--------|------|--------|
| P0 | 明确 `profile_facts` 及所有 `profile_*` 衍生表的写入归属（profile-service） | 整个 AI 闭环 |
| P0 | 补充 `follows` 表和 `social.follow.*` 事件 | 推荐、通知、信任分 |
| P0 | 补充 `dm_participants` 表 | 私信查询效率 |
| P1 | 定义 model-gateway 的舱壁隔离和降级策略 | 全站可用性 |
| P1 | 在事件模型中补充投递语义、分区键和幂等规范 | 数据一致性 |

以上 5 项确认后，`10-SERVICE-BOUNDARIES.md` 和 `11-DATA-EVENT-MODEL.md` 才算达到可以交付给 Composer 1.5 细化的标准。
