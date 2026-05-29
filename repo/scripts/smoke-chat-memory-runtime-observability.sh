#!/usr/bin/env bash
# 验证：Postgres + 011 下 routing 写入 DB，且 asmr-lite 跨进程重启仍能读到 last_observation / failure 计数视角。
#
# 前置：001_identity（users）、003_context、011_runtime_observability.sql
# 依赖：curl、jq、psql
#
# 退出码：0 成功；1 实现/断言失败（HTTP 非 200、DB 行数未增等）；2 环境或 SQL 前置缺失（缺 DATABASE_URL、缺表等）
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

for tbl in context_routing_observations context_failure_events; do
  if ! psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -tc \
    "SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_name='$tbl'" \
    | grep -q '[[:space:]]*1[[:space:]]*$'; then
    die_env "表 $tbl 不存在：请执行 repo/data-platform/db-schema/drafts/011_runtime_observability.sql"
  fi
done

USER_ID="$(uuidgen | tr '[:upper:]' '[:lower:]')"
CONV_ID="smoke-runtime-obs-$(date +%s)"
echo "== upsert users (FK) =="
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -c \
  "INSERT INTO users (id) VALUES ('$USER_ID'::uuid) ON CONFLICT (id) DO NOTHING"

R_BEFORE="$(psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -Atc \
  "SELECT count(*)::text FROM context_routing_observations WHERE user_id = '$USER_ID'::uuid")"

echo "== start context-service (pass 1) =="
PID="$(start_ctx)"
trap 'kill "$PID" 2>/dev/null || true; wait "$PID" 2>/dev/null || true' EXIT
wait_health

BODY_TMP="$(mktemp)"
trap 'rm -f "$BODY_TMP"; kill "$PID" 2>/dev/null || true; wait "$PID" 2>/dev/null || true' EXIT
HTTP_CODE="$(curl -sS -o "$BODY_TMP" -w "%{http_code}" -X POST "http://127.0.0.1:${CTX_PORT}/internal/context/build" \
  -H "Content-Type: application/json" \
  -H "x-internal-token: ${INTERNAL_SHARED_SECRET}" \
  -d "{\"user_id\":\"$USER_ID\",\"agent_id\":\"00000000-0000-0000-0000-0000000000bb\",\"conversation_id\":\"$CONV_ID\",\"input\":\"runtime obs smoke $(date +%s)\",\"task_type\":\"chat\",\"max_tokens\":8000,\"memory_limit\":6,\"summary_limit\":3,\"reply_style\":\"brief\",\"trace_id\":\"smoke-rt-obs-$RANDOM\",\"retrieval_modes\":[\"structured\"]}")"
rm -f "$BODY_TMP"
if [[ "$HTTP_CODE" != "200" ]]; then
  die_impl "build 非 200: $HTTP_CODE"
fi

R_AFTER="$(psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -Atc \
  "SELECT count(*)::text FROM context_routing_observations WHERE user_id = '$USER_ID'::uuid")"
if [[ "$R_AFTER" -le "$R_BEFORE" ]]; then
  die_impl "context_routing_observations 未新增: before=$R_BEFORE after=$R_AFTER"
fi

OBS1="$(mktemp)"
curl -sS "http://127.0.0.1:${CTX_PORT}/internal/observability/asmr-lite" \
  -H "x-internal-token: ${INTERNAL_SHARED_SECRET}" >"$OBS1"
PREVIEW1="$(jq -r '.routing.last_observation.query_preview // empty' "$OBS1")"
if [[ -z "$PREVIEW1" ]]; then
  cat "$OBS1" >&2
  rm -f "$OBS1"
  die_impl "pass1 asmr-lite 缺少 last_observation.query_preview"
fi
rm -f "$OBS1"

echo "== restart context-service (pass 2) =="
kill "$PID" 2>/dev/null || true
wait "$PID" 2>/dev/null || true
trap - EXIT

PID="$(start_ctx)"
trap 'kill "$PID" 2>/dev/null || true; wait "$PID" 2>/dev/null || true' EXIT
wait_health

OBS2="$(mktemp)"
curl -sS "http://127.0.0.1:${CTX_PORT}/internal/observability/asmr-lite" \
  -H "x-internal-token: ${INTERNAL_SHARED_SECRET}" >"$OBS2"
PREVIEW2="$(jq -r '.routing.last_observation.query_preview // empty' "$OBS2")"
rm -f "$OBS2"

if [[ "$PREVIEW1" != "$PREVIEW2" ]]; then
  die_impl "跨重启 last_observation 不一致: '$PREVIEW1' vs '$PREVIEW2'"
fi

echo "== OK: routing 已持久化且 asmr-lite 跨重启可读 =="
