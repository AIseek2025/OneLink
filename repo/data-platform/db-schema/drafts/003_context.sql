-- Draft from OneLink/Rules/14-MVP-SQL-SCHEMA-DRAFT.md (§3.3 记忆计算域 context-service), subject to review
-- Prerequisite: 001_identity.sql (users). Recommended after 002_profile.sql for local dev consistency.

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
