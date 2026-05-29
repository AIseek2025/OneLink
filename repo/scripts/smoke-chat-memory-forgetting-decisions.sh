#!/usr/bin/env bash
# 验证：Postgres 下 POST /internal/memory/forgetting/decide 写入 forgetting_decisions。
#
# 前置：001_identity（users）、003_context（含 forgetting_decisions）
#
# 退出码：0 成功；1 实现/断言失败；2 环境或 SQL 前置缺失
set -euo pipefail

die_env() { echo "[exit=2 环境/前置] $*" >&2; exit 2; }
die_impl() { echo "[exit=1 实现/断言] $*" >&2; exit 1; }

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CTX_PORT="${CONTEXT_PORT:-8089}"
if [[ -z "${DATABASE_URL:-}" ]]; then
  die_env "请设置 DATABASE_URL"
fi
INTERNAL_SHARED_SECRET="${INTERNAL_SHARED_SECRET:-onelink-dev-internal-token}"

wait_health() {
  local i=0
  while ! curl -sf "http://127.0.0.1:${CTX_PORT}/health" >/dev/null; do
    i=$((i + 1))
    if [[ "$i" -gt 60 ]]; then
      die_impl "timeout: context-service 未在 ${CTX_PORT} 响应 /health"
    fi
    sleep 0.25
  done
}

start_ctx() {
  cd "$REPO_ROOT"
  export DATABASE_URL INTERNAL_SHARED_SECRET
  PORT="$CTX_PORT" "$CTX_BIN" &
  echo $!
}

echo "== build context-service =="
(cd "$REPO_ROOT" && cargo build -q -p context-service)
CTX_BIN="$REPO_ROOT/target/debug/context-service"

if ! psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -tc \
  "SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_name='forgetting_decisions'" \
  | grep -q '[[:space:]]*1[[:space:]]*$'; then
  die_env "表 forgetting_decisions 不存在：请执行 003_context.sql"
fi

USER_ID="$(uuidgen | tr '[:upper:]' '[:lower:]')"
TARGET_ID="$(uuidgen | tr '[:upper:]' '[:lower:]')"
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -c \
  "INSERT INTO users (id) VALUES ('$USER_ID'::uuid) ON CONFLICT (id) DO NOTHING"

COUNT_BEFORE="$(psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -Atc \
  "SELECT count(*)::text FROM forgetting_decisions WHERE user_id = '$USER_ID'::uuid")"

PID="$(start_ctx)"
trap 'kill "$PID" 2>/dev/null || true; wait "$PID" 2>/dev/null || true' EXIT
wait_health

BODY_TMP="$(mktemp)"
HTTP_CODE="$(curl -sS -o "$BODY_TMP" -w "%{http_code}" -X POST \
  "http://127.0.0.1:${CTX_PORT}/internal/memory/forgetting/decide" \
  -H "Content-Type: application/json" \
  -H "x-internal-token: ${INTERNAL_SHARED_SECRET}" \
  -d "{\"user_id\":\"$USER_ID\",\"target_type\":\"memory_artifact\",\"target_id\":\"$TARGET_ID\",\"decision\":\"retain\",\"reason_codes\":[\"smoke_test\"],\"policy_version\":\"v1\",\"cold_storage_ref\":\"s3://cold/smoke\"}")"
jq . <"$BODY_TMP" 2>/dev/null || cat "$BODY_TMP"
rm -f "$BODY_TMP"
if [[ "$HTTP_CODE" != "200" ]]; then
  die_impl "forgetting/decide 非 200: $HTTP_CODE"
fi

COUNT_AFTER="$(psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -Atc \
  "SELECT count(*)::text FROM forgetting_decisions WHERE user_id = '$USER_ID'::uuid")"
if [[ "$COUNT_AFTER" -le "$COUNT_BEFORE" ]]; then
  die_impl "forgetting_decisions 未新增: before=$COUNT_BEFORE after=$COUNT_AFTER"
fi

echo "== OK: forgetting_decisions 已写入 (before=$COUNT_BEFORE after=$COUNT_AFTER) =="
