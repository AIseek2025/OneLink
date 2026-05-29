# OneLink V2 Memory Layer

## 1. 文档目标

定义 `OneLink V2` 的长期记忆层，包括认知网络、核心表、整合流水线、检索策略与投影边界。

---

## 2. 记忆层定位

`memory-layer` 不是一个“聊天历史仓库”，而是一个长期认知系统。

它的核心任务是：

1. 从交互中抽取值得记住的东西
2. 将碎片信息整合为可持续使用的认知单元
3. 在需要时把最相关的长期理解召回给会话层和业务层

---

## 3. 四类认知网络

V2 统一采用四类认知网络，`memory_artifacts.network_type` 冻结为：

### 3.1 `world`

更偏客观、可验证、稳定的外部事实。

例子：

- 用户在上海
- 用户说自己使用 Rust
- 用户所属行业是 AI 创业

### 3.2 `experience`

系统与用户之间发生过的经历、历史互动与时间线信息。

例子：

- 最近连续三次提到在找投资人
- 最近一周多次拒绝高频主动提问
- 上次匹配后未继续沟通

### 3.3 `opinion`

用户主观看法、偏好、立场、态度与可能变化的判断。

例子：

- 不喜欢被推销型聊天
- 对远程协作更开放
- 对婚恋导向社交排斥

### 3.4 `entity`

围绕人、组织、话题、地点、项目等形成的实体认知。

例子：

- 某位投资人
- 某创业项目
- 某城市
- 某技术主题

---

## 4. 核心数据对象

### 4.1 `memory_artifacts`

长期记忆的最小认知原子。

它不是画像事实，不是原始消息，也不是推荐结果。

它是：

> 可被检索、可被整合、可被投影、可被衰减的记忆单元。

### 4.2 `memory_summaries`

工作记忆的持久化摘要层。

作用：

- 稳定上下文组装
- 降低 token 波动
- 充当 working memory 与 persistent memory 之间的桥

### 4.3 `memory_entities`

实体主档案。

用于承接：

- 人
- 公司
- 组织
- 主题
- 城市
- 项目

### 4.4 `memory_entity_links`

实体之间的关系边。

支持：

- 有向关系
- 双向关系
- 基于证据的关系更新

---

## 5. 核心字段约束

### 5.1 `memory_artifacts`

必须至少包含：

- `id`
- `user_id`
- `network_type`
- `evidence_type`
- `content`
- `content_structured`
- `confidence`
- `importance_score`
- `consistency_score`
- `valid_from`
- `valid_until`
- `source_type`
- `source_service`
- `source_ref_id`
- `source_event_id`
- `entity_refs`
- `version`
- `superseded_by`
- `memory_level`
- `visibility`
- `vector_ref`
- `region`
- `expires_at`
- `created_at`
- `updated_at`

说明：

- `importance_score`、`consistency_score` 是落库后的内部评分字段
- 请求层或策略层可以使用统一的 `memory_value_score` 概念
- MVP 中由 `context-service` 把 `memory_value_score` 映射到 `importance_score` 等内部评分字段，不要求表字段同名

### 5.2 `memory_entity_links`

必须包含：

- `source_entity_id`
- `target_entity_id`
- `relation_type`
- `confidence`
- `evidence_artifact_id`
- `is_bidirectional`

### 5.3 `supersedes`

V2 文档预留 `supersedes` 为 Phase 2 扩展位；MVP 只强制实现 `superseded_by` 即可。

---

## 6. Memory Layer 与业务层边界

`memory-layer` 可以：

- 抽取长期记忆
- 标记冲突
- 建立候选事实
- 发起画像投影请求
- 提供推荐与安全的记忆特征输入

`memory-layer` 不可以：

- 直接写 `profile_facts`
- 直接写推荐结果
- 直接写处罚结论

---

## 7. Consolidation Pipeline

### 7.1 目标

`consolidation` 负责把重复、矛盾、零散的记忆候选整合成稳定的长期认知。

### 7.2 MVP 最小流水线

MVP 阶段至少实现：

1. `LLM / heuristic extraction`
2. `exact / near-exact dedup`
3. `conflict marking`
4. `summary update`
5. `projection request`

### 7.3 MVP 必须满足的工程要求

- 以 `event_id` 为处理主键
- 每一步幂等
- 支持失败重试
- 支持从事件日志重放

### 7.4 为什么必须可重放

因为长期记忆一旦写错，会污染：

- 后续上下文组装
- 用户画像
- 推荐
- 风险判断

没有重放能力，就无法系统修复错误历史。

---

## 8. 检索总架构

### 8.1 四路检索总图

```mermaid
flowchart LR
    query[Query] --> structured[StructuredRetrieval]
    query --> semantic[SemanticRetrieval]
    query --> graph[GraphExpansion]
    query --> temporal[TemporalFilter]
    structured --> fusion[FusionAndBudget]
    semantic --> fusion
    graph --> fusion
    temporal --> fusion
    fusion --> result[ContextBundle]
```

### 8.2 MVP 默认启用

- 结构化检索
- 语义检索
- 时间过滤

### 8.3 MVP 预埋但默认关闭

- 图扩展检索
- 完整 rerank

### 8.4 激活条件

只有当以下指标达到阈值时，图扩展检索才允许自动开闸：

- `memory_entities` 数据量足够
- `memory_entity_links` 关系密度足够
- 图路径命中对结果有正收益

---

## 9. MVP 检索降级策略

当以下任一情况发生：

- 向量索引超时
- Qdrant 不可用
- 图扩展数据不足

系统必须降级到：

1. `working memory`
2. 最近 `N` 条 `memory_summaries`
3. 最近时窗内的结构化高置信度 `memory_artifacts`

不得因为高级检索失败而阻塞回复主链路。

---

## 10. 画像投影原则

### 10.1 投影是请求，不是直接写入

长期记忆进入画像层时，必须通过领域事件表达为：

`profile.memory_projection.requested.v1`

### 10.2 画像层有最终解释权

`profile-service` 可以：

- 接受投影
- 合并投影
- 延后投影
- 放弃低置信度投影

这保证 `Memory` 和 `Profile` 各自保持边界清晰。

---

## 11. 质量目标

### 11.1 MVP 阶段接受的现实

- `network_type` 分类目标：80% 正确率
- 不追求所有消息都被正确归类
- 不追求一次整合就得到完美认知图

### 11.2 重点优化方向

- 高价值记忆命中率
- 误遗忘率
- 错误长期保留率
- 召回相关性
- 画像投影有效率

---

## 12. 一句话定义

> Memory Layer 不是聊天归档系统，而是 OneLink 的长期认知引擎。
