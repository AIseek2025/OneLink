#!/usr/bin/env bash
# Shared helpers for benchmark-asmr-lite-v2.sh and benchmark-asmr-lite-v2.1.sh.
# Sourcing script MUST set ASMR_BENCH_RUN_TAG (e.g. benchmark-v2 / benchmark-v21)
# before: source "$(dirname "$0")/lib/asmr-benchmark-v2-common.sh"

: "${ASMR_BENCH_RUN_TAG:?set ASMR_BENCH_RUN_TAG before sourcing asmr-benchmark-v2-common.sh}"

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

# Lexical-FullTranscript（脚本内函数名仍为 baseline_a，与历史 runner 兼容）
baseline_a() {
  local query="$1"
  local messages="$2"
  if [[ "$query" == *"现在"* && ( "$query" == *"哪"* || "$query" == *"城市"* ) ]]; then
    if [[ "$messages" == *"上海"* ]]; then
      echo "上海"
      return
    fi
  fi
  if [[ "$query" == *"之前"* && ( "$query" == *"哪"* || "$query" == *"城市"* ) ]]; then
    if [[ "$messages" == *"北京"* ]]; then
      echo "北京"
      return
    fi
  fi
  if [[ "$query" == *"沟通"* || "$query" == *"偏好"* ]]; then
    if [[ "$messages" == *"不喜欢被推销式沟通"* ]]; then
      echo "不喜欢被推销式沟通"
      return
    fi
  fi
  if [[ "$query" == *"认识谁"* || "$query" == *"投资人"* || "$query" == *"合伙人"* ]]; then
    if [[ "$messages" == *"投资人"* ]]; then
      echo "投资人"
      return
    fi
  fi
  if [[ "$query" == *"办公方式"* || "$query" == *"远程"* ]]; then
    if [[ "$messages" == *"远程办公"* ]]; then
      echo "远程办公"
      return
    fi
  fi
  echo "unknown"
}

baseline_b() {
  local query="$1"
  local latest_message="$2"
  baseline_a "$query" "$latest_message"
}

contains_expectations() {
  local text="$1"
  local expected_json="$2"
  EXPECTED_JSON="$expected_json" TEXT="$text" python3 - <<'PY'
import json
import os
expected = json.loads(os.environ["EXPECTED_JSON"])
text = os.environ["TEXT"]
print("true" if all(token in text for token in expected) else "false")
PY
}

register_user() {
  local email="${ASMR_BENCH_RUN_TAG}-$(date +%s)-$RANDOM@example.com"
  curl -sS -X POST "http://127.0.0.1:8081/api/v1/identity/register" \
    -H "Content-Type: application/json" \
    -d "{\"provider\":\"email\",\"email\":\"$email\",\"password_hash\":\"$PASS\",\"primary_region\":\"CN\",\"primary_language\":\"zh\"}"
}

init_conversation() {
  local token="$1"
  curl -sS "http://127.0.0.1:8083/api/v1/bff/chat/init" \
    -H "Authorization: Bearer $token"
}

send_message() {
  local token="$1"
  local conv="$2"
  local content="$3"
  local key="$4"
  curl -sS -X POST "http://127.0.0.1:8085/api/v1/chat/conversations/$conv/messages" \
    -H "Authorization: Bearer $token" \
    -H "Content-Type: application/json" \
    -d "{\"content_type\":\"text\",\"content_text\":$(jq -Rn --arg v "$content" '$v'),\"idempotency_key\":$(jq -Rn --arg v "$key" '$v')}" >/dev/null
}

build_context() {
  local user_id="$1"
  local conv="$2"
  local query="$3"
  local agent_id="$4"
  curl -sS -X POST "http://127.0.0.1:8089/internal/context/build" \
    -H "Content-Type: application/json" \
    -H "x-internal-token: $INTERNAL_SHARED_SECRET" \
    -d "{\"user_id\":\"$user_id\",\"agent_id\":\"$agent_id\",\"conversation_id\":\"$conv\",\"input\":$(jq -Rn --arg v "$query" '$v'),\"task_type\":\"chat\",\"max_tokens\":8000,\"memory_limit\":6,\"summary_limit\":3,\"reply_style\":\"brief\",\"trace_id\":\"${ASMR_BENCH_RUN_TAG}-trace-$RANDOM\",\"retrieval_modes\":[\"structured\",\"temporal\"]}"
}

read_observability() {
  curl -sS "http://127.0.0.1:8089/internal/observability/asmr-lite" \
    -H "x-internal-token: $INTERNAL_SHARED_SECRET"
}

assert_observation_shape() {
  local obs_json="$1"
  local missing
  missing=$(echo "$obs_json" | jq -r '
    .routing.last_observation as $o
    | [
        ($o.summary_hits == null),
        ($o.artifact_hits == null),
        ($o.entity_hits == null),
        ($o.route_confidence == null),
        ($o.estimated_llm_calls == null),
        ($o.estimated_tokens == null),
        ($o.query_preview == null),
        ($o.upgraded == null),
        ($o.query_preference_polarity == null),
        ($o.evidence_preference_polarity == null),
        ($o.retrieval_modes == null),
        (($o.retrieval_modes | type) != "array"),
        (.policy_version_label == null),
        ((.policy_version_label | type) != "string"),
        (.policy_version_label == ""),
        ((.summary_count > 0) and (.latest_summary_policy_version == null))
      ]
    | any
  ')
  if [[ "$missing" == "true" ]]; then
    echo "${ASMR_BENCH_SHAPE_ERR:-benchmark}: asmr-lite observability shape failed (last_observation incl. retrieval_modes, policy_version_label, latest_summary when summaries exist)" >&2
    exit 1
  fi
}
