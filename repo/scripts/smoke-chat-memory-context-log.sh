#!/usr/bin/env bash
# 验证：Postgres 模式下每次 POST /internal/context/build 会写入一行 context_logs。
#
# 前置条件：
#   - PostgreSQL 已应用 001_identity.sql（users）、003_context.sql（含 context_logs）
#   - DATABASE_URL 指向该库
#   - 依赖：curl、jq、psql
#
# 可选：INTERNAL_SHARED_SECRET（默认 onelink-dev-internal-token）
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CTX_PORT="${CONTEXT_PORT:-8089}"
DATABASE_URL="${DATABASE_URL:?请设置 DATABASE_URL（并确保已执行 003_context.sql，含 context_logs）}"
INTERNAL_SHARED_SECRET="${INTERNAL_SHARED_SECRET:-onelink-dev-internal-token}"

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

echo "== build context-service =="
(cd "$REPO_ROOT" && cargo build -q -p context-service)
CTX_BIN="$REPO_ROOT/target/debug/context-service"
if [[ ! -x "$CTX_BIN" ]]; then
  echo "missing binary: $CTX_BIN" >&2
  exit 1
fi

echo "== DB sanity (context_logs) =="
if ! psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -tc \
  "SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_name='context_logs'" \
  | grep -q '[[:space:]]*1[[:space:]]*$'; then
  echo "表 context_logs 不存在：请先对 DATABASE_URL 执行 repo/data-platform/db-schema/drafts/003_context.sql" >&2
  exit 1
fi

USER_ID="$(uuidgen | tr '[:upper:]' '[:lower:]')"
CONV_ID="smoke-context-log-$(date +%s)"
echo "== upsert users row for FK (user_id=$USER_ID) =="
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -c \
  "INSERT INTO users (id) VALUES ('$USER_ID'::uuid) ON CONFLICT (id) DO NOTHING"

COUNT_BEFORE="$(psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -Atc \
  "SELECT count(*)::text FROM context_logs WHERE user_id = '$USER_ID'::uuid")"

echo "== start context-service =="
PID="$(start_ctx)"
trap 'kill "$PID" 2>/dev/null || true; wait "$PID" 2>/dev/null || true' EXIT
wait_health

echo "== POST /internal/context/build =="
BODY_TMP="$(mktemp)"
trap 'rm -f "$BODY_TMP"; kill "$PID" 2>/dev/null || true; wait "$PID" 2>/dev/null || true' EXIT
HTTP_CODE="$(curl -sS -o "$BODY_TMP" -w "%{http_code}" -X POST "http://127.0.0.1:${CTX_PORT}/internal/context/build" \
  -H "Content-Type: application/json" \
  -H "x-internal-token: ${INTERNAL_SHARED_SECRET}" \
  -d "{\"user_id\":\"$USER_ID\",\"agent_id\":\"00000000-0000-0000-0000-0000000000aa\",\"conversation_id\":\"$CONV_ID\",\"input\":\"context log smoke query $(date +%s)\",\"task_type\":\"chat\",\"max_tokens\":8000,\"memory_limit\":6,\"summary_limit\":3,\"reply_style\":\"brief\",\"trace_id\":\"smoke-context-log-$RANDOM\",\"retrieval_modes\":[\"structured\",\"temporal\"]}")"
jq . <"$BODY_TMP" 2>/dev/null || cat "$BODY_TMP"
rm -f "$BODY_TMP"
trap 'kill "$PID" 2>/dev/null || true; wait "$PID" 2>/dev/null || true' EXIT
if [[ "$HTTP_CODE" != "200" ]]; then
  echo "build 返回非 200: $HTTP_CODE（若为 5xx，请检查服务日志与 DB 连接）" >&2
  exit 1
fi

COUNT_AFTER="$(psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -Atc \
  "SELECT count(*)::text FROM context_logs WHERE user_id = '$USER_ID'::uuid")"

if ! [[ "$COUNT_BEFORE" =~ ^[0-9]+$ ]] || ! [[ "$COUNT_AFTER" =~ ^[0-9]+$ ]]; then
  echo "无法解析 context_logs 计数（环境/psql 问题） before=$COUNT_BEFORE after=$COUNT_AFTER" >&2
  exit 1
fi

if [[ "$COUNT_AFTER" -le "$COUNT_BEFORE" ]]; then
  echo "context_logs 未新增行：before=$COUNT_BEFORE after=$COUNT_AFTER（若为 FK/约束错误，请确认 users 行与 DDL）" >&2
  exit 1
fi

echo "== OK: context_logs 已新增 (before=$COUNT_BEFORE after=$COUNT_AFTER) =="
