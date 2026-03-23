# OneLink MVP SQL Schema Draft

> 基于 `11-DATA-EVENT-MODEL.md` 和 `10-SERVICE-BOUNDARIES.md` 产出
> 目标：可建表的第一版 DDL 初稿

---

## 1. 总体约定

### 1.1 主键策略
- 使用 `ulid` 或 `uuid` 类型（PostgreSQL 需 `pgcrypto` 或 `gen_random_uuid()`）
- 建议：`id UUID PRIMARY KEY DEFAULT gen_random_uuid()`
- 时间有序 ID 有利于分区表写入（可选 ULID 扩展）

### 1.2 时间戳
- 统一使用 `timestamptz`
- 创建时间：`created_at timestamptz NOT NULL DEFAULT now()`
- 更新时间：`updated_at timestamptz NOT NULL DEFAULT now()`

### 1.3 外键策略
- **高写入表**（`ai_messages`、`ai_message_contents`、`recommendation_feedbacks`）：MVP 不加外键约束，仅建索引
- **其他表**：可加外键，但需评估写入量

### 1.4 分区与冷热分层
- `ai_messages`：按 `created_at` 月分区
- `ai_message_contents`：按 `created_at` 月分区
- `recommendation_feedbacks`：MVP 可先不分区，Phase 2 迁移事件流
- `profile_fact_revisions`：按 `changed_at` 或 `fact_id` hash 分区

---

## 2. 按服务分组的表清单

| 服务 | 表 | 写权限 |
|------|-----|--------|
| identity-service | users, identity_bindings, sessions, verification_attempts | identity-service |
| profile-service | profiles, discovery_preferences, follows, profile_facts, profile_fact_revisions, profile_traits, trait_supporting_facts, profile_summaries, profile_embeddings | profile-service |
| context-service | memory_summaries, memory_artifacts, memory_entities, memory_entity_links, agent_runtime_checkpoints, forgetting_decisions, context_logs | context-service |
| ai-chat-service | ai_conversations, ai_messages, ai_message_contents | ai-chat-service |
| dm-service | dm_threads, dm_participants, dm_messages | dm-service |
| question-service | question_templates, question_variants, question_deliveries, question_answers | question-service |
| match-service | find_requests, recommendation_result_sets, recommendation_cards, recommendation_feedbacks | match-service |
| safety-service | risk_assessments, report_tickets, moderation_actions, appeal_cases, user_blocks | safety-service |
| model-gateway | model_invocation_logs | model-gateway |

---

## 3. DDL 草案

### 3.1 账户域（identity-service）

```sql
-- users
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    status VARCHAR(32) NOT NULL DEFAULT 'active',
    primary_region VARCHAR(64),
    primary_language VARCHAR(16),
    timezone VARCHAR(64),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_users_status ON users(status);
CREATE INDEX idx_users_created_at ON users(created_at);

-- identity_bindings
CREATE TABLE identity_bindings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider VARCHAR(64) NOT NULL,
    provider_subject VARCHAR(256),
    email_or_phone_hash VARCHAR(128),
    is_primary BOOLEAN NOT NULL DEFAULT false,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE(provider, provider_subject)
);

CREATE INDEX idx_identity_bindings_user_id ON identity_bindings(user_id);
-- 注：UNIQUE(provider, provider_subject) 在 provider_subject 为 NULL 时不生效（PostgreSQL NULL != NULL）。
-- email/sms 等 provider_subject 可为空的场景，需在应用层或 conditional unique index 补充去重：
-- CREATE UNIQUE INDEX idx_identity_bindings_provider_null_subject
--     ON identity_bindings(provider, user_id) WHERE provider_subject IS NULL;

-- sessions
CREATE TABLE sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash VARCHAR(128) NOT NULL,
    device_info TEXT,
    ip_address INET,
    expires_at timestamptz NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_sessions_token_hash ON sessions(token_hash);
CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_expires_at ON sessions(expires_at);

-- verification_attempts
CREATE TABLE verification_attempts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    channel VARCHAR(16) NOT NULL CHECK (channel IN ('email', 'sms')),
    target_hash VARCHAR(128) NOT NULL,
    code_hash VARCHAR(128) NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'verified', 'expired', 'failed')),
    attempted_at timestamptz NOT NULL DEFAULT now(),
    verified_at timestamptz
);

CREATE INDEX idx_verification_attempts_target_hash ON verification_attempts(target_hash);
CREATE INDEX idx_verification_attempts_attempted_at ON verification_attempts(attempted_at);
```

### 3.2 资料与画像域（profile-service）

```sql
-- profiles
CREATE TABLE profiles (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    display_name VARCHAR(128),
    avatar_url TEXT,
    headline VARCHAR(256),
    bio TEXT,
    city_level_location VARCHAR(128),
    languages TEXT[],
    is_searchable BOOLEAN NOT NULL DEFAULT true,
    allow_discovery BOOLEAN NOT NULL DEFAULT true,
    updated_at timestamptz NOT NULL DEFAULT now()
);

-- discovery_preferences
CREATE TABLE discovery_preferences (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    can_be_found BOOLEAN NOT NULL DEFAULT true,
    accepted_request_types TEXT[],
    accepted_languages TEXT[],
    accepted_regions TEXT[],
    max_intro_frequency_per_week INT,
    allow_cross_language BOOLEAN NOT NULL DEFAULT false,
    available_time_windows JSONB,
    updated_at timestamptz NOT NULL DEFAULT now()
);

-- follows
CREATE TABLE follows (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    follower_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    followee_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    source VARCHAR(32) CHECK (source IN ('recommendation', 'search', 'profile_visit')),
    created_at timestamptz NOT NULL DEFAULT now(),
    unfollowed_at timestamptz
);

-- 部分唯一索引：同一对用户只能有一条活跃关注记录，取关后可重新关注
CREATE UNIQUE INDEX idx_follows_active
    ON follows(follower_user_id, followee_user_id)
    WHERE unfollowed_at IS NULL;
CREATE INDEX idx_follows_follower ON follows(follower_user_id);
CREATE INDEX idx_follows_followee ON follows(followee_user_id);
CREATE INDEX idx_follows_created_at ON follows(created_at);

-- profile_facts
CREATE TABLE profile_facts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    fact_type VARCHAR(64) NOT NULL,
    fact_key VARCHAR(128) NOT NULL,
    fact_value_json JSONB NOT NULL,
    source_type VARCHAR(32) NOT NULL,
    source_ref_id UUID,
    confidence DECIMAL(5,4),
    status VARCHAR(32) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'superseded', 'disputed', 'pending_confirmation')),
    effective_time timestamptz,
    captured_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_profile_facts_user_id ON profile_facts(user_id);
CREATE INDEX idx_profile_facts_user_fact_key ON profile_facts(user_id, fact_key);
CREATE INDEX idx_profile_facts_captured_at ON profile_facts(captured_at);

-- profile_fact_revisions（高写入，建议 Phase 2 按 changed_at 月分区）
CREATE TABLE profile_fact_revisions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fact_id UUID NOT NULL REFERENCES profile_facts(id) ON DELETE CASCADE,
    previous_value_json JSONB,
    previous_confidence DECIMAL(5,4),
    previous_status VARCHAR(32),
    changed_by VARCHAR(16) NOT NULL CHECK (changed_by IN ('system', 'user', 'model')),
    changed_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_profile_fact_revisions_fact_id ON profile_fact_revisions(fact_id);
CREATE INDEX idx_profile_fact_revisions_changed_at ON profile_fact_revisions(changed_at);

-- profile_traits
CREATE TABLE profile_traits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    trait_type VARCHAR(64) NOT NULL,
    trait_key VARCHAR(128) NOT NULL,
    trait_score DECIMAL(5,4),
    model_version VARCHAR(64),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_profile_traits_user_id ON profile_traits(user_id);

-- trait_supporting_facts
CREATE TABLE trait_supporting_facts (
    trait_id UUID NOT NULL REFERENCES profile_traits(id) ON DELETE CASCADE,
    fact_id UUID NOT NULL REFERENCES profile_facts(id) ON DELETE CASCADE,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (trait_id, fact_id)
);

-- profile_embeddings
CREATE TABLE profile_embeddings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    embedding_type VARCHAR(32) NOT NULL CHECK (embedding_type IN ('match_profile', 'question_targeting', 'safety_context')),
    vector_ref TEXT NOT NULL,
    model_version VARCHAR(64),
    source_fact_version UUID,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_profile_embeddings_user_type ON profile_embeddings(user_id, embedding_type);

-- profile_summaries
CREATE TABLE profile_summaries (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    public_summary TEXT,
    internal_summary TEXT,
    model_version VARCHAR(64),
    source_fact_version UUID,
    updated_at timestamptz NOT NULL DEFAULT now()
);
```

### 3.3 记忆计算域（context-service）

```sql
-- memory_summaries
CREATE TABLE memory_summaries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    conversation_id UUID NOT NULL,
    summary_type VARCHAR(32) NOT NULL DEFAULT 'working_memory',
    summary_text TEXT NOT NULL,
    key_points_json JSONB,
    source_message_range JSONB,
    token_count INT,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_memory_summaries_user_id ON memory_summaries(user_id);
CREATE INDEX idx_memory_summaries_conversation_id ON memory_summaries(conversation_id);
CREATE INDEX idx_memory_summaries_updated_at ON memory_summaries(updated_at);

-- memory_artifacts
CREATE TABLE memory_artifacts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    network_type VARCHAR(16) NOT NULL CHECK (network_type IN ('world', 'experience', 'opinion', 'entity')),
    evidence_type VARCHAR(16) NOT NULL DEFAULT 'fact' CHECK (evidence_type IN ('fact', 'inference')),
    memory_level VARCHAR(16) NOT NULL CHECK (memory_level IN ('working', 'persistent')),
    content TEXT NOT NULL,
    content_structured JSONB,
    source_type VARCHAR(32) NOT NULL CHECK (source_type IN ('chat', 'questionnaire', 'behavior')),
    source_service VARCHAR(64) NOT NULL,
    source_ref_id UUID,
    source_event_id UUID,
    valid_from timestamptz,
    valid_until timestamptz,
    entity_refs JSONB,
    confidence DECIMAL(5,4),
    importance_score DECIMAL(5,4),
    consistency_score DECIMAL(5,4),
    version INT NOT NULL DEFAULT 1,
    superseded_by UUID REFERENCES memory_artifacts(id),
    visibility VARCHAR(16) NOT NULL DEFAULT 'private' CHECK (visibility IN ('private', 'shared', 'safety_only')),
    vector_ref TEXT,
    region VARCHAR(64),
    expires_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_memory_artifacts_user_id ON memory_artifacts(user_id);
CREATE INDEX idx_memory_artifacts_user_level ON memory_artifacts(user_id, memory_level);
CREATE INDEX idx_memory_artifacts_network_type ON memory_artifacts(network_type);
CREATE INDEX idx_memory_artifacts_source_event_id ON memory_artifacts(source_event_id);
CREATE INDEX idx_memory_artifacts_expires_at ON memory_artifacts(expires_at);
CREATE INDEX idx_memory_artifacts_updated_at ON memory_artifacts(updated_at);

-- memory_entities
CREATE TABLE memory_entities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    entity_type VARCHAR(32) NOT NULL,
    name TEXT NOT NULL,
    aliases JSONB,
    attributes JSONB,
    vector_ref TEXT,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_memory_entities_user_id ON memory_entities(user_id);
CREATE INDEX idx_memory_entities_entity_type ON memory_entities(entity_type);

-- memory_entity_links
CREATE TABLE memory_entity_links (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    source_entity_id UUID NOT NULL REFERENCES memory_entities(id) ON DELETE CASCADE,
    target_entity_id UUID NOT NULL REFERENCES memory_entities(id) ON DELETE CASCADE,
    relation_type VARCHAR(64) NOT NULL,
    confidence DECIMAL(5,4),
    evidence_artifact_id UUID REFERENCES memory_artifacts(id) ON DELETE SET NULL,
    is_bidirectional BOOLEAN NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_memory_entity_links_user_id ON memory_entity_links(user_id);
CREATE INDEX idx_memory_entity_links_source_target ON memory_entity_links(source_entity_id, target_entity_id);

-- agent_runtime_checkpoints
CREATE TABLE agent_runtime_checkpoints (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    agent_id UUID NOT NULL,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    conversation_id UUID,
    schema_version INT NOT NULL DEFAULT 1,
    working_summary_ref UUID,
    runtime_state_blob JSONB NOT NULL,
    policy_versions_json JSONB,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_agent_runtime_checkpoints_user_id ON agent_runtime_checkpoints(user_id);
CREATE INDEX idx_agent_runtime_checkpoints_agent_id ON agent_runtime_checkpoints(agent_id);

-- forgetting_decisions
CREATE TABLE forgetting_decisions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_type VARCHAR(32) NOT NULL,
    target_id UUID NOT NULL,
    decision VARCHAR(32) NOT NULL CHECK (decision IN ('retain', 'summarize', 'archive_only', 'forget_from_hot_layer')),
    reason_codes JSONB,
    policy_version VARCHAR(64),
    cold_storage_ref TEXT,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_forgetting_decisions_user_id ON forgetting_decisions(user_id);
CREATE INDEX idx_forgetting_decisions_target ON forgetting_decisions(target_type, target_id);

-- context_logs
CREATE TABLE context_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    conversation_id UUID,
    input_ref_id UUID,
    selected_summary_ids UUID[],
    selected_memory_ids UUID[],
    task_type VARCHAR(64),
    token_budget_json JSONB NOT NULL,
    model_context_size INT,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_context_logs_user_id ON context_logs(user_id);
CREATE INDEX idx_context_logs_conversation_id ON context_logs(conversation_id);
CREATE INDEX idx_context_logs_created_at ON context_logs(created_at);
```

### 3.4 AI 对话域（ai-chat-service）

```sql
-- ai_conversations
CREATE TABLE ai_conversations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status VARCHAR(32) NOT NULL DEFAULT 'active',
    last_message_at timestamptz,
    context_version INT NOT NULL DEFAULT 0,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_ai_conversations_user_id ON ai_conversations(user_id);

-- ai_messages（高写入，不加 FK 到 contents；按 created_at 月分区）
CREATE TABLE ai_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID NOT NULL REFERENCES ai_conversations(id) ON DELETE CASCADE,
    sender_type VARCHAR(16) NOT NULL CHECK (sender_type IN ('user', 'assistant', 'system')),
    content_type VARCHAR(32) NOT NULL DEFAULT 'text',
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_ai_messages_conversation_id ON ai_messages(conversation_id);
CREATE INDEX idx_ai_messages_created_at ON ai_messages(created_at);

-- ai_message_contents（高写入，单向引用 message_id；按 created_at 月分区）
CREATE TABLE ai_message_contents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_id UUID NOT NULL,
    content_text TEXT,
    content_metadata JSONB,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_ai_message_contents_message_id ON ai_message_contents(message_id);
CREATE INDEX idx_ai_message_contents_created_at ON ai_message_contents(created_at);
-- 注：message_id 不加重 FK，避免高写入锁竞争；应用层保证一致性
```

### 3.5 私信域（dm-service）

```sql
-- dm_threads
CREATE TABLE dm_threads (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    thread_type VARCHAR(16) NOT NULL CHECK (thread_type IN ('stranger', 'connected')),
    status VARCHAR(32) NOT NULL DEFAULT 'active',
    created_at timestamptz NOT NULL DEFAULT now()
);

-- dm_participants
CREATE TABLE dm_participants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    thread_id UUID NOT NULL REFERENCES dm_threads(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(16) NOT NULL CHECK (role IN ('initiator', 'recipient')),
    status VARCHAR(16) NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'muted', 'left')),
    last_read_message_id UUID,
    joined_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_dm_participants_user_updated ON dm_participants(user_id, updated_at DESC);
CREATE INDEX idx_dm_participants_thread_id ON dm_participants(thread_id);

-- dm_messages
CREATE TABLE dm_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    thread_id UUID NOT NULL REFERENCES dm_threads(id) ON DELETE CASCADE,
    sender_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content_type VARCHAR(32) NOT NULL DEFAULT 'text',
    content_text TEXT NOT NULL,
    review_status VARCHAR(32) DEFAULT 'pending' CHECK (review_status IN ('pending', 'approved', 'rejected')),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_dm_messages_thread_id ON dm_messages(thread_id);
CREATE INDEX idx_dm_messages_created_at ON dm_messages(created_at);
```

### 3.6 问卷域（question-service）

```sql
-- question_templates
CREATE TABLE question_templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    dimension VARCHAR(64) NOT NULL,
    subdimension VARCHAR(64),
    question_style VARCHAR(32) NOT NULL,
    template_text TEXT NOT NULL,
    sensitivity_level INT,
    status VARCHAR(32) NOT NULL DEFAULT 'active',
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_question_templates_dimension ON question_templates(dimension);

-- question_variants
CREATE TABLE question_variants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    template_id UUID NOT NULL REFERENCES question_templates(id) ON DELETE CASCADE,
    variant_text TEXT NOT NULL,
    generation_source VARCHAR(32),
    review_status VARCHAR(32) DEFAULT 'pending',
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_question_variants_template_id ON question_variants(template_id);

-- question_deliveries
CREATE TABLE question_deliveries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    variant_id UUID NOT NULL REFERENCES question_variants(id) ON DELETE CASCADE,
    delivery_channel VARCHAR(32) NOT NULL CHECK (delivery_channel IN ('onboarding_form', 'ai_chat', 'profile_completion')),
    requirement_tier VARCHAR(32) NOT NULL CHECK (requirement_tier IN ('starter_required', 'profile_required', 'optional')),
    status VARCHAR(32) NOT NULL DEFAULT 'delivered' CHECK (status IN ('delivered', 'answered', 'skipped', 'expired')),
    delivered_at timestamptz NOT NULL DEFAULT now(),
    answered_at timestamptz
);

CREATE INDEX idx_question_deliveries_user_id ON question_deliveries(user_id);
CREATE INDEX idx_question_deliveries_variant_id ON question_deliveries(variant_id);

-- question_answers
CREATE TABLE question_answers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    variant_id UUID NOT NULL REFERENCES question_variants(id) ON DELETE CASCADE,
    delivery_id UUID NOT NULL REFERENCES question_deliveries(id) ON DELETE CASCADE,
    answer_payload JSONB NOT NULL,
    answer_state VARCHAR(32) NOT NULL CHECK (answer_state IN ('answered', 'skipped', 'decline')),
    answered_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_question_answers_delivery_id ON question_answers(delivery_id);
CREATE INDEX idx_question_answers_user_id ON question_answers(user_id);
```

### 3.7 匹配域（match-service）

```sql
-- find_requests
CREATE TABLE find_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    raw_query TEXT NOT NULL,
    parsed_goal VARCHAR(64),
    target_languages TEXT[],
    target_regions TEXT[],
    target_timezones TEXT[],
    allow_cross_language BOOLEAN,
    extra_constraints_json JSONB,
    status VARCHAR(32) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'need_clarification', 'blocked', 'matched')),
    risk_level VARCHAR(8),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_find_requests_user_id ON find_requests(user_id);
CREATE INDEX idx_find_requests_status ON find_requests(status);
CREATE INDEX idx_find_requests_created_at ON find_requests(created_at);

-- recommendation_result_sets
CREATE TABLE recommendation_result_sets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    find_request_id UUID NOT NULL REFERENCES find_requests(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    ranking_version VARCHAR(64),
    candidate_count INT NOT NULL,
    served_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_recommendation_result_sets_find_request ON recommendation_result_sets(find_request_id);

-- recommendation_cards
CREATE TABLE recommendation_cards (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    result_set_id UUID NOT NULL REFERENCES recommendation_result_sets(id) ON DELETE CASCADE,
    candidate_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    rank_position INT NOT NULL,
    score DECIMAL(10,6),
    reason_summary TEXT,
    served_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_recommendation_cards_result_set_id ON recommendation_cards(result_set_id);

-- recommendation_feedbacks（高写入，不加 card_id FK；必须带 source_event_id, source_event_name）
CREATE TABLE recommendation_feedbacks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    card_id UUID NOT NULL,
    actor_user_id UUID NOT NULL,
    source_event_id UUID NOT NULL,
    source_event_name VARCHAR(128) NOT NULL,
    feedback_type VARCHAR(32) NOT NULL CHECK (feedback_type IN ('impression', 'click', 'follow', 'dm_start', 'dm_reply', 'dismiss', 'block', 'report')),
    dismiss_reason VARCHAR(64),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_recommendation_feedbacks_card_id ON recommendation_feedbacks(card_id);
CREATE INDEX idx_recommendation_feedbacks_actor_user_id ON recommendation_feedbacks(actor_user_id);
CREATE INDEX idx_recommendation_feedbacks_created_at ON recommendation_feedbacks(created_at);
-- 注：card_id 不加重 FK，避免高写入锁竞争
```

### 3.8 安全治理域（safety-service）

```sql
-- risk_assessments
CREATE TABLE risk_assessments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    target_type VARCHAR(32) NOT NULL CHECK (target_type IN ('find_request', 'dm_message', 'profile')),
    target_id UUID NOT NULL,
    risk_level VARCHAR(8) NOT NULL,
    risk_codes TEXT[],
    rule_version VARCHAR(64),
    model_version VARCHAR(64),
    decision VARCHAR(32) NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_risk_assessments_target ON risk_assessments(target_type, target_id);

-- report_tickets
CREATE TABLE report_tickets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    reporter_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_type VARCHAR(32) NOT NULL,
    target_id UUID NOT NULL,
    reason_code VARCHAR(64) NOT NULL,
    evidence_json JSONB,
    status VARCHAR(32) NOT NULL DEFAULT 'open',
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_report_tickets_reporter ON report_tickets(reporter_user_id);
CREATE INDEX idx_report_tickets_target ON report_tickets(target_type, target_id);

-- moderation_actions
CREATE TABLE moderation_actions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    target_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    source_ticket_id UUID REFERENCES report_tickets(id) ON DELETE SET NULL,
    action_type VARCHAR(32) NOT NULL CHECK (action_type IN ('warn', 'throttle', 'mute', 'suspend', 'ban')),
    duration_seconds INT,
    reason_summary TEXT,
    executed_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_moderation_actions_target_user ON moderation_actions(target_user_id);

-- appeal_cases
CREATE TABLE appeal_cases (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    moderation_action_id UUID NOT NULL REFERENCES moderation_actions(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    appeal_text TEXT NOT NULL,
    status VARCHAR(32) NOT NULL DEFAULT 'pending',
    resolution_summary TEXT,
    resolved_at timestamptz
);

CREATE INDEX idx_appeal_cases_moderation_action ON appeal_cases(moderation_action_id);

-- user_blocks
CREATE TABLE user_blocks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    blocker_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    blocked_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    source VARCHAR(16) NOT NULL CHECK (source IN ('manual', 'moderation')),
    created_at timestamptz NOT NULL DEFAULT now(),
    released_at timestamptz
);

-- 部分唯一索引：同一对用户只能有一条活跃拉黑记录，解除后可重新拉黑
CREATE UNIQUE INDEX idx_user_blocks_active
    ON user_blocks(blocker_user_id, blocked_user_id)
    WHERE released_at IS NULL;
CREATE INDEX idx_user_blocks_blocker ON user_blocks(blocker_user_id);
CREATE INDEX idx_user_blocks_blocked ON user_blocks(blocked_user_id);
```

### 3.9 模型平台域（model-gateway）

```sql
-- model_invocation_logs
CREATE TABLE model_invocation_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    capability_name VARCHAR(64) NOT NULL,
    provider VARCHAR(64),
    model_id VARCHAR(128),
    prompt_version VARCHAR(64),
    request_region VARCHAR(64),
    latency_ms INT,
    cost_estimate DECIMAL(12,6),
    status VARCHAR(32) NOT NULL,
    trace_id UUID,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_model_invocation_logs_capability ON model_invocation_logs(capability_name);
CREATE INDEX idx_model_invocation_logs_created_at ON model_invocation_logs(created_at);
```

---

## 4. 分区与冷热分层建议

### 4.1 ai_messages（按 created_at 月分区）

```sql
-- 示例：需 PostgreSQL 11+ 原生分区
CREATE TABLE ai_messages (
    id UUID NOT NULL DEFAULT gen_random_uuid(),
    conversation_id UUID NOT NULL,
    sender_type VARCHAR(16) NOT NULL,
    content_type VARCHAR(32) NOT NULL DEFAULT 'text',
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

CREATE TABLE ai_messages_2026_03 PARTITION OF ai_messages
    FOR VALUES FROM ('2026-03-01') TO ('2026-04-01');
-- 后续按月创建分区
```

### 4.2 ai_message_contents（按 created_at 月分区）

同上，分区键 `created_at`。

### 4.3 recommendation_feedbacks

- MVP：单表，不加分区
- Phase 2：迁移到事件流/分析存储，PostgreSQL 只保留最近 N 天热数据

### 4.4 profile_fact_revisions

- MVP：单表
- Phase 2：按 `changed_at` 月分区或按 `fact_id` hash 分区

---

## 5. 外键策略汇总

| 表 | 外键策略 | 说明 |
|----|----------|------|
| ai_message_contents.message_id | 不加 FK | 高写入，应用层保证 |
| recommendation_feedbacks.card_id | 不加 FK | 高写入，应用层保证 |
| recommendation_feedbacks.actor_user_id | 不加 FK | 高写入，应用层保证 |
| 其他表 | 按需加 FK | 写入量可控 |

---

## 6. 预留 Phase 3

### trust_score_snapshots

```sql
-- Phase 3 随 trust-service 独立化后引入
-- CREATE TABLE trust_score_snapshots (
--     id UUID PRIMARY KEY,
--     user_id UUID NOT NULL REFERENCES users(id),
--     score DECIMAL(5,4),
--     score_version VARCHAR(64),
--     source_window VARCHAR(64),
--     updated_at timestamptz NOT NULL
-- );
```

---

## 7. 建表顺序

1. 依赖关系：`users` 最先
2. 然后：`identity_bindings`, `sessions`, `verification_attempts`, `profiles`, `discovery_preferences`
3. 再：`follows`, `profile_facts`, `profile_fact_revisions`, `profile_traits`, `trait_supporting_facts`, `profile_embeddings`, `profile_summaries`
4. 再：`memory_summaries`, `memory_artifacts`, `memory_entities`, `memory_entity_links`, `agent_runtime_checkpoints`, `forgetting_decisions`, `context_logs`
5. 再：`ai_conversations`, `ai_messages`, `ai_message_contents`
6. 再：`dm_threads`, `dm_participants`, `dm_messages`
7. 再：`question_templates`, `question_variants`, `question_deliveries`, `question_answers`
8. 再：`find_requests`, `recommendation_result_sets`, `recommendation_cards`, `recommendation_feedbacks`
9. 再：`risk_assessments`, `report_tickets`, `moderation_actions`, `appeal_cases`, `user_blocks`
10. 最后：`model_invocation_logs`
