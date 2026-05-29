-- Draft schema for question-service (Phase C → Postgres persistence)
-- Prerequisite: 001_identity.sql

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE question_catalog (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    question_key VARCHAR(128) UNIQUE NOT NULL,
    question_text TEXT NOT NULL,
    category VARCHAR(64),
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_question_catalog_key ON question_catalog(question_key);
CREATE INDEX idx_question_catalog_active ON question_catalog(is_active) WHERE is_active = true;

CREATE TABLE question_deliveries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    question_id UUID NOT NULL REFERENCES question_catalog(id) ON DELETE CASCADE,
    status VARCHAR(32) NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'delivered', 'answered', 'skipped', 'expired')),
    delivered_at timestamptz NOT NULL DEFAULT now(),
    answered_at timestamptz
);

CREATE INDEX idx_question_deliveries_user_id ON question_deliveries(user_id);
CREATE INDEX idx_question_deliveries_question_id ON question_deliveries(question_id);
CREATE INDEX idx_question_deliveries_status ON question_deliveries(status);

CREATE TABLE question_answers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    delivery_id UUID NOT NULL REFERENCES question_deliveries(id) ON DELETE CASCADE,
    answer_text TEXT NOT NULL,
    answered_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_question_answers_delivery_id ON question_answers(delivery_id);
