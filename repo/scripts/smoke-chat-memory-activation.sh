#!/usr/bin/env bash
# 验证：Phase 1 activation scoring 已真实接通。
# - memory/write 会写入 importance_score
# - context/build 命中后会 best-effort 增加 access_count / last_accessed_at
#
# 前置条件：
#   - PostgreSQL 已应用 001_identity.sql、003_context.sql、003_context_activation.sql
#   - DATABASE_URL 指向该库
#   - 依赖：curl、jq、psql
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CTX_PORT="${CONTEXT_PORT:-8089}"
DATABASE_URL="${DATABASE_URL:-}"
INTERNAL_SHARED_SECRET="${INTERNAL_SHARED_SECRET:-onelink-dev-internal-token}"

if [[ -z "$DATABASE_URL" ]]; then
  echo "缺少 DATABASE_URL：请先设置并确保已执行 003_context.sql + 003_context_activation.sql" >&2
  exit 2
fi

wait_health() {
  local i=0
  while ! curl -sf "http://127.0.0.1:${CTX_PORT}/health" >/dev/null; do
    i=$((i + 1))
    if [[ "$i" -gt 60 ]]; then
      echo "timeout: context-service 未在 ${CTX_PORT} 响应 /health" >&2
      return 1
    fi
    sleep 0.25
  done
}

start_ctx() {
  cd "$REPO_ROOT"
  export DATABASE_URL
  export INTERNAL_SHARED_SECRET
  PORT="$CTX_PORT" "$CTX_BIN" &
  echo $!
}

require_sql() {
  local sql="$1"
  local err="$2"
  if ! psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -tc "$sql" | grep -q '[[:space:]]*1[[:space:]]*$'; then
    echo "$err" >&2
    exit 2
  fi
}

echo "== build context-service =="
(cd "$REPO_ROOT" && cargo build -q -p context-service)
CTX_BIN="$REPO_ROOT/target/debug/context-service"
if [[ ! -x "$CTX_BIN" ]]; then
  echo "missing binary: $CTX_BIN" >&2
  exit 1
fi

echo "== DB sanity (activation columns) =="
require_sql \
  "SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_name='memory_artifacts'" \
  "表 memory_artifacts 不存在：请先执行 repo/data-platform/db-schema/drafts/003_context.sql"
require_sql \
  "SELECT 1 FROM information_schema.columns WHERE table_schema='public' AND table_name='memory_artifacts' AND column_name='last_accessed_at'" \
  "列 memory_artifacts.last_accessed_at 不存在：请执行 repo/data-platform/db-schema/drafts/003_context_activation.sql"
require_sql \
  "SELECT 1 FROM information_schema.columns WHERE table_schema='public' AND table_name='memory_artifacts' AND column_name='access_count'" \
  "列 memory_artifacts.access_count 不存在：请执行 repo/data-platform/db-schema/drafts/003_context_activation.sql"

USER_ID="$(uuidgen | tr '[:upper:]' '[:lower:]')"
TRACE_ID="smoke-activation-$(date +%s)"
QUERY_TOKEN="activation-smoke-$(date +%s)"
CONV_ID="$(uuidgen | tr '[:upper:]' '[:lower:]')"
echo "== upsert users row for FK (user_id=$USER_ID) =="
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -c \
  "INSERT INTO users (id) VALUES ('$USER_ID'::uuid) ON CONFLICT (id) DO NOTHING"

echo "== start context-service =="
PID="$(start_ctx)"
trap 'kill "$PID" 2>/dev/null || true; wait "$PID" 2>/dev/null || true' EXIT
wait_health

echo "== POST /internal/memory/write =="
WRITE_BODY="$(mktemp)"
trap 'rm -f "$WRITE_BODY"; kill "$PID" 2>/dev/null || true; wait "$PID" 2>/dev/null || true' EXIT
WRITE_CODE="$(curl -sS -o "$WRITE_BODY" -w "%{http_code}" -X POST "http://127.0.0.1:${CTX_PORT}/internal/memory/write" \
  -H "Content-Type: application/json" \
  -H "x-internal-token: ${INTERNAL_SHARED_SECRET}" \
  -d "{\"event_id\":\"$TRACE_ID\",\"user_id\":\"$USER_ID\",\"source_type\":\"chat\",\"source_ref_id\":\"$CONV_ID\",\"raw_text\":\"用户提到 ${QUERY_TOKEN} 并且持续关注 AI 创业\",\"memory_value_score\":0.85}")"
jq . <"$WRITE_BODY" 2>/dev/null || cat "$WRITE_BODY"
if [[ "$WRITE_CODE" != "200" ]]; then
  echo "memory/write 返回非 200: $WRITE_CODE" >&2
  exit 1
fi
rm -f "$WRITE_BODY"

ARTIFACT_ROW="$(psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -Atc \
  "SELECT id::text || '|' || COALESCE(importance_score::text, '') || '|' || access_count::text FROM memory_artifacts WHERE user_id = '$USER_ID'::uuid ORDER BY created_at DESC LIMIT 1")"
ARTIFACT_ID="${ARTIFACT_ROW%%|*}"
REST="${ARTIFACT_ROW#*|}"
IMPORTANCE="${REST%%|*}"
ACCESS_BEFORE="${ARTIFACT_ROW##*|}"

if [[ -z "$ARTIFACT_ID" ]]; then
  echo "memory/write 后未查到 artifact 行" >&2
  exit 1
fi
if [[ -z "$IMPORTANCE" ]]; then
  echo "importance_score 为空，activation Phase 1 未正确写入" >&2
  exit 1
fi

echo "== POST /internal/context/build =="
BUILD_BODY="$(mktemp)"
trap 'rm -f "$BUILD_BODY"; kill "$PID" 2>/dev/null || true; wait "$PID" 2>/dev/null || true' EXIT
BUILD_CODE="$(curl -sS -o "$BUILD_BODY" -w "%{http_code}" -X POST "http://127.0.0.1:${CTX_PORT}/internal/context/build" \
  -H "Content-Type: application/json" \
  -H "x-internal-token: ${INTERNAL_SHARED_SECRET}" \
  -d "{\"user_id\":\"$USER_ID\",\"agent_id\":\"00000000-0000-0000-0000-0000000000aa\",\"conversation_id\":\"$CONV_ID\",\"input\":\"${QUERY_TOKEN}\",\"task_type\":\"chat\",\"max_tokens\":8000,\"memory_limit\":6,\"summary_limit\":3,\"reply_style\":\"brief\",\"trace_id\":\"${TRACE_ID}-build\",\"retrieval_modes\":[\"structured\"]}")"
jq . <"$BUILD_BODY" 2>/dev/null || cat "$BUILD_BODY"
if [[ "$BUILD_CODE" != "200" ]]; then
  echo "context/build 返回非 200: $BUILD_CODE" >&2
  exit 1
fi

if ! jq -e --arg id "$ARTIFACT_ID" '.selected_memory_ids | index($id)' <"$BUILD_BODY" >/dev/null 2>&1; then
  echo "context/build 未命中刚写入的 artifact：无法验证 activation touch" >&2
  exit 1
fi
rm -f "$BUILD_BODY"

ACTIVATION_ROW="$(psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -Atc \
  "SELECT access_count::text || '|' || COALESCE(last_accessed_at::text, '') FROM memory_artifacts WHERE id = '$ARTIFACT_ID'::uuid")"
ACCESS_AFTER="${ACTIVATION_ROW%%|*}"
LAST_ACCESSED="${ACTIVATION_ROW#*|}"

if ! [[ "$ACCESS_BEFORE" =~ ^[0-9]+$ ]] || ! [[ "$ACCESS_AFTER" =~ ^[0-9]+$ ]]; then
  echo "access_count 解析失败：before=$ACCESS_BEFORE after=$ACCESS_AFTER" >&2
  exit 1
fi
if [[ "$ACCESS_AFTER" -le "$ACCESS_BEFORE" ]]; then
  echo "access_count 未增长：before=$ACCESS_BEFORE after=$ACCESS_AFTER" >&2
  exit 1
fi
if [[ -z "$LAST_ACCESSED" ]]; then
  echo "last_accessed_at 为空：activation touch 未生效" >&2
  exit 1
fi

echo "== OK: activation smoke 通过 (importance_score=$IMPORTANCE access_count $ACCESS_BEFORE -> $ACCESS_AFTER) =="
