# OneLink V2 Data Model

## 1. 文档目标

把 `Rules-V2` 的架构冻结结果下沉成第一版数据模型基线，避免架构文档与后续 SQL / 事件 / 内部契约再次脱节。

本文件先冻结：

- 核心对象
- 主键与 owner
- 关键字段
- 设计约束

不在本文件中输出完整 SQL。

---

## 2. 核心设计原则

### 2.1 主写权唯一

所有表都必须能映射到唯一 owner service。

### 2.2 原始数据、长期记忆、画像事实分层

以下三层不得混写：

- 原始聊天 / 原始行为
- 长期记忆
- 画像事实

### 2.3 可回放优先

与记忆整合、策略优化、运行时 checkpoint 相关的对象，都必须具备回放与版本化能力。

---

## 3. Memory Domain

### 3.1 `memory_artifacts`

owner: `context-service`

关键字段：

- `id`
- `user_id`
- `network_type`
  - `world`
  - `experience`
  - `opinion`
  - `entity`
- `evidence_type`
  - `fact`
  - `inference`
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
  - `working`
  - `persistent`
- `visibility`
  - `private`
  - `shared`
  - `safety_only`
- `vector_ref`
- `region`
- `expires_at`
- `created_at`
- `updated_at`

约束：

- `network_type` 是大分类，不承担所有细分业务语义
- 精细标签放入 `content_structured`
- `superseded_by` 为 MVP 强制字段；反向关系 `supersedes` 保留为 Phase 2 扩展位

### 3.2 `memory_summaries`

owner: `context-service`

关键字段：

- `id`
- `user_id`
- `conversation_id`
- `summary_type`
- `summary_text`
- `key_points_json`
- `source_message_range`
- `token_count`
- `policy_version`
- `updated_at`

约束：

- 这是 `Working Memory` 的持久化摘要层
- 不是画像摘要
- 不是原始聊天替代品

### 3.3 `memory_entities`

owner: `context-service`

关键字段：

- `id`
- `user_id`
- `entity_type`
- `name`
- `aliases`
- `attributes`
- `vector_ref`
- `created_at`
- `updated_at`

### 3.4 `memory_entity_links`

owner: `context-service`

关键字段：

- `id`
- `user_id`
- `source_entity_id`
- `target_entity_id`
- `relation_type`
- `confidence`
- `evidence_artifact_id`
- `is_bidirectional`
- `created_at`

约束：

- MVP 虽然未默认激活图检索，但这张表从第一天就建模

### 3.5 `context_logs`

owner: `context-service`

关键字段：

- `id`
- `user_id`
- `conversation_id`
- `input_ref_id`
- `selected_summary_ids`
- `selected_memory_ids`
- `retrieval_modes`
- `task_type`
- `token_budget_json`
- `model_context_size`
- `created_at`

---

## 4. Session Domain

### 4.1 `agent_runtime_checkpoints`

owner: `context-service`

关键字段：

- `id`
- `agent_id`
- `user_id`
- `conversation_id`
- `schema_version`
- `working_summary_ref`
- `runtime_state_blob`
- `policy_versions_json`
- `created_at`

约束：

- 所有 checkpoint 必须版本化
- 必须支持向前兼容读取

### 4.2 `conversation_archives`

owner: `ai-chat-service`

关键字段：

- `id`
- `conversation_id`
- `archive_tier`
  - `hot`
  - `warm`
  - `cold`
- `storage_ref`
- `content_hash`
- `archived_at`

约束：

- 用于长会话与冷存储管理
- 不替代 `ai_messages / ai_message_contents`

### 4.3 `forgetting_decisions`

owner: `context-service`

关键字段：

- `id`
- `user_id`
- `target_type`
- `target_id`
- `decision`
  - `retain`
  - `summarize`
  - `archive_only`
  - `forget_from_hot_layer`
- `reason_codes`
- `policy_version`
- `cold_storage_ref`
- `created_at`

约束：

- 遗忘必须可追溯
- 冷层原文不能无痕消失

---

## 5. Optimization Domain

### 5.1 `policy_configs`

owner: `optimization-layer`

关键字段：

- `id`
- `policy_key`
- `policy_domain`
- `value_type`
- `default_value`
- `allowed_range_json`
- `current_value`
- `status`
- `updated_at`

### 5.2 `policy_experiments`

owner: `optimization-layer`

关键字段：

- `id`
- `experiment_name`
- `policy_domain`
- `hypothesis`
- `dataset_version`
- `candidate_values_json`
- `status`
- `created_at`
- `completed_at`

### 5.3 `policy_rollouts`

owner: `optimization-layer`

关键字段：

- `id`
- `experiment_id`
- `rollout_stage`
  - `replay`
  - `shadow`
  - `canary`
  - `full`
  - `rollback`
- `target_scope`
- `result_metrics_json`
- `created_at`

---

## 6. Persona Domain

### 6.1 `persona_interaction_policies`

owner: `ai-friend-persona-layer`

关键字段：

- `id`
- `language`
- `humor_level`
- `comfort_level`
- `reply_length_default`
- `ask_depth_default`
- `persuasion_style`
- `policy_version`
- `updated_at`

约束：

- 只能表达 `Interaction Policy`
- 不能覆盖 `Persona Constitution`

### 6.2 `user_persona_preferences`

owner: `ai-friend-persona-layer`

关键字段：

- `id`
- `user_id`
- `preferred_tone`
- `directness_level`
- `proactive_care_opt_in`
- `nickname_preference`
- `sensitive_topic_boundaries`
- `updated_at`

---

## 7. 与 V1 表的映射

### 7.1 继续沿用的对象

- `ai_conversations`
- `ai_messages`
- `ai_message_contents`
- `profile_facts`
- `profile_traits`
- `question_*`
- `recommendation_*`
- `moderation_*`

### 7.2 需要在级联更新中替换或升级的对象

- V1 的 `memory_type` 大分类，升级为 `network_type`
- `memory_artifacts` 从轻量 schema 升级为 V2 结构
- 引入 `memory_entities` 与 `memory_entity_links`
- 引入 `agent_runtime_checkpoints`
- 引入 `policy_configs / policy_experiments / policy_rollouts`

---

## 8. MVP 默认激活与预埋项

### 8.1 MVP 默认激活

- `memory_artifacts`
- `memory_summaries`
- `context_logs`
- `agent_runtime_checkpoints`
- `forgetting_decisions`
- `policy_configs`

### 8.2 MVP 预埋但未完全激活

- `memory_entities`
- `memory_entity_links`
- `policy_experiments`
- `policy_rollouts`
- `persona_interaction_policies`

---

## 9. 一句话定义

> V2 数据模型的目标不是一次写完所有 DDL，而是先把 owner、对象边界和未来级联更新的方向钉死。
