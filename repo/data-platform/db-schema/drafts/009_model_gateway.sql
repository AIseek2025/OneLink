-- Draft from Rules/14-MVP-SQL-SCHEMA-DRAFT.md (§3.8 model-gateway), subject to review
-- No FK to users required for MVP logging table

-- model_invocation_logs
CREATE TABLE model_invocation_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    capability_name VARCHAR(64) NOT NULL,
    provider VARCHAR(64),
    model_id VARCHAR(128),
    prompt_version VARCHAR(64),
    request_region VARCHAR(64),
    latency_ms INT,
    cost_estimate DECIMAL(12,6),
    status VARCHAR(32) NOT NULL,
    trace_id UUID,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX idx_model_invocation_logs_capability ON model_invocation_logs(capability_name);
CREATE INDEX idx_model_invocation_logs_created_at ON model_invocation_logs(created_at);
