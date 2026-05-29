#!/usr/bin/env bash
# Phase C：问卷 pending -> POST answers -> context（source_type=questionnaire）-> profile 最小纵切。
# 依赖服务默认端口：identity 8081, profile 8082, bff 8083, ai-chat 8085, question-service 8086, context 8089。
# model-gateway 8090 本脚本不直接调用。
# INTERNAL_SHARED_SECRET 须在 question-service / context / profile / ai-chat 间一致（默认 onelink-dev-internal-token）。
set -euo pipefail

EMAIL="qsmoke-$(date +%s)@example.com"
PASS="dev-password"
INTERNAL_HDR="${INTERNAL_SHARED_SECRET:-onelink-dev-internal-token}"

# Poll profile/me until questionnaire projection produces structured facts (async pipeline; no fixed sleep).
POLL_INTERVAL_SEC="${QUESTIONNAIRE_SMOKE_POLL_INTERVAL:-0.5}"
POLL_MAX_ATTEMPTS="${QUESTIONNAIRE_SMOKE_POLL_MAX:-120}"

profile_me_has_expected_facts() {
  local body="$1"
  echo "$body" | jq -e '
    (."facts" | type == "array") and (."facts" | length > 0) and
    (any(.facts[]; (.fact_type | type == "string") and (.value | type == "string"))) and
    (any(.facts[]; .fact_type == "location")) and
    (any(.facts[]; (.source_memory_id | type == "string") and (.source_memory_id | length > 0))) and
    (all(.facts[]; (.confidence | type == "number") and .confidence >= 0 and .confidence <= 1))
  ' >/dev/null 2>&1
}

echo "== register =="
REG=$(curl -sS -X POST "http://127.0.0.1:8081/api/v1/identity/register" \
  -H "Content-Type: application/json" \
  -d "{\"provider\":\"email\",\"email\":\"$EMAIL\",\"password_hash\":\"$PASS\",\"primary_region\":\"CN\",\"primary_language\":\"zh\"}")
echo "$REG" | jq .
TOKEN=$(echo "$REG" | jq -r .session.token)
USER_ID=$(echo "$REG" | jq -r .user_id)

echo "== bff onboarding (identity + pending_questions + questionnaire progress; not profile completion) =="
ONB=$(curl -sS "http://127.0.0.1:8083/api/v1/bff/onboarding" \
  -H "Authorization: Bearer $TOKEN")
echo "$ONB" | jq .

echo "$ONB" | jq -e '
  (.user | type == "object") and (.user.user_id | type == "string") and (.user.user_id | length > 0) and
  (.pending_questions | type == "array") and (.pending_questions | length > 0) and
  (.progress | type == "object") and
  ((.progress.degraded // false) | not) and
  (.progress.starter_required_total | type == "number")
' >/dev/null \
  || { echo "smoke failed: bff/onboarding missing user, pending, or questionnaire progress (is BFF built with onboarding route and QUESTION_SERVICE_BASE_URL set?)" >&2; exit 1; }

echo "== bff chat/init (pending_questions from question-service; failure -> []) =="
INIT=$(curl -sS "http://127.0.0.1:8083/api/v1/bff/chat/init" \
  -H "Authorization: Bearer $TOKEN")
echo "$INIT" | jq .

echo "$INIT" | jq -e '(.pending_questions | type == "array") and (.pending_questions | length > 0)' >/dev/null \
  || { echo "smoke failed: pending_questions missing or empty (is question-service on 8086 and BFF QUESTION_SERVICE_BASE_URL set?)" >&2; exit 1; }

echo "$INIT" | jq -e '.pending_questions[0] | (.delivery_id | type == "string") and (.variant_id | type == "string")' >/dev/null \
  || { echo "smoke failed: first pending_question missing delivery_id/variant_id" >&2; exit 1; }

BODY=$(echo "$INIT" | jq -c '
  .pending_questions[0] as $p |
  {
    delivery_id: $p.delivery_id,
    variant_id: $p.variant_id,
    answer_payload: (
      if $p.question_style == "open_text" then
        {"text": "smoke questionnaire open text"}
      else
        {"choice": ($p.options[0] | (if type == "string" then . else tostring end))}
      end
    ),
    answer_state: "answered"
  }
')

echo "== POST /api/v1/questions/answers (first pending; choice/text from item, no fixed copy) =="
ANS=$(curl -sS -X POST "http://127.0.0.1:8086/api/v1/questions/answers" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "$BODY")
echo "$ANS" | jq .

echo "$ANS" | jq -e '
  (.answer_id | type == "string") and (.answer_id | length > 0) and
  (.delivery_id | type == "string") and
  (.answered_at | type == "string") and (.answered_at | length > 0)
' >/dev/null || { echo "smoke failed: POST /answers response missing answer_id/delivery_id/answered_at" >&2; exit 1; }

echo "== wait async pipeline (poll profile/me for structured facts incl. location; max ${POLL_MAX_ATTEMPTS} attempts, ${POLL_INTERVAL_SEC}s apart) =="
PROFILE=""
attempt=0
ok=0
while (( attempt < POLL_MAX_ATTEMPTS )); do
  attempt=$((attempt + 1))
  PROFILE=$(curl -sS "http://127.0.0.1:8082/api/v1/profile/me" \
    -H "Authorization: Bearer $TOKEN") || true
  if profile_me_has_expected_facts "$PROFILE"; then
    echo "pipeline ready after poll attempt ${attempt}/${POLL_MAX_ATTEMPTS}"
    ok=1
    break
  fi
  sleep "$POLL_INTERVAL_SEC"
done
if (( ok != 1 )); then
  echo "smoke failed: profile/me never showed expected facts within budget." >&2
  echo "Hints: check context-service / profile INTERNAL_SHARED_SECRET; question-service relay logs; profile-service logs." >&2
  echo "Last profile/me body:" >&2
  echo "$PROFILE" | jq . >&2 || echo "$PROFILE" >&2
  exit 1
fi

echo "== profile /me =="
echo "$PROFILE" | jq .

echo "== profile /me/completion (五维口径；与问卷 status/completion 不联动) =="
COMPLETION=$(curl -sS "http://127.0.0.1:8082/api/v1/profile/me/completion" \
  -H "Authorization: Bearer $TOKEN")
echo "$COMPLETION" | jq .

echo "== Phase C assertions =="
profile_me_has_expected_facts "$PROFILE" \
  || { echo "smoke failed: profile/me missing expected facts (incl. location) after questionnaire answer" >&2; exit 1; }

echo "$COMPLETION" | jq -e '
  (.required_dimensions | sort) ==
    ["communication_preferences","connection_goals","current_location","display_name","interest_tags"] and
  (.completion_rate >= 0 and .completion_rate <= 1) and
  (((.filled_dimensions | length) + (.missing_dimensions | length)) == (.required_dimensions | length))
' >/dev/null || { echo "smoke failed: profile/me/completion five-dimension contract drift (see CHAT_MEMORY_PROFILE_SLICE.md)" >&2; exit 1; }

echo "== context asmr-lite (问卷-only 路径下仍应有 artifact/summary 计数) =="
ASM=$(curl -sS "http://127.0.0.1:8089/internal/observability/asmr-lite" \
  -H "x-internal-token: $INTERNAL_HDR")
echo "$ASM" | jq .

echo "$ASM" | jq -e '(.artifact_count | type == "number") and .artifact_count > 0 and (.summary_count | type == "number") and .summary_count > 0' >/dev/null \
  || { echo "smoke failed: context observability shows no artifacts/summaries (questionnaire path may not have reached context)" >&2; exit 1; }

echo "OK user_id=$USER_ID"
