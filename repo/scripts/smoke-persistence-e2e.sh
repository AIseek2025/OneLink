#!/usr/bin/env bash
# Persistence smoke test: verifies that identity-service, profile-service, and
# context-service correctly persist data to Postgres when DATABASE_URL is set,
# and that data survives across service restarts.
#
# Prerequisites:
#   - Postgres running with DDL applied (Docker or native)
#   - DATABASE_URL set to a valid Postgres connection string
#   - psql, cargo, curl, jq available
#
# Usage:
#   RUN_PERSISTENCE_SMOKE=1 cargo test -p onelink-persistence-smoke
#   Or directly: ./smoke-persistence-e2e.sh
set -euo pipefail

export PATH="$HOME/.cargo/bin:$PATH"

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DATABASE_URL="${DATABASE_URL:-postgres://onelink:onelink-local-dev-password@127.0.0.1:5432/onelink}"
export DATABASE_URL
export INTERNAL_SHARED_SECRET="onelink-dev-internal-token"
export ONELINK_ENV="dev"

PIDS=()
cleanup() {
    for pid in ${PIDS+"${PIDS[@]}"}; do
        kill "$pid" 2>/dev/null || true
    done
    for pid in ${PIDS+"${PIDS[@]}"}; do
        wait "$pid" 2>/dev/null || true
    done
    PIDS=()
}
trap cleanup EXIT

wait_for_health() {
    local port=$1
    local timeout=${2:-20}
    local i=0
    while ! curl -sf "http://127.0.0.1:${port}/health" >/dev/null 2>&1; do
        i=$((i + 1))
        if [[ "$i" -gt $((timeout * 4)) ]]; then
            echo "timeout: service on port ${port} not ready after ${timeout}s" >&2
            return 1
        fi
        sleep 0.25
    done
    echo "  service ready on port ${port}"
}

run_psql() {
    if command -v psql >/dev/null 2>&1; then
        psql "$DATABASE_URL" "$@"
    else
        docker exec onelink-postgres psql -U onelink -d onelink "$@"
    fi
}

echo "=== persistence smoke: verify Postgres is reachable ==="
run_psql -c "SELECT 1" >/dev/null 2>&1 || {
    echo "Postgres not ready at $DATABASE_URL. Ensure Postgres is running and DDL is applied." >&2
    exit 1
}

TABLE_COUNT="$(run_psql -tc "SELECT count(*) FROM information_schema.tables WHERE table_schema='public'" | tr -d ' ')"
if [[ "$TABLE_COUNT" -lt 10 ]]; then
    echo "DDL not fully applied (only $TABLE_COUNT tables). Apply DDL first." >&2
    exit 1
fi
echo "  Postgres OK: $TABLE_COUNT tables exist"

echo "=== persistence smoke: build all services ==="
(cd "$REPO_ROOT" && cargo build -q -p identity-service -p profile-service -p context-service)

IDENTITY_BIN="$REPO_ROOT/target/debug/identity-service"
PROFILE_BIN="$REPO_ROOT/target/debug/profile-service"
CONTEXT_BIN="$REPO_ROOT/target/debug/context-service"

IDENTITY_PORT=18081
PROFILE_PORT=18082
CONTEXT_PORT=18089

# ── Phase 1: identity-service ──

echo "=== persistence smoke: start identity-service with DATABASE_URL ==="
PORT=$IDENTITY_PORT "$IDENTITY_BIN" &
IDENTITY_PID=$!
PIDS+=("$IDENTITY_PID")
wait_for_health "$IDENTITY_PORT"

echo "=== persistence smoke: register user via HTTP ==="
EMAIL_UNIQUE="persistence-e2e-$(date +%s)@example.com"
REGISTER_RESP="$(curl -sS -X POST "http://127.0.0.1:${IDENTITY_PORT}/api/v1/identity/register" \
    -H "Content-Type: application/json" \
    -d "{\"provider\":\"email\",\"email\":\"${EMAIL_UNIQUE}\",\"password\":\"smoke-pw-persist-123\",\"primary_region\":\"CN\",\"primary_language\":\"zh\"}")"
echo "$REGISTER_RESP" | jq .
TOKEN="$(echo "$REGISTER_RESP" | jq -r '.session.token')"
USER_ID="$(echo "$REGISTER_RESP" | jq -r '.user_id')"
if [[ -z "$TOKEN" || "$TOKEN" == "null" ]]; then
    echo "register failed: no token returned" >&2
    exit 1
fi
echo "  registered user $USER_ID"

echo "=== persistence smoke: verify /me returns correct user ==="
ME_RESP="$(curl -sS "http://127.0.0.1:${IDENTITY_PORT}/api/v1/identity/me" \
    -H "Authorization: Bearer $TOKEN")"
echo "$ME_RESP" | jq .
ME_STATUS="$(echo "$ME_RESP" | jq -r '.status')"
ME_REGION="$(echo "$ME_RESP" | jq -r '.primary_region')"
if [[ "$ME_STATUS" != "active" ]]; then
    echo "/me returned unexpected status: $ME_STATUS" >&2
    exit 1
fi
if [[ "$ME_REGION" != "CN" ]]; then
    echo "/me returned unexpected region: $ME_REGION" >&2
    exit 1
fi
echo "  /me verified: status=$ME_STATUS region=$ME_REGION"

echo "=== persistence smoke: verify identity data persisted to Postgres ==="
DB_USER_COUNT="$(run_psql -tc "SELECT count(*) FROM users" | tr -d ' ')"
DB_SESSION_COUNT="$(run_psql -tc "SELECT count(*) FROM sessions" | tr -d ' ')"
if [[ "$DB_USER_COUNT" -lt 1 ]]; then
    echo "no users in DB — identity persistence failed" >&2
    exit 1
fi
if [[ "$DB_SESSION_COUNT" -lt 1 ]]; then
    echo "no sessions in DB — identity persistence failed" >&2
    exit 1
fi
echo "  DB verified: $DB_USER_COUNT user(s), $DB_SESSION_COUNT session(s)"

# ── Phase 2: profile-service ──

echo "=== persistence smoke: start profile-service with DATABASE_URL ==="
PORT=$PROFILE_PORT \
IDENTITY_SERVICE_BASE_URL="http://127.0.0.1:${IDENTITY_PORT}" \
CONTEXT_SERVICE_BASE_URL="http://127.0.0.1:${CONTEXT_PORT}" \
"$PROFILE_BIN" &
PROFILE_PID=$!
PIDS+=("$PROFILE_PID")
wait_for_health "$PROFILE_PORT"

echo "=== persistence smoke: send profile event via internal endpoint ==="
PROFILE_EVENT="$(cat <<'EOF'
{
    "event_id": "evt-profile-persist-001",
    "event_name": "profile.fact.created.v1",
    "event_version": "v1",
    "occurred_at": "2026-05-17T23:00:00.000Z",
    "producer": "persistence-smoke",
    "payload": {
        "user_id": "__USER_ID__",
        "fact_type": "preference",
        "fact_key": "language_preference",
        "fact_value_json": {"preferred": "zh"},
        "source_type": "questionnaire",
        "confidence": 0.9,
        "status": "active"
    }
}
EOF
)"
PROFILE_EVENT="$(echo "$PROFILE_EVENT" | sed "s/__USER_ID__/$USER_ID/")"

PROFILE_RESP="$(curl -sS -o /dev/null -w '%{http_code}' \
    -X POST "http://127.0.0.1:${PROFILE_PORT}/internal/events/receive" \
    -H "Content-Type: application/json" \
    -H "x-internal-token: onelink-dev-internal-token" \
    -d "$PROFILE_EVENT")"
if [[ "$PROFILE_RESP" != "202" ]]; then
    echo "profile event ingest returned $PROFILE_RESP, expected 202" >&2
    exit 1
fi
echo "  profile event ingested: 202"

echo "=== persistence smoke: verify profile data persisted to Postgres ==="
sleep 2
DB_PROFILE_FACT_COUNT="$(run_psql -tc "SELECT count(*) FROM profile_facts WHERE user_id = '${USER_ID}'::uuid AND status = 'active'" | tr -d ' ')"
if [[ "$DB_PROFILE_FACT_COUNT" -lt 1 ]]; then
    echo "no profile_facts in DB for user $USER_ID — profile persistence failed" >&2
    DB_PROFILE_FACT_TOTAL="$(run_psql -tc "SELECT count(*) FROM profile_facts" | tr -d ' ')"
    echo "  total profile_facts: $DB_PROFILE_FACT_TOTAL"
    DB_PROFILE_COUNT="$(run_psql -tc "SELECT count(*) FROM profiles WHERE user_id = '${USER_ID}'::uuid" | tr -d ' ')"
    echo "  profiles for user: $DB_PROFILE_COUNT"
    DB_USER_IN_TABLE="$(run_psql -tc "SELECT count(*) FROM users WHERE id = '${USER_ID}'::uuid" | tr -d ' ')"
    echo "  users for id: $DB_USER_IN_TABLE"
    exit 1
else
    echo "  DB verified: $DB_PROFILE_FACT_COUNT profile_fact(s) for user $USER_ID"
fi

# ── Phase 3: context-service ──

echo "=== persistence smoke: start context-service with DATABASE_URL ==="
PORT=$CONTEXT_PORT \
PROFILE_SERVICE_BASE_URL="http://127.0.0.1:${PROFILE_PORT}" \
AI_CHAT_SERVICE_BASE_URL="http://127.0.0.1:18085" \
"$CONTEXT_BIN" &
CONTEXT_PID=$!
PIDS+=("$CONTEXT_PID")
wait_for_health "$CONTEXT_PORT"

echo "=== persistence smoke: send context event via internal endpoint ==="
CONTEXT_EVENT="$(cat <<'EOF'
{
    "event_id": "evt-context-persist-001",
    "event_name": "chat.user_message.created.v1",
    "event_version": "v1",
    "occurred_at": "2026-05-17T23:00:00.000Z",
    "producer": "persistence-smoke",
    "payload": {
        "user_id": "__USER_ID__",
        "conversation_id": "conv-persist-smoke-001",
        "message_id": "msg-persist-smoke-001",
        "content": "我对AI创业很感兴趣，希望认识投资人"
    }
}
EOF
)"
CONTEXT_EVENT="$(echo "$CONTEXT_EVENT" | sed "s/__USER_ID__/$USER_ID/")"

CONTEXT_RESP="$(curl -sS -o /dev/null -w '%{http_code}' \
    -X POST "http://127.0.0.1:${CONTEXT_PORT}/internal/events/receive" \
    -H "Content-Type: application/json" \
    -H "x-internal-token: onelink-dev-internal-token" \
    -d "$CONTEXT_EVENT")"
if [[ "$CONTEXT_RESP" != "202" ]]; then
    echo "context event ingest returned $CONTEXT_RESP, expected 202" >&2
    exit 1
fi
echo "  context event ingested: 202"

echo "=== persistence smoke: wait for async context pipeline ==="
sleep 3

echo "=== persistence smoke: build context via internal endpoint ==="
BUILD_RESP="$(curl -sS -X POST "http://127.0.0.1:${CONTEXT_PORT}/internal/context/build" \
    -H "Content-Type: application/json" \
    -H "x-internal-token: onelink-dev-internal-token" \
    -d "{\"user_id\":\"${USER_ID}\",\"agent_id\":\"lumi\",\"conversation_id\":\"conv-persist-smoke-001\",\"input\":\"hello\",\"task_type\":\"chat\",\"max_tokens\":512,\"memory_limit\":10,\"summary_limit\":5}")"
BUILD_DEGRADED="$(echo "$BUILD_RESP" | jq -r '.degraded // "unknown"')"
echo "  context build: degraded=$BUILD_DEGRADED"

echo "=== persistence smoke: verify context data persisted to Postgres ==="
sleep 2
DB_ARTIFACT_COUNT="$(run_psql -tc "SELECT count(*) FROM memory_artifacts WHERE user_id = '${USER_ID}'::uuid" | tr -d ' ')"
DB_SUMMARY_COUNT="$(run_psql -tc "SELECT count(*) FROM memory_summaries WHERE user_id = '${USER_ID}'::uuid" | tr -d ' ')"
if [[ "$DB_ARTIFACT_COUNT" -lt 1 ]]; then
    echo "no memory_artifacts in DB for user $USER_ID — context persistence may have failed" >&2
else
    echo "  DB verified: $DB_ARTIFACT_COUNT artifact(s), $DB_SUMMARY_COUNT summary(s) for user $USER_ID"
fi

# ── Phase 4: restart all services and verify data survives ──

echo "=== persistence smoke: stop all services ==="
cleanup

echo "=== persistence smoke: restart identity-service ==="
PORT=$IDENTITY_PORT "$IDENTITY_BIN" &
IDENTITY_PID=$!
PIDS+=("$IDENTITY_PID")
wait_for_health "$IDENTITY_PORT"

echo "=== persistence smoke: login with same credentials after restart ==="
LOGIN_RESP="$(curl -sS -X POST "http://127.0.0.1:${IDENTITY_PORT}/api/v1/identity/login" \
    -H "Content-Type: application/json" \
    -d "{\"provider\":\"email\",\"email\":\"${EMAIL_UNIQUE}\",\"password\":\"smoke-pw-persist-123\"}")"
echo "$LOGIN_RESP" | jq .
LOGIN_USER_ID="$(echo "$LOGIN_RESP" | jq -r '.user_id')"
if [[ "$LOGIN_USER_ID" != "$USER_ID" ]]; then
    echo "login after restart returned different user_id: $LOGIN_USER_ID vs $USER_ID" >&2
    exit 1
fi
echo "  login after restart verified: same user_id=$LOGIN_USER_ID"

echo "=== persistence smoke: verify identity internal auth after restart ==="
INTERNAL_RESP="$(curl -sS -o /dev/null -w '%{http_code}' \
    "http://127.0.0.1:${IDENTITY_PORT}/internal/identity/health-detail" \
    -H "x-internal-token: onelink-dev-internal-token")"
if [[ "$INTERNAL_RESP" != "200" ]]; then
    echo "internal auth after restart returned $INTERNAL_RESP, expected 200" >&2
    exit 1
fi
echo "  internal auth after restart: 200 OK"

echo "=== persistence smoke: restart profile-service ==="
PORT=$PROFILE_PORT \
IDENTITY_SERVICE_BASE_URL="http://127.0.0.1:${IDENTITY_PORT}" \
CONTEXT_SERVICE_BASE_URL="http://127.0.0.1:${CONTEXT_PORT}" \
"$PROFILE_BIN" &
PROFILE_PID=$!
PIDS+=("$PROFILE_PID")
wait_for_health "$PROFILE_PORT"

echo "=== persistence smoke: verify profile data survives restart ==="
DB_PROFILE_FACT_AFTER="$(run_psql -tc "SELECT count(*) FROM profile_facts WHERE user_id = '${USER_ID}'::uuid AND status = 'active'" | tr -d ' ')"
if [[ "$DB_PROFILE_FACT_AFTER" -lt 1 ]]; then
    echo "profile_facts lost after restart — persistence failed" >&2
    DB_PROFILE_AFTER="$(run_psql -tc "SELECT count(*) FROM profiles WHERE user_id = '${USER_ID}'::uuid" | tr -d ' ')"
    echo "  profiles for user after restart: $DB_PROFILE_AFTER"
    exit 1
fi
echo "  profile data survives restart: $DB_PROFILE_FACT_AFTER fact(s)"

echo "=== persistence smoke: restart context-service ==="
PORT=$CONTEXT_PORT \
PROFILE_SERVICE_BASE_URL="http://127.0.0.1:${PROFILE_PORT}" \
AI_CHAT_SERVICE_BASE_URL="http://127.0.0.1:18085" \
"$CONTEXT_BIN" &
CONTEXT_PID=$!
PIDS+=("$CONTEXT_PID")
wait_for_health "$CONTEXT_PORT"

echo "=== persistence smoke: verify context data survives restart ==="
DB_ARTIFACT_AFTER="$(run_psql -tc "SELECT count(*) FROM memory_artifacts WHERE user_id = '${USER_ID}'::uuid" | tr -d ' ')"
DB_SUMMARY_AFTER="$(run_psql -tc "SELECT count(*) FROM memory_summaries WHERE user_id = '${USER_ID}'::uuid" | tr -d ' ')"
if [[ "$DB_ARTIFACT_AFTER" -lt 1 ]]; then
    echo "memory_artifacts lost after restart — context persistence failed" >&2
    exit 1
fi
echo "  context data survives restart: $DB_ARTIFACT_AFTER artifact(s), $DB_SUMMARY_AFTER summary(s)"

echo "=== persistence smoke: verify context internal auth after restart ==="
CONTEXT_AUTH="$(curl -sS -o /dev/null -w '%{http_code}' \
    "http://127.0.0.1:${CONTEXT_PORT}/internal/events/receive" \
    -H "Content-Type: application/json" \
    -d "$CONTEXT_EVENT")"
if [[ "$CONTEXT_AUTH" != "401" ]]; then
    echo "context internal route without token returned $CONTEXT_AUTH, expected 401" >&2
    exit 1
fi
echo "  context auth rejection after restart: 401 (no token)"

CONTEXT_AUTH_OK="$(curl -sS -o /dev/null -w '%{http_code}' \
    -X POST "http://127.0.0.1:${CONTEXT_PORT}/internal/events/receive" \
    -H "Content-Type: application/json" \
    -H "x-internal-token: onelink-dev-internal-token" \
    -d "$CONTEXT_EVENT")"
if [[ "$CONTEXT_AUTH_OK" != "202" ]]; then
    echo "context internal route with correct token returned $CONTEXT_AUTH_OK, expected 202" >&2
    exit 1
fi
echo "  context auth acceptance after restart: 202"

echo "=== persistence smoke: verify profile internal auth after restart ==="
PROFILE_AUTH="$(curl -sS -o /dev/null -w '%{http_code}' \
    -X POST "http://127.0.0.1:${PROFILE_PORT}/internal/events/receive" \
    -H "Content-Type: application/json" \
    -d "$PROFILE_EVENT")"
if [[ "$PROFILE_AUTH" != "401" ]]; then
    echo "profile internal route without token returned $PROFILE_AUTH, expected 401" >&2
    exit 1
fi
echo "  profile auth rejection after restart: 401 (no token)"

cleanup

echo ""
echo "ALL PERSISTENCE SMOKE TESTS PASSED"
