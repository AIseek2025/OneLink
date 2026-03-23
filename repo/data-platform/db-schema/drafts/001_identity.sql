-- Draft from Rules/14-MVP-SQL-SCHEMA-DRAFT.md (§3.1 identity-service), subject to review
-- Run first. Requires PostgreSQL with gen_random_uuid (e.g. CREATE EXTENSION IF NOT EXISTS pgcrypto).

CREATE EXTENSION IF NOT EXISTS pgcrypto;

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
