-- Minimal idempotency tables for context-service (checkpoint + memory consolidate).
-- Apply immediately after 003_context.sql (same DB). Not optimization-layer.

CREATE TABLE IF NOT EXISTS context_checkpoint_dedupe (
    dedupe_key TEXT PRIMARY KEY,
    checkpoint_id UUID NOT NULL REFERENCES agent_runtime_checkpoints(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS context_memory_consolidate_dedupe (
    event_id TEXT PRIMARY KEY,
    summary_id UUID NOT NULL REFERENCES memory_summaries(id) ON DELETE CASCADE
);
