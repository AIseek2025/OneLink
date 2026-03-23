-- Draft from Rules/14-MVP-SQL-SCHEMA-DRAFT.md (§3.5 question-service), subject to review
-- Prerequisite: 001_identity.sql

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
