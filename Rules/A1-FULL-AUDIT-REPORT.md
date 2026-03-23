# OneLink 全套文档总审查报告

> 审查人：Opus 4.6
> 日期：2026-03-20
> 范围：`Rules/00` ~ `Rules/13`，共 14 份文档
> 重点：跨文档口径一致性、主键与写路径、事件模型完整性、缺失定义、事件风暴风险

---

## 总体结论

核心架构文档（`10-SERVICE-BOUNDARIES.md`、`11-DATA-EVENT-MODEL.md`、`02-TECH-ARCHITECTURE.md`、`13-COMPOSER-1.5-EXECUTION-BRIEF.md`）之间的口径已经高度统一。MVP 的服务集合、语言选型、数据拥有权、事件命名、问卷入 MVP、BFF 入 MVP、AI 对话与私信拆分等核心决策在这四份文档中完全一致。

但有 **3 份旧文档未同步更新**，存在直接冲突；另外数据模型中有 **4 处结构性缺失** 需要在 Composer 1.5 动手前修正。

以下按严重程度从高到低排列。

---

## 一、跨文档口径冲突（必须修正）

### 1.1 `00-EXECUTIVE-BLUEPRINT.md` 第 5.1 节 — 语言职责过时

**现状**

```
通用业务与基础设施服务：Go
```

**应为**

当前 MVP 已决定核心后端统一 Rust，Go 只用于非 MVP 辅助服务。

**影响**

`00` 是总蓝图，按其第 8 节约束力规则"后续文档与本蓝图冲突，以本蓝图为准"，如果不更新 `00`，团队可能误判 Go 仍然是默认选择。

**建议**

更新 `00` 第 5.1 节，与 `02` 第 2.1 节对齐。

---

### 1.2 `07-ENGINEERING-RULES.md` 第 4.2 节 — 后端语言规则过时

**现状**

```
Rust 仅用于性能敏感核心链路
Go 用于通用服务和基础设施服务
```

**应为**

Rust 为 MVP 默认后端主语言，Go 保留给后续辅助服务。

**影响**

这是团队编码标准文件。如果不更新，代码审查时会产生争议。

**建议**

更新第 4.2 节，与 `02` 第 2.1 节和 `10` 全局口径对齐。

---

### 1.3 `08-DELIVERY-ROADMAP.md` — 多处过时

**问题 A：Phase 1 团队配置（第 8.1 节）**

仍然列出"Go 工程师"作为 Phase 1 独立角色。在 Rust-first 策略下，应改为强调 Rust 工程师为主力。

**问题 B：Phase 3 新能力（第 5.2 节）**

仍列出"独立问题服务"。但 `question-service` 已经在 MVP 中。

**建议**

- 第 8.1 节：把"Go 工程师"改为"Rust 工程师（主力）"，可保留"Go 工程师（辅助工具链）"
- 第 5.2 节：删除"独立问题服务"，改为"扩大 question-service 的动态题库和试投放能力"

---

## 二、数据模型缺失（必须补齐后才能交给 Composer）

### 2.1 `profile_fact_revisions` — 有名字无定义

**现状**

- 出现在 `11` 第 3.4 节域对象列表
- 出现在 `11` 第 10 节 MVP 优先表清单
- 出现在 `13` 第 5.2 节 Composer 必做表
- **但第 4 节核心表草案中没有表定义**

**影响**

Composer 1.5 在写 SQL 时没有字段参考，会自己发明字段结构，可能偏离意图。

**建议**

在 `11` 第 4.12 节 `profile_facts` 后补充 `profile_fact_revisions` 表定义：

```
profile_fact_revisions
- id
- fact_id
- previous_value_json
- previous_confidence
- previous_status
- changed_by (system / user / model)
- changed_at
```

---

### 2.2 `sessions` 和 `verification_attempts` — 有名字无定义

**现状**

- 出现在 `11` 第 3.1 节域对象列表
- `13` 第 5.1 节要求 Composer 做这两张表的 SQL
- **但第 4 节没有表定义**

**建议**

补充两张表的最小定义：

```
sessions
- id
- user_id
- token_hash
- device_info
- ip_address
- expires_at
- created_at

verification_attempts
- id
- user_id (nullable, 注册前可能没有)
- channel (email / sms)
- target_hash
- code_hash
- status (pending / verified / expired / failed)
- attempted_at
- verified_at
```

---

### 2.3 `question_variants` 不在 MVP 优先表清单

**现状**

- 有域对象定义（3.5）、有表草案（4.18）
- `question-service` 拥有数据中列出了 `question variants`
- `13` Composer 任务书第 5.5 节要求做 `question_variants` 但 `11` 第 10 节 MVP 清单没有

**建议**

在 `11` 第 10 节 MVP 清单的问卷域部分加入 `question_variants`。否则 Composer 会在两个权威来源之间矛盾。

---

### 2.4 `trait_supporting_facts` 不在 MVP 优先表清单

**现状**

- 有表草案（4.14）
- `13` Composer 任务书第 5.2 节明确要求
- `11` 第 10 节 MVP 清单没列

**建议**

在 `11` 第 10 节画像域部分加入 `trait_supporting_facts`。

---

## 三、主键与写路径风险

### 3.1 `ai_messages.content_ref_id` 与 `ai_message_contents.message_id` 双向引用

**现状**

- `ai_messages` 有 `content_ref_id`（指向内容）
- `ai_message_contents` 有 `message_id`（指向消息）

**风险**

双向外键在写入时需要两步：先写一侧再回填另一侧，在 Rust 异步上下文中容易造成部分写入。

**建议**

删掉 `ai_messages.content_ref_id`，只保留 `ai_message_contents.message_id` 单向引用。查询时从 `ai_messages` JOIN `ai_message_contents` 即可。

---

### 3.2 `question_deliveries.question_id` 指向不明确

**现状**

`question_id` 没有说明是指向 `question_templates.id` 还是 `question_variants.id`。

**风险**

如果指向 template，则无法追溯具体投放了哪个变体文本。如果指向 variant，则字段名容易误导。

**建议**

改为 `variant_id`，明确指向 `question_variants.id`。同时在 `question_answers` 的 `question_id` 也改为 `variant_id`。这样从 variant 可以回溯到 template。

---

### 3.3 `question_answers` 与 `question_deliveries` 的 `answer_state` / `status` 重叠

**现状**

- `question_deliveries.status` 有 `answered` 状态
- `question_answers.answer_state` 也有 `answered` 状态

**风险**

两张表同时记录"是否已回答"，在并发写入时可能不一致。

**建议**

- `question_deliveries.status` 只表示投放的生命周期：`delivered` → `answered` / `skipped` / `expired`
- `question_answers` 只在用户实际提交答案后才创建记录
- Composer 在 SQL 注释中必须说明：delivery 的 `answered` 状态由 `question-service` 在收到 answer 后同步更新，不允许其他服务直接改

---

## 四、事件模型风险

### 4.1 缺少 `profile.updated.v1` 事件

**现状**

画像域事件只覆盖了 discovery_preference、visibility_rule、fact、trait、summary、embedding 的变更。

**缺失**

用户直接编辑 `profiles` 表字段（display_name、bio、headline 等）时，没有对应事件。

**影响**

- `match-service` 无法实时更新召回索引中的公开标签
- `bff` 无法做缓存失效
- 推荐名片可能展示过时的用户简介

**建议**

在画像域事件中补充 `profile.profile.updated.v1`。

---

### 4.2 事件风暴量级评估

**场景**

每条用户聊天消息触发的最大事件链：

1. `chat.user_message.created.v1`
2. `profile.fact.extracted.v1`（可能 0-N 条）
3. `profile.fact.upserted.v1`（每个新事实 1 条）
4. `profile.trait.updated.v1`
5. `profile.embedding.updated.v1`
6. `profile.summary.updated.v1`

**估算**

以 1M DAU、日均 10 条消息计算：
- 聊天事件：1000 万/天
- 假设 30% 消息触发事实抽取，平均每次提取 1.5 个事实
- 事实相关事件：1000 万 × 0.3 × 1.5 × 3（extracted + upserted + trait）= 1350 万
- embedding + summary：1000 万 × 0.3 = 300 万
- **总计约 2650 万事件/天，仅来自聊天链路**

加上问卷、匹配、安全、社交、模型调用等域，MVP 日事件量级约在 **3000-5000 万/天**。

**结论**

这个量级 Kafka 完全能承载（单集群轻松扛住 10 亿/天级别）。但需要注意：
- 画像管线消费者不能串行处理——必须按 `user_id` 分区并行消费
- `profile-service` 的异步消费者需要 backpressure 机制
- embedding 重算是最慢的一步（需要调用 model-gateway），必须有限流

**建议**

在 Composer 1.5 的事件 schema 中，为 `profile.fact.upserted.v1` 和 `profile.embedding.updated.v1` 标注 `high_throughput: true`，提醒工程团队优先做消费者性能优化。

---

### 4.3 `question.delivery.created.v1` 与 `question.delivered.v1` 语义模糊

**现状**

两个事件名容易混淆。

**建议**

在任务书或事件 schema 中明确：
- `question.delivery.created.v1`：投放记录写入数据库时发出（内部事件）
- `question.delivered.v1`：问题实际展示给用户时发出（用户侧事件）

如果 MVP 中两者是同步发生的（创建投放记录 = 立即展示），可以合并为一个事件，Phase 2 再拆。

---

## 五、ER 关系图缺失

`11` 第 5 节 ER 图缺少以下关系：

| 缺失关系 | 应有表达 |
|---------|---------|
| `users` → `dm_participants` | `users \|\|--o{ dm_participants : participates` |
| `users` → `user_blocks` | `users \|\|--o{ user_blocks : blocks` |
| `question_templates` → `question_variants` | `question_templates \|\|--o{ question_variants : has` |
| `question_variants` → `question_deliveries` | `question_variants \|\|--o{ question_deliveries : delivers` |
| `question_deliveries` → `question_answers` | `question_deliveries \|\|--o{ question_answers : answers` |

不补也不影响 Composer 工作，但会让新加入的工程师对问卷和私信链路产生误解。

---

## 六、`03-AI-PROFILE-QUESTIONNAIRE.md` 小问题

第 6.4 节（MVP 问卷策略）出现在第 6.3 节（问题对象）之前，编号倒序。这是 GPT 追加内容时插入位置不当导致的。不影响语义，但建议调整编号顺序。

---

## 七、`12-ARCHITECTURE-REVIEW.md` 历史状态

这份文档是第一轮审查报告，其中多条建议已被后续决策覆盖（如 BFF 入 MVP、chat 拆分、question 入 MVP、Rust-first）。当前文档内容本身没有错误（它忠实记录了当时的审查意见），但可能让新读者误以为这些建议尚未执行。

**建议**

在文件头部追加一行：

```
> 状态：本报告为首轮审查记录。其中多条建议已在后续迭代中被采纳或被更高级别决策覆盖，最终口径以 `10-SERVICE-BOUNDARIES.md` 和 `11-DATA-EVENT-MODEL.md` 为准。
```

---

## 八、总结：Composer 1.5 动手前必须完成的修正

| 优先级 | 事项 | 影响 |
|--------|------|------|
| P0 | 在 `11` 中补充 `profile_fact_revisions` 表定义 | Composer 无字段参考 |
| P0 | 在 `11` 中补充 `sessions` 和 `verification_attempts` 表定义 | Composer 无字段参考 |
| P0 | 在 `11` 第 10 节 MVP 清单补入 `question_variants` 和 `trait_supporting_facts` | 与 `13` 任务书矛盾 |
| P0 | 在 `11` 中把 `question_deliveries.question_id` 改为 `variant_id`，`question_answers` 同理 | 外键指向不明确 |
| P0 | 删除 `ai_messages.content_ref_id`，只保留单向引用 | 双向 FK 写入风险 |
| P1 | 更新 `00` 第 5.1 节语言职责 | 蓝图与实际冲突 |
| P1 | 更新 `07` 第 4.2 节后端语言规则 | 编码标准与实际冲突 |
| P1 | 更新 `08` 第 8.1 节团队配置和第 5.2 节 Phase 3 内容 | 路线图与实际冲突 |
| P1 | 在 `11` 事件清单补充 `profile.profile.updated.v1` | 主页编辑不可见 |
| P2 | 补齐 ER 图缺失关系 | 新成员理解成本 |
| P2 | 修正 `03` 第 6.3/6.4 节编号顺序 | 可读性 |
| P2 | 在 `12` 头部标注历史状态 | 避免误读 |

**P0 项全部解决后，`13-COMPOSER-1.5-EXECUTION-BRIEF.md` 才算真正可执行。**

如果你同意，我可以现在直接帮你把这些 P0 和 P1 修正全部落到对应文档里。
