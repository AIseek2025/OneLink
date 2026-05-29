#!/usr/bin/env bash
# Persistence smoke test: spins up Docker Postgres, applies DDL, runs context-service persistence smoke.
#
# Usage:
#   ./smoke-persistence-docker.sh          # full run (up + DDL + smoke + down)
#   ./smoke-persistence-docker.sh --skip-down  # leave Postgres running after test
#
# Requires: docker, cargo, curl, jq, psql (libpq)
set -euo pipefail

export PATH="$HOME/.cargo/bin:$PATH"

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DOCKER_DIR="$REPO_ROOT/infra/docker"
DDL_DIR="$REPO_ROOT/data-platform/db-schema/drafts"
SKIP_DOWN="${1:-}"

echo "== docker compose reset =="
(cd "$DOCKER_DIR" && docker compose down -v >/dev/null 2>&1 || true)

echo "== docker compose up =="
(cd "$DOCKER_DIR" && docker compose up -d --wait)

DATABASE_URL="postgres://onelink:onelink-local-dev-password@127.0.0.1:5432/onelink"
export DATABASE_URL
export INTERNAL_SHARED_SECRET="onelink-dev-internal-token"

run_psql() {
  if command -v psql >/dev/null 2>&1; then
    psql "$DATABASE_URL" "$@"
  else
    docker exec onelink-postgres psql -U onelink -d onelink "$@"
  fi
}

echo "== apply DDL in order =="
DDL_ORDER=(
  001_identity.sql
  002_profile.sql
  003_context.sql
  003_context_activation.sql
  003_context_idempotency.sql
  011_runtime_observability.sql
  004_ai_chat.sql
  005_dm.sql
  006_question.sql
  007_match.sql
  008_safety.sql
  010_optimization.sql
  009_model_gateway.sql
)
for f in "${DDL_ORDER[@]}"; do
  echo "  applying $f"
  if command -v psql >/dev/null 2>&1; then
    psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f "$DDL_DIR/$f"
  else
    docker exec -i onelink-postgres psql -U onelink -d onelink -v ON_ERROR_STOP=1 < "$DDL_DIR/$f"
  fi
done

echo "== DDL applied, running persistence smoke =="
"$REPO_ROOT/scripts/smoke-chat-memory-persistence.sh"

echo "== verify identity-service with Postgres =="
(cd "$REPO_ROOT" && cargo build -q -p identity-service)
IDENTITY_BIN="$REPO_ROOT/target/debug/identity-service"
IDENTITY_PORT=8081
export DATABASE_URL
export INTERNAL_SHARED_SECRET
PORT="$IDENTITY_PORT" "$IDENTITY_BIN" &
IDENTITY_PID=$!
trap 'kill "$IDENTITY_PID" 2>/dev/null || true' EXIT
i=0
while ! curl -sf "http://127.0.0.1:${IDENTITY_PORT}/health" >/dev/null; do
  i=$((i + 1))
  if [[ "$i" -gt 60 ]]; then
    echo "timeout: identity-service not ready" >&2
    exit 1
  fi
  sleep 0.25
done

echo "== identity-service register + login with Postgres =="
REGISTER_RESP="$(curl -sS -X POST "http://127.0.0.1:${IDENTITY_PORT}/api/v1/identity/register" \
  -H "Content-Type: application/json" \
  -d '{"provider":"email","email":"persistence-smoke@example.com","password":"smoke-pw-123","primary_region":"CN","primary_language":"zh"}')"
echo "$REGISTER_RESP" | jq .
TOKEN="$(echo "$REGISTER_RESP" | jq -r '.session.token')"
if [[ -z "$TOKEN" || "$TOKEN" == "null" ]]; then
  echo "register failed: no token returned" >&2
  exit 1
fi

ME_RESP="$(curl -sS "http://127.0.0.1:${IDENTITY_PORT}/api/v1/identity/me" \
  -H "Authorization: Bearer $TOKEN")"
echo "$ME_RESP" | jq .
ME_STATUS="$(echo "$ME_RESP" | jq -r '.status')"
if [[ "$ME_STATUS" != "active" ]]; then
  echo "me endpoint returned unexpected status: $ME_STATUS" >&2
  exit 1
fi

echo "== verify DB row exists =="
run_psql -tc "SELECT count(*) FROM users WHERE id IS NOT NULL" | tr -d ' '

echo "== identity-service persistence smoke OK =="
kill "$IDENTITY_PID" 2>/dev/null || true
trap - EXIT

if [[ "$SKIP_DOWN" != "--skip-down" ]]; then
  echo "== docker compose down =="
  (cd "$DOCKER_DIR" && docker compose down -v)
  echo "Postgres container removed"
else
  echo "Postgres container left running (--skip-down). To remove: cd $DOCKER_DIR && docker compose down -v"
fi

echo "ALL PERSISTENCE SMOKE TESTS PASSED"
