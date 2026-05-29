-- optimization-layer / Policy Config Store（占位草案）
-- 来源：OneLink/docs/archive/rules-legacy-2026-05-15/Rules/14-MVP-SQL-SCHEMA-DRAFT.md §3.3A
-- 建表顺序：policy_configs → policy_experiments → policy_rollouts（rollout 外键引用 experiments）

-- policy_configs
CREATE TABLE policy_configs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    policy_key VARCHAR(128) NOT NULL,
    policy_domain VARCHAR(64) NOT NULL,
    value_type VARCHAR(32) NOT NULL,
    default_value TEXT,
    allowed_range_json JSONB,
    current_value TEXT,
    status VARCHAR(32) NOT NULL DEFAULT 'active',
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_policy_configs_key ON policy_configs(policy_key);

-- policy_experiments（预埋；MVP 可不写入）
CREATE TABLE policy_experiments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    experiment_name VARCHAR(256) NOT NULL,
    policy_domain VARCHAR(64) NOT NULL,
    hypothesis TEXT,
    dataset_version VARCHAR(64),
    candidate_values_json JSONB,
    status VARCHAR(32) NOT NULL DEFAULT 'draft',
    created_at timestamptz NOT NULL DEFAULT now(),
    completed_at timestamptz
);

-- policy_rollouts（预埋；MVP 可不写入）
CREATE TABLE policy_rollouts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    experiment_id UUID REFERENCES policy_experiments(id) ON DELETE SET NULL,
    rollout_stage VARCHAR(32) NOT NULL CHECK (rollout_stage IN ('replay', 'shadow', 'canary', 'full', 'rollback')),
    target_scope JSONB,
    result_metrics_json JSONB,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_policy_rollouts_experiment_id ON policy_rollouts(experiment_id);
