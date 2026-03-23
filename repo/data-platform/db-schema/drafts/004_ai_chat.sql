-- Draft from OneLink/Rules/14-MVP-SQL-SCHEMA-DRAFT.md (§3.4 AI 对话域 ai-chat-service), subject to review
-- Prerequisite: 001_identity.sql (optional: 003_context.sql for aligned local ordering)

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

-- ai_messages
CREATE TABLE ai_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID NOT NULL REFERENCES ai_conversations(id) ON DELETE CASCADE,
    sender_type VARCHAR(16) NOT NULL CHECK (sender_type IN ('user', 'assistant', 'system')),
    content_type VARCHAR(32) NOT NULL DEFAULT 'text',
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_ai_messages_conversation_id ON ai_messages(conversation_id);
CREATE INDEX idx_ai_messages_created_at ON ai_messages(created_at);

-- ai_message_contents
CREATE TABLE ai_message_contents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_id UUID NOT NULL,
    content_text TEXT,
    content_metadata JSONB,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_ai_message_contents_message_id ON ai_message_contents(message_id);
CREATE INDEX idx_ai_message_contents_created_at ON ai_message_contents(created_at);
-- message_id 不加重 FK，避免高写入锁竞争；应用层保证一致性
