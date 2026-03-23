-- Draft from Rules/14-MVP-SQL-SCHEMA-DRAFT.md (§3.2 profile-service), subject to review
-- Prerequisite: 001_identity.sql (users)

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

-- profile_fact_revisions
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
