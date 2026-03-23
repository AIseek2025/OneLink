# OneLink Context / Memory OS Architecture

## 1. 系统定位

### 1.1 核心定义
> `Context / Memory OS` 是 OneLink 的**记忆计算层（Memory Compute Layer）**，负责对用户长期行为数据进行抽取、压缩、建模与上下文组装，为 AI 决策提供最优输入。

### 1.2 非目标
- 不负责原始聊天存储
- 不负责用户画像最终写入
- 不负责推荐结果生成
- 不负责社交关系主存
- 不直接替代 `ai-chat-service`、`profile-service`、`match-service` 的主数据 owner 角色

### 1.3 核心职责
`context-service` 只做三件事：
1. `Memory Extraction`
2. `Memory Distillation`
3. `Context Assembly`

## 2. 核心设计原则

### 2.1 Single Source of Truth
| 数据类型 | Owner Service |
|----------|---------------|
| 原始聊天与会话 | `ai-chat-service` |
| 问卷投放与答案 | `question-service` |
| 用户画像与衍生画像层 | `profile-service` |
| 推荐请求与反馈 | `match-service` |
| 风险治理与处罚 | `safety-service` |
| Memory Artifacts / Memory Summaries / Memory Entities / Memory Entity Links / Agent Runtime Checkpoints / Forgetting Decisions / Context Logs | `context-service` |

### 2.2 Context Is Compute, Not Truth Storage
- `context-service` 是计算层，不是所有原始业务数据的唯一存储
- 它只拥有：
  - `memory_artifacts`
  - `memory_summaries`
  - `memory_entities`
  - `memory_entity_links`
  - `agent_runtime_checkpoints`
  - `forgetting_decisions`
  - `context_logs`
  - 向量索引与缓存状态

### 2.3 Sync for Response, Async for Evolution
- 同步主链路：
  - `/context/build`
  - 任务路由
  - 上下文组装
  - 模型调用前的 token budget 控制
- 异步演化链路：
  - 记忆抽取
  - 摘要压缩
  - 画像投影请求
  - 向量重算

### 2.4 Event-Driven by Default
- 所有长期记忆沉淀、画像投影、推荐学习、风控演化都优先走事件总线
- 不允许业务服务跨服务直接写库

## 3. 总体架构

### 3.1 主链路

```text
User
  -> API Gateway / BFF
  -> ai-chat-service
  -> context-service (Memory Compute Layer)
      -> memory-engine
      -> context-builder
      -> task-router
  -> model-gateway
  -> result-aggregator
  -> response
```

### 3.2 异步演化链路

```text
ai-chat-service / question-service / match-service / safety-service
  -> Event Bus (Kafka)
  -> context-service
      -> memory-extractor
      -> memory-distiller
      -> memory write
  -> profile-service / match-service / safety-service
```

### 3.3 `task-router` 定位
- MVP 阶段是 `context-service` 内部的逻辑组件
- 负责根据任务类型选择：
  - 对话主模型
  - 推理模型
  - 检索增强模式
  - 轻量 vs 重量上下文装配策略
- MVP **不独立部署**

### 3.4 `result-aggregator` 定位
- MVP 阶段作为 `ai-chat-service` 内部结果融合组件存在
- 负责整理 `model-gateway` 返回结果、补齐元数据、形成最终回复
- MVP **不独立部署**

## 4. Context Service 内部架构

```text
context-service
 ├── memory-extractor
 ├── memory-distiller
 ├── context-builder
 ├── memory-store
 ├── vector-index
 └── task-router
```

- MVP 阶段以上 6 个模块全部运行在同一个 `context-service` 进程内
- 工程上可拆为 Rust 的 module / crate，**不允许**在 MVP 阶段独立部署成多个微服务

### 4.1 `memory-extractor`
输入：
- `chat.user_message.created.v1`
- `question.answered.v1`
- 行为反馈事件

输出：
- `context.memory.extracted.v1`
- 原始 `memory_artifact` 候选

### 4.2 `memory-distiller`
功能：
- 去噪
- 合并
- 抽象
- 冲突检测
- 会话摘要生成
- importance / confidence / consistency 评分

输出：
- `context.memory.summary.updated.v1`
- 持久化后的 `memory_artifacts`

### 4.3 `context-builder`
功能：
- 根据用户输入、最近会话、working memory、persistent memory 组装上下文
- 执行 relevance retrieval
- 排序与 token budget 控制
- 产出模型输入包

### 4.4 `memory-store`
存储：
- `memory_summaries`
- `memory_artifacts`
- `memory_entities`
- `memory_entity_links`
- `agent_runtime_checkpoints`
- `forgetting_decisions`
- `context_logs`

### 4.5 `vector-index`
存储：
- `memory_artifact` 的 embedding
- 语义检索索引

## 5. 记忆分层模型

### 5.0 `ephemeral`
- 面向单次请求或极短窗口的瞬时上下文
- 只存在于 Redis session state 或进程内短时缓存
- 不进入 PostgreSQL
- 不进入 `memory_artifacts`

### 5.1 `working`
- 面向当前会话和最近一段时间
- 用于 session 级理解
- 主要由：
  - `Redis`
  - `memory_summaries`
 共同构成

### 5.2 `persistent`
- 面向长期理解
- 存储长期兴趣、目标、关系偏好、能力模型等
- 主要由：
  - `memory_artifacts`
  - 向量索引
共同构成

### 5.3 暂不进入 MVP 的层
- `strategic memory`
- 多区域全局共享记忆层
- 自动长期目标调度引擎

## 6. Memory Artifact 设计

### 6.1 标准结构

```json
{
  "memory_id": "uuid",
  "user_id": "uuid",
  "network_type": "world | experience | opinion | entity",
  "evidence_type": "fact | inference",
  "memory_level": "working | persistent",
  "content": "用户对AI创业非常感兴趣",
  "content_structured": {
    "topic": "AI创业"
  },
  "confidence": 0.87,
  "importance_score": 0.76,
  "consistency_score": 0.92,
  "source_type": "chat | questionnaire | behavior",
  "source_service": "ai-chat-service",
  "source_ref_id": "uuid",
  "source_event_id": "uuid",
  "entity_refs": ["uuid"],
  "version": 1,
  "superseded_by": null,
  "vector_ref": "qdrant://memory_artifacts/uuid",
  "region": "ap-southeast-1",
  "visibility": "private | shared | safety_only",
  "expires_at": null,
  "created_at": "2026-03-20T12:00:00Z",
  "updated_at": "2026-03-20T12:00:00Z"
}
```

### 6.2 Memory Summary 设计

```json
{
  "summary_id": "uuid",
  "user_id": "uuid",
  "conversation_id": "uuid",
  "summary_type": "working_memory",
  "summary_text": "最近会话主要围绕 AI 创业、产品定位与融资准备",
  "key_points_json": [
    "对AI创业高度感兴趣",
    "希望连接投资人与技术合伙人"
  ],
  "source_message_range": {
    "from_message_id": "uuid",
    "to_message_id": "uuid"
  },
  "token_count": 320,
  "updated_at": "2026-03-20T12:00:00Z"
}
```

## 7. Context Assembly

### 7.1 输入
- `user_id`
- 当前输入
- 最近对话
- `memory_summaries`
- `memory_artifacts` Top-K

### 7.2 输出

```json
{
  "system_prompt": "...",
  "user_context": "...",
  "memory_context": "...",
  "task_context": "...",
  "selected_summary_ids": ["uuid"],
  "selected_memory_ids": ["uuid"],
  "token_budget": {
    "max_tokens": 8000,
    "memory_limit": 20,
    "summary_limit": 3
  }
}
```

### 7.3 排序策略
```text
Score =
0.4 * similarity +
0.3 * importance +
0.2 * recency +
0.1 * confidence
```

### 7.4 `/internal/context/build` 约束
- 必须包含 token budget
- 必须限制 memory 选择上限
- 不允许无限制把所有 raw chat 拼进模型
- 当向量检索超时或 `Qdrant` 不可用时，必须降级为：
  - `working memory`
  - 最近 `N` 条 `memory_summaries`
  - 跳过 persistent memory 向量召回

## 8. Memory Distillation

### 8.1 触发机制
- 每 20 条消息
- 每次高价值交互
- 定时任务
- 问卷答案写入后

### 8.2 压缩流程

```text
raw messages
  -> summary
  -> structure extraction
  -> dedup / conflict detection
  -> memory artifact
```

### 8.3 冲突处理
- 标记冲突
- 降低 `confidence`
- 生成澄清任务
- 需要时发出画像投影请求，而非直接改写 profile

## 9. 事件系统

### 9.1 事件总线
- MVP 推荐：`Kafka`
- 后续若存在更复杂多租户与 geo 需求，再评估 `Pulsar`

### 9.2 核心事件
- `chat.user_message.created.v1`
- `context.memory.extracted.v1`
- `context.memory.summary.updated.v1`
- `profile.memory_projection.requested.v1`

### 9.3 标准链路

```text
chat.user_message.created.v1
  -> context.memory.extracted.v1
  -> context.memory.summary.updated.v1
  -> profile.memory_projection.requested.v1
```

### 9.4 事件规则
- `context.memory.extracted.v1` 与 `context.memory.summary.updated.v1` 不允许阻塞聊天主响应
- `profile.memory_projection.requested.v1` 只是投影请求，不等于画像已写入
- 画像最终主写仍归 `profile-service`
- `context.memory.extracted.v1` 允许由 `context-service` 自消费：
  - 这是为了把 extraction、distillation、memory write 解耦成可重试的异步阶段
  - 不是“自己调自己”的反模式，而是事件化的内部流水线

## 10. 数据存储设计

### 10.1 PostgreSQL
存：
- `memory_artifacts`
- `memory_summaries`
- `memory_entities`
- `memory_entity_links`
- `agent_runtime_checkpoints`
- `forgetting_decisions`
- `context_logs`

### 10.2 Redis
存：
- session context
- 热用户上下文
- token cache

### 10.3 Qdrant
存：
- memory artifact embedding
- similarity search index

## 11. API 设计

### 11.1 `POST /internal/context/build`
用途：
- 内部核心同步接口
- 由 `ai-chat-service` 主调，后续 `match-service` 可复用

请求：

```json
{
  "user_id": "uuid",
  "agent_id": "uuid",
  "conversation_id": "uuid",
  "input": "string",
  "task_type": "chat",
  "max_tokens": 8000,
  "memory_limit": 20,
  "summary_limit": 3,
  "reply_style": "brief",
  "retrieval_modes": ["structured", "semantic", "temporal"]
}
```

### 11.2 `POST /internal/memory/write`
用途：
- 内部保留接口
- 绝大多数写入优先来自事件消费者，而不是业务服务直接同步调用

### 11.3 `GET /internal/memory/search`
用途：
- 内部检索接口
- 不直接暴露给前端

## 12. 性能与 SLA

### 12.1 目标预算
- `/context/build`：P95 < 150ms
- 向量检索：P95 < 80ms
- Redis 命中 working context：P95 < 20ms
- 记忆蒸馏：异步，不阻塞用户响应
- 画像投影：最终一致性，秒级完成

### 12.2 分片策略
- 按 `user_id` hash
- 后续按 `region` 叠加分片

### 12.3 缓存策略
- 热用户 working memory 缓存
- 高频 artifact 检索缓存
- token budget 计算结果短暂缓存

## 13. 风险与对策

### 13.1 Memory 爆炸
- TTL
- 分层压缩
- importance 降级淘汰
- `expires_at` 到期的低价值记忆由定时任务清理，热点读取时可做 lazy eviction

### 13.2 Context 过大
- token budget
- summary_limit
- memory_limit

### 13.3 错误记忆
- `confidence`
- `consistency_score`
- 用户确认
- 冲突追踪

### 13.4 越权读取
- `visibility`
- 数据域隔离
- 区域与合规标签

## 14. MVP v1 冻结版

### 14.1 范围
- `working` + `persistent`
- `memory_summaries`
- `memory_artifacts`
- `memory_entities`
- `memory_entity_links`
- `agent_runtime_checkpoints`
- `forgetting_decisions`
- `context_logs`
- `POST /internal/context/build`
- `Kafka`
- `PostgreSQL + Redis + Qdrant`

### 14.2 服务边界
- `context-service`：memory compute owner
- `ai-chat-service`：conversation owner
- `profile-service`：profile owner

### 14.3 核心事件
- `chat.user_message.created.v1`
- `context.memory.extracted.v1`
- `context.memory.summary.updated.v1`
- `profile.memory_projection.requested.v1`

### 14.4 暂不做
- strategic memory
- 多区域全局记忆同步
- 独立部署的 `task-router`
- 面向前端的 memory 查询接口

## 15. 最终总结

> Context / Memory OS 不是“存用户说过什么”，而是“决定什么值得被记住，以及什么时候被使用”。

> OneLink 的核心不是“连接人”，而是“长期建模人”。
