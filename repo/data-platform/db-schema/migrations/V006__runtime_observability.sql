-- context-service runtime observability append-only (routing / failure)
-- 应用顺序：在 003_context.sql 之后（依赖 users）；可与 003_context_idempotency 同批次。
-- 用于 Postgres 模式下 asmr-lite 跨重启读取 last_observation / recent_failures / total_failures。

-- Routing observation 一行一次 context build（append-only）
CREATE TABLE context_routing_observations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    executed_route VARCHAR(16) NOT NULL,
    candidate_route VARCHAR(16) NOT NULL,
    escalation_reasons TEXT[] NOT NULL DEFAULT '{}',
    upgraded BOOLEAN NOT NULL,
    evidence_count INT NOT NULL,
    summary_hits INT NOT NULL,
    artifact_hits INT NOT NULL,
    entity_hits INT NOT NULL,
    conflict_count INT NOT NULL,
    route_confidence DOUBLE PRECISION NOT NULL,
    estimated_llm_calls INT NOT NULL,
    estimated_tokens INT NOT NULL,
    query_preview TEXT NOT NULL,
    degraded BOOLEAN NOT NULL,
    elapsed_ms BIGINT NOT NULL,
    query_preference_polarity VARCHAR(32) NOT NULL,
    evidence_preference_polarity VARCHAR(32) NOT NULL,
    retrieval_modes TEXT[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_context_routing_obs_user_created ON context_routing_observations (user_id, created_at DESC);
CREATE INDEX idx_context_routing_obs_created ON context_routing_observations (created_at DESC);

-- Failure 事件（append-only）
CREATE TABLE context_failure_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    stage VARCHAR(128) NOT NULL,
    category VARCHAR(128) NOT NULL,
    detail TEXT NOT NULL,
    trace_id TEXT,
    retryable BOOLEAN NOT NULL,
    attempt_count INT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_context_failure_events_user_created ON context_failure_events (user_id, created_at DESC);
CREATE INDEX idx_context_failure_events_created ON context_failure_events (created_at DESC);
