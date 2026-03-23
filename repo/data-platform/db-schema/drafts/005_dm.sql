-- Draft from Rules/14-MVP-SQL-SCHEMA-DRAFT.md (§3.4 dm-service), subject to review
-- Prerequisite: 001_identity.sql

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
