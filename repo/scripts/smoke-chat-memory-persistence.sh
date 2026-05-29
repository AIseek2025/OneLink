#!/usr/bin/env bash
# 跨重启验证：context-service 在 DATABASE_URL 存在时持久化 memory / checkpoint。
#
# 前置条件：
#   - PostgreSQL 已按序应用：001_identity.sql、002_profile.sql（可选）、003_context.sql、003_context_idempotency.sql
#   - 环境变量 DATABASE_URL 指向该库（postgres://...）
#   - 依赖：curl、jq、psql（若本机缺失则回退到 Docker Postgres 容器内 psql）
#
# 本脚本会：在本机编译并两次启动 target/debug/context-service（同一端口），中间 kill 进程，
# 用 internal API 写入一条 memory 后校验 observability 计数在重启后不归零。
#
# 可选：INTERNAL_SHARED_SECRET（默认 onelink-dev-internal-token）
set -euo pipefail

export PATH="$HOME/.cargo/bin:$PATH"

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CTX_PORT="${CONTEXT_PORT:-8089}"
DATABASE_URL="${DATABASE_URL:?请设置 DATABASE_URL（并确保已执行 003_context.sql 与 003_context_idempotency.sql）}"
INTERNAL_SHARED_SECRET="${INTERNAL_SHARED_SECRET:-onelink-dev-internal-token}"

run_psql() {
  if command -v psql >/dev/null 2>&1; then
    psql "$DATABASE_URL" "$@"
  else
    docker exec onelink-postgres psql -U onelink -d onelink "$@"
  fi
}

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

echo "== build context-service =="
(cd "$REPO_ROOT" && cargo build -q -p context-service)
CTX_BIN="$REPO_ROOT/target/debug/context-service"
if [[ ! -x "$CTX_BIN" ]]; then
  echo "missing binary: $CTX_BIN" >&2
  exit 1
fi

echo "== DB sanity (memory_artifacts + dedupe tables) =="
if ! run_psql -v ON_ERROR_STOP=1 -tc \
  "SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_name='memory_artifacts'" \
  | grep -q '[[:space:]]*1[[:space:]]*$'; then
  echo "表 memory_artifacts 不存在：请先对 DATABASE_URL 执行 repo/data-platform/db-schema/drafts/003_context.sql" >&2
  exit 1
fi
if ! run_psql -v ON_ERROR_STOP=1 -tc \
  "SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_name='context_checkpoint_dedupe'" \
  | grep -q '[[:space:]]*1[[:space:]]*$'; then
  echo "表 context_checkpoint_dedupe 不存在：请执行 repo/data-platform/db-schema/drafts/003_context_idempotency.sql" >&2
  exit 1
fi

USER_ID="$(uuidgen | tr '[:upper:]' '[:lower:]')"
echo "== upsert users row for FK (user_id=$USER_ID) =="
run_psql -v ON_ERROR_STOP=1 -c \
  "INSERT INTO users (id) VALUES ('$USER_ID'::uuid) ON CONFLICT (id) DO NOTHING"

echo "== start context-service (1st) =="
PORT="$CTX_PORT" DATABASE_URL="$DATABASE_URL" INTERNAL_SHARED_SECRET="$INTERNAL_SHARED_SECRET" "$CTX_BIN" &
PID1=$!
trap 'kill "$PID1" 2>/dev/null || true; wait "$PID1" 2>/dev/null || true' EXIT
wait_health

echo "== POST /internal/memory/write =="
curl -sS -X POST "http://127.0.0.1:${CTX_PORT}/internal/memory/write" \
  -H "Content-Type: application/json" \
  -H "x-internal-token: ${INTERNAL_SHARED_SECRET}" \
  -d "{\"user_id\":\"$USER_ID\",\"raw_text\":\"persistence smoke 记忆样本 $(date +%s)\",\"source_type\":\"direct_internal\"}" | jq .

echo "== POST /internal/session/checkpoint =="
curl -sS -X POST "http://127.0.0.1:${CTX_PORT}/internal/session/checkpoint" \
  -H "Content-Type: application/json" \
  -H "x-internal-token: ${INTERNAL_SHARED_SECRET}" \
  -d "{\"agent_id\":\"00000000-0000-0000-0000-000000000099\",\"user_id\":\"$USER_ID\",\"conversation_id\":null,\"schema_version\":1,\"working_summary_ref\":null,\"runtime_state_blob\":{},\"policy_versions\":{}}" | jq .

OBS1="$(curl -sS "http://127.0.0.1:${CTX_PORT}/internal/observability/asmr-lite" \
  -H "x-internal-token: ${INTERNAL_SHARED_SECRET}")"
echo "$OBS1" | jq .
A1="$(echo "$OBS1" | jq -r '.artifact_count')"
C1="$(echo "$OBS1" | jq -r '.checkpoint_count')"
PV1="$(echo "$OBS1" | jq -r '.policy_version_label // empty')"

if [[ "$A1" =~ ^[0-9]+$ ]] && [[ "$A1" -lt 1 ]]; then
  echo "第一次观测 artifact_count 应 >= 1，实际=$A1" >&2
  exit 1
fi
if [[ "$C1" =~ ^[0-9]+$ ]] && [[ "$C1" -lt 1 ]]; then
  echo "第一次观测 checkpoint_count 应 >= 1，实际=$C1" >&2
  exit 1
fi
if [[ -z "$PV1" ]]; then
  echo "policy_version_label 不应为空（与 ASMR-Lite 对账字段一致）" >&2
  exit 1
fi

echo "== restart context-service (kill pid=$PID1) =="
kill "$PID1" || true
wait "$PID1" 2>/dev/null || true
trap - EXIT
sleep 1

echo "== start context-service (2nd) =="
PORT="$CTX_PORT" DATABASE_URL="$DATABASE_URL" INTERNAL_SHARED_SECRET="$INTERNAL_SHARED_SECRET" "$CTX_BIN" &
PID2=$!
trap 'kill "$PID2" 2>/dev/null || true; wait "$PID2" 2>/dev/null || true' EXIT
wait_health

OBS2="$(curl -sS "http://127.0.0.1:${CTX_PORT}/internal/observability/asmr-lite" \
  -H "x-internal-token: ${INTERNAL_SHARED_SECRET}")"
echo "$OBS2" | jq .
A2="$(echo "$OBS2" | jq -r '.artifact_count')"
C2="$(echo "$OBS2" | jq -r '.checkpoint_count')"
PV2="$(echo "$OBS2" | jq -r '.policy_version_label // empty')"

if [[ "$A2" =~ ^[0-9]+$ ]] && [[ "$A2" -lt "$A1" ]]; then
  echo "重启后 artifact_count 不应下降 ($A2 < $A1)" >&2
  exit 1
fi
if [[ "$C2" =~ ^[0-9]+$ ]] && [[ "$C2" -lt "$C1" ]]; then
  echo "重启后 checkpoint_count 不应下降 ($C2 < $C1)" >&2
  exit 1
fi
if [[ "$PV2" != "$PV1" ]]; then
  echo "policy_version_label 重启前后应一致: '$PV1' vs '$PV2'" >&2
  exit 1
fi

echo "OK persistence smoke: artifact_count $A1 -> $A2, checkpoint_count $C1 -> $C2, policy_version_label ok"
kill "$PID2" || true
wait "$PID2" 2>/dev/null || true
trap - EXIT
