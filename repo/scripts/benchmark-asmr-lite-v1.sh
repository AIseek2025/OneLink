#!/usr/bin/env bash
# 最小 ASMR-Lite benchmark v1：
# - 成功样本：L1 命中 + profile 可见
# - 升级样本：candidate_route 升级但 executed_route 仍为 L1，并留下带 trace_id 的 failure sample
set -euo pipefail

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

require_cmd curl
require_cmd jq

INTERNAL_SHARED_SECRET="${INTERNAL_SHARED_SECRET:-onelink-dev-internal-token}"
EMAIL="benchmark-$(date +%s)@example.com"
PASS="dev-password"

echo "== register =="
REG=$(curl -sS -X POST "http://127.0.0.1:8081/api/v1/identity/register" \
  -H "Content-Type: application/json" \
  -d "{\"provider\":\"email\",\"email\":\"$EMAIL\",\"password_hash\":\"$PASS\",\"primary_region\":\"CN\",\"primary_language\":\"zh\"}")
TOKEN=$(echo "$REG" | jq -r '.session.token')
if [[ -z "$TOKEN" || "$TOKEN" == "null" ]]; then
  echo "failed to obtain token from register" >&2
  echo "$REG" | jq .
  exit 1
fi

echo "== init conversation =="
INIT=$(curl -sS "http://127.0.0.1:8083/api/v1/bff/chat/init" \
  -H "Authorization: Bearer $TOKEN")
CONV=$(echo "$INIT" | jq -r '.conversation.conversation_id')
if [[ -z "$CONV" || "$CONV" == "null" ]]; then
  echo "failed to obtain conversation id from bff/chat/init" >&2
  echo "$INIT" | jq .
  exit 1
fi

echo "== sample 1: success / L1 =="
curl -sS -X POST "http://127.0.0.1:8085/api/v1/chat/conversations/$CONV/messages" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"content_type":"text","content_text":"我在上海做AI产品","idempotency_key":"benchmark-success"}' \
  | jq .
sleep 2

OBS1=$(curl -sS "http://127.0.0.1:8089/internal/observability/asmr-lite" \
  -H "x-internal-token: $INTERNAL_SHARED_SECRET")
echo "$OBS1" | jq .

ARTIFACT_COUNT=$(echo "$OBS1" | jq -r '.artifact_count')
SUMMARY_COUNT=$(echo "$OBS1" | jq -r '.summary_count')
ENTITY_COUNT=$(echo "$OBS1" | jq -r '.entity_count')
CHECKPOINT_COUNT=$(echo "$OBS1" | jq -r '.checkpoint_count')
EXECUTED_ROUTE=$(echo "$OBS1" | jq -r '.routing.last_observation.executed_route')
CANDIDATE_ROUTE_1=$(echo "$OBS1" | jq -r '.routing.last_observation.candidate_route')
if [[ "${ARTIFACT_COUNT:-0}" -lt 1 ]]; then
  echo "benchmark failed: artifact_count < 1 after success sample" >&2
  exit 1
fi
if [[ "${SUMMARY_COUNT:-0}" -lt 1 ]]; then
  echo "benchmark failed: summary_count < 1 after success sample" >&2
  exit 1
fi
if [[ "${ENTITY_COUNT:-0}" -lt 1 ]]; then
  echo "benchmark failed: entity_count < 1 after success sample" >&2
  exit 1
fi
if [[ "${CHECKPOINT_COUNT:-0}" -lt 1 ]]; then
  echo "benchmark failed: checkpoint_count < 1 after success sample" >&2
  exit 1
fi
if [[ "$EXECUTED_ROUTE" != "L1" ]]; then
  echo "benchmark failed: expected executed_route=L1 after success sample, got $EXECUTED_ROUTE" >&2
  exit 1
fi
if [[ "$CANDIDATE_ROUTE_1" != "L1" ]]; then
  echo "benchmark failed: expected candidate_route=L1 after success sample, got $CANDIDATE_ROUTE_1" >&2
  exit 1
fi

echo "== sample 2: escalation / deferred =="
curl -sS -X POST "http://127.0.0.1:8085/api/v1/chat/conversations/$CONV/messages" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"content_type":"text","content_text":"我之前在北京，现在在上海，后来改为远程办公","idempotency_key":"benchmark-escalation"}' \
  | jq .
sleep 2

OBS2=$(curl -sS "http://127.0.0.1:8089/internal/observability/asmr-lite" \
  -H "x-internal-token: $INTERNAL_SHARED_SECRET")
echo "$OBS2" | jq .

CANDIDATE_ROUTE=$(echo "$OBS2" | jq -r '.routing.last_observation.candidate_route')
EXECUTED_ROUTE_2=$(echo "$OBS2" | jq -r '.routing.last_observation.executed_route')
ESCALATION_SEEN=$(echo "$OBS2" | jq -r '[.recent_failures[] | select(.category == "route_escalation_deferred")] | length')
ESCALATION_TRACE_SEEN=$(echo "$OBS2" | jq -r '[.recent_failures[] | select(.category == "route_escalation_deferred" and (.trace_id != null and .trace_id != ""))] | length')
if [[ "$CANDIDATE_ROUTE" != "L3" ]]; then
  echo "benchmark failed: expected candidate_route=L3 after escalation sample, got $CANDIDATE_ROUTE" >&2
  exit 1
fi
if [[ "$EXECUTED_ROUTE_2" != "L1" ]]; then
  echo "benchmark failed: expected executed_route=L1 after escalation sample, got $EXECUTED_ROUTE_2" >&2
  exit 1
fi
if [[ "${ESCALATION_SEEN:-0}" -lt 1 ]]; then
  echo "benchmark failed: expected route_escalation_deferred in recent_failures" >&2
  exit 1
fi
if [[ "${ESCALATION_TRACE_SEEN:-0}" -lt 1 ]]; then
  echo "benchmark failed: expected route_escalation_deferred to carry non-null trace_id" >&2
  exit 1
fi

echo "== profile visible =="
PROFILE=$(curl -sS "http://127.0.0.1:8082/api/v1/profile/me" \
  -H "Authorization: Bearer $TOKEN")
echo "$PROFILE" | jq .
HEADLINE=$(echo "$PROFILE" | jq -r '.headline')
if [[ "$HEADLINE" != *"记忆已同步"* ]]; then
  echo "benchmark failed: profile headline does not show projection result" >&2
  exit 1
fi

echo "== ai-chat relay observability =="
CHAT_OBS=$(curl -sS "http://127.0.0.1:8085/internal/observability/chat-relay" \
  -H "x-internal-token: $INTERNAL_SHARED_SECRET")
echo "$CHAT_OBS" | jq .
CHAT_RELAY_FAILURES=$(echo "$CHAT_OBS" | jq -r '.total_failures')
if [[ "${CHAT_RELAY_FAILURES:-0}" != "0" ]]; then
  echo "benchmark failed: expected chat relay total_failures == 0, got $CHAT_RELAY_FAILURES" >&2
  exit 1
fi

echo "benchmark v1 passed"
