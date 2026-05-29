-- Draft from Rules/14-MVP-SQL-SCHEMA-DRAFT.md (§3.7 safety-service), subject to review
-- Prerequisite: 001_identity.sql

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

CREATE UNIQUE INDEX idx_user_blocks_active
    ON user_blocks(blocker_user_id, blocked_user_id)
    WHERE released_at IS NULL;
CREATE INDEX idx_user_blocks_blocker ON user_blocks(blocker_user_id);
CREATE INDEX idx_user_blocks_blocked ON user_blocks(blocked_user_id);
