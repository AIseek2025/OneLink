#!/usr/bin/env bash
# ASMR-Lite benchmark v2.2（门禁）：Postgres + policy_configs 下 graph/rerank 进入 retrieval_used，
# 且 build 响应可解析（不替代 v2 / v2.1）。
#
# 前置：DATABASE_URL；001_identity；003_context；010_optimization.sql（policy_configs）
# 可选：011（与本脚本断言无硬依赖）
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
      die_impl "timeout waiting for context-service"
    fi
    sleep 0.25
  done
}

echo "== build context-service =="
(cd "$REPO_ROOT" && cargo build -q -p context-service)
CTX_BIN="$REPO_ROOT/target/debug/context-service"

if ! psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -tc \
  "SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND table_name='policy_configs'" \
  | grep -q '[[:space:]]*1[[:space:]]*$'; then
  die_env "policy_configs 不存在：请执行 010_optimization.sql"
fi

USER_ID="$(uuidgen | tr '[:upper:]' '[:lower:]')"
CONV_ID="bench-v22-$(date +%s)"
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -c \
  "INSERT INTO users (id) VALUES ('$USER_ID'::uuid) ON CONFLICT (id) DO NOTHING"

echo "== seed policy_configs (optimization-layer keys; context-service 只读) =="
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -c "
INSERT INTO policy_configs (policy_key, policy_domain, value_type, current_value, status)
VALUES
  ('graph_enabled', 'retrieval', 'bool', 'true', 'active'),
  ('rerank_enabled', 'retrieval', 'bool', 'true', 'active'),
  ('enabled_retrieval_modes', 'retrieval', 'json', '[\"structured\",\"semantic\",\"temporal\",\"graph\",\"rerank\"]', 'active')
ON CONFLICT (policy_key) DO UPDATE SET current_value = EXCLUDED.current_value, status = 'active', updated_at = now();
"

export DATABASE_URL INTERNAL_SHARED_SECRET
PORT="$CTX_PORT" "$CTX_BIN" &
PID=$!
trap 'kill "$PID" 2>/dev/null || true; wait "$PID" 2>/dev/null || true' EXIT
wait_health

OUT="$(mktemp)"
HTTP_CODE="$(curl -sS -o "$OUT" -w "%{http_code}" -X POST "http://127.0.0.1:${CTX_PORT}/internal/context/build" \
  -H "Content-Type: application/json" \
  -H "x-internal-token: ${INTERNAL_SHARED_SECRET}" \
  -d "{\"user_id\":\"$USER_ID\",\"agent_id\":\"00000000-0000-0000-0000-0000000000cc\",\"conversation_id\":\"$CONV_ID\",\"input\":\"v22 bench query $(date +%s)\",\"task_type\":\"chat\",\"max_tokens\":8000,\"memory_limit\":6,\"summary_limit\":3,\"reply_style\":\"brief\",\"trace_id\":\"v22-$RANDOM\",\"retrieval_modes\":[\"structured\",\"graph\",\"rerank\"]}")"

if [[ "$HTTP_CODE" != "200" ]]; then
  cat "$OUT" >&2
  rm -f "$OUT"
  die_impl "build failed http=$HTTP_CODE"
fi

HAS_GRAPH="$(jq -r '.retrieval_used | index("graph") != null' "$OUT")"
HAS_RERANK="$(jq -r '.retrieval_used | index("rerank") != null' "$OUT")"
rm -f "$OUT"

if [[ "$HAS_GRAPH" != "true" || "$HAS_RERANK" != "true" ]]; then
  die_impl "benchmark v2.2 failed: expected retrieval_used to contain graph and rerank (policy + request)"
fi

echo "== OK: benchmark v2.2 (graph+r+policy read path) =="
