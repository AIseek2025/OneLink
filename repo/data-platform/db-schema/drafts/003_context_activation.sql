-- Phase 1 / Activation Scoring extension for 003_context.sql
-- Source: OneLink Elo-like Memory Phase 1 / Activation Scoring
-- Purpose: activate memory_artifacts dynamic state without changing owner boundaries

ALTER TABLE memory_artifacts
    ADD COLUMN IF NOT EXISTS last_accessed_at timestamptz;

ALTER TABLE memory_artifacts
    ADD COLUMN IF NOT EXISTS access_count INT NOT NULL DEFAULT 0;

ALTER TABLE memory_artifacts
    ALTER COLUMN importance_score SET DEFAULT 0.5;

UPDATE memory_artifacts
SET last_accessed_at = COALESCE(last_accessed_at, updated_at, created_at, now())
WHERE last_accessed_at IS NULL;

ALTER TABLE memory_artifacts
    ALTER COLUMN last_accessed_at SET DEFAULT now();

-- 与任务书口径一致：回填后收紧为 NOT NULL（新行仍由 DEFAULT now() 填充）
ALTER TABLE memory_artifacts
    ALTER COLUMN last_accessed_at SET NOT NULL;

CREATE INDEX IF NOT EXISTS idx_memory_artifacts_activation
    ON memory_artifacts(user_id, importance_score DESC, last_accessed_at DESC);
