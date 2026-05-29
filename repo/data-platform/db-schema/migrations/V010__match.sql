-- Draft from Rules/14-MVP-SQL-SCHEMA-DRAFT.md (§3.6 match-service), subject to review
-- Prerequisite: 001_identity.sql

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

-- recommendation_feedbacks
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
