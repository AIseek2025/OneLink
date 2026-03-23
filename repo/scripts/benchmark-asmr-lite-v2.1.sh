#!/usr/bin/env bash
# ASMR-Lite benchmark v2.1:
# - 不替代 v1 / v2：本脚本只跑 v2.1 增补套件（歧视性样本 + entity 断言）
# - 对照组命名（与 v2 脚本内函数一致）：
#     Baseline-A  = Lexical-FullTranscript（全量 setup 拼接词法规则）
#     Baseline-B  = Lexical-LatestMessage（仅最后一条 setup）
# - OneLink-L1  = context-service POST /internal/context/build + observability
set -euo pipefail

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

require_cmd curl
require_cmd jq

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DATA_DIR="${ASMR_BENCHMARK_V2_1_DATA_DIR:-$SCRIPT_DIR/../tests/integration/asmr_benchmark_v2_1}"
INTERNAL_SHARED_SECRET="${INTERNAL_SHARED_SECRET:-onelink-dev-internal-token}"
PASS="dev-password"

# Lexical-FullTranscript（脚本内函数名仍为 baseline_a，与 v2 兼容）
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
  local email="benchmark-v21-$(date +%s)-$RANDOM@example.com"
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
    -d "{\"user_id\":\"$user_id\",\"agent_id\":\"$agent_id\",\"conversation_id\":\"$conv\",\"input\":$(jq -Rn --arg v "$query" '$v'),\"task_type\":\"chat\",\"max_tokens\":8000,\"memory_limit\":6,\"summary_limit\":3,\"reply_style\":\"brief\",\"trace_id\":\"benchmark-v21-trace-$RANDOM\",\"retrieval_modes\":[\"structured\",\"temporal\"]}"
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
        ($o.evidence_preference_polarity == null)
      ]
    | any
  ')
  if [[ "$missing" == "true" ]]; then
    echo "benchmark v2.1 failed: last_observation missing required fields" >&2
    exit 1
  fi
}

run_suite() {
  local file="$1"
  local suite_name
  suite_name=$(jq -r '.suite' "$file")
  local setup_text
  setup_text=$(jq -r '.setup_messages | join("\n")' "$file")
  local latest_message
  latest_message=$(jq -r '.setup_messages[-1] // ""' "$file")

  echo "== suite: $suite_name =="

  local reg token user_id init conv agent_id
  reg=$(register_user)
  token=$(echo "$reg" | jq -r '.session.token')
  user_id=$(echo "$reg" | jq -r '.user_id')
  init=$(init_conversation "$token")
  conv=$(echo "$init" | jq -r '.conversation.conversation_id')
  agent_id="benchmark-v21-agent-$RANDOM"

  if [[ -z "$token" || "$token" == "null" || -z "$conv" || "$conv" == "null" ]]; then
    echo "benchmark v2.1 failed: unable to initialize suite $suite_name" >&2
    exit 1
  fi

  local index=0
  while IFS= read -r message; do
    send_message "$token" "$conv" "$message" "benchmark-v21-$suite_name-setup-$index"
    index=$((index + 1))
    sleep 1
  done < <(jq -r '.setup_messages[]' "$file")
  sleep 2

  local suite_total=0
  local suite_l1=0
  local suite_l1_beats_both=0

  while IFS= read -r case_json; do
    local case_id query expected_json expected_route
    case_id=$(echo "$case_json" | jq -r '.id')
    query=$(echo "$case_json" | jq -r '.query')
    expected_json=$(echo "$case_json" | jq -c '.expected_contains')
    expected_route=$(echo "$case_json" | jq -r '.expected_candidate_route')

    local build obs memory_context task_context candidate_route executed_route entity_hits
    build=$(build_context "$user_id" "$conv" "$query" "$agent_id")
    obs=$(read_observability)
    assert_observation_shape "$obs"

    memory_context=$(echo "$build" | jq -r '.memory_context')
    task_context=$(echo "$build" | jq -r '.task_context')
    candidate_route=$(echo "$obs" | jq -r '.routing.last_observation.candidate_route')
    executed_route=$(echo "$obs" | jq -r '.routing.last_observation.executed_route')
    entity_hits=$(echo "$obs" | jq -r '.routing.last_observation.entity_hits')

    local baseline_a_out baseline_b_out
    baseline_a_out=$(baseline_a "$query" "$setup_text")
    baseline_b_out=$(baseline_b "$query" "$latest_message")

    local pass_a pass_b pass_l1
    pass_a=$(contains_expectations "$baseline_a_out" "$expected_json")
    pass_b=$(contains_expectations "$baseline_b_out" "$expected_json")
    pass_l1=$(contains_expectations "$memory_context $task_context" "$expected_json")
    if [[ "$candidate_route" != "$expected_route" || "$executed_route" != "L1" ]]; then
      pass_l1="false"
    fi

    local min_hits
    min_hits=$(echo "$case_json" | jq -r '.benchmark_v2_1.min_entity_hits // empty')
    if [[ -n "$min_hits" && "$min_hits" != "null" ]]; then
      if [[ "${entity_hits:-0}" -lt "$min_hits" ]]; then
        echo "benchmark v2.1 failed: case $case_id entity_hits=$entity_hits < min_entity_hits=$min_hits" >&2
        exit 1
      fi
    fi

    local exp_a exp_b
    exp_a=$(echo "$case_json" | jq -r 'if (.benchmark_v2_1 | type) == "object" and (.benchmark_v2_1 | has("expect_lexical_full_pass")) then .benchmark_v2_1.expect_lexical_full_pass | tostring else "skip" end')
    exp_b=$(echo "$case_json" | jq -r 'if (.benchmark_v2_1 | type) == "object" and (.benchmark_v2_1 | has("expect_lexical_latest_pass")) then .benchmark_v2_1.expect_lexical_latest_pass | tostring else "skip" end')

    if [[ "$exp_a" != "skip" ]]; then
      local want_a
      want_a="$exp_a"
      if [[ "$want_a" == "true" && "$pass_a" != "true" ]]; then
        echo "benchmark v2.1 failed: case $case_id expected Lexical-Full pass=$want_a got $pass_a" >&2
        exit 1
      fi
      if [[ "$want_a" == "false" && "$pass_a" == "true" ]]; then
        echo "benchmark v2.1 failed: case $case_id expected Lexical-Full to FAIL but got pass" >&2
        exit 1
      fi
    fi
    if [[ "$exp_b" != "skip" ]]; then
      local want_b
      want_b="$exp_b"
      if [[ "$want_b" == "true" && "$pass_b" != "true" ]]; then
        echo "benchmark v2.1 failed: case $case_id expected Lexical-Latest pass=$want_b got $pass_b" >&2
        exit 1
      fi
      if [[ "$want_b" == "false" && "$pass_b" == "true" ]]; then
        echo "benchmark v2.1 failed: case $case_id expected Lexical-Latest to FAIL but got pass" >&2
        exit 1
      fi
    fi

    suite_total=$((suite_total + 1))
    [[ "$pass_l1" == "true" ]] && suite_l1=$((suite_l1 + 1))

    local l1_win_lexical
    l1_win_lexical="false"
    if [[ "$pass_l1" == "true" && "$pass_a" == "false" && "$pass_b" == "false" ]]; then
      l1_win_lexical="true"
      suite_l1_beats_both=$((suite_l1_beats_both + 1))
    fi

    echo "-- case: $case_id"
    echo "query: $query"
    echo "expected_contains: $(echo "$expected_json" | jq -c .)"
    echo "Lexical-FullTranscript (Baseline-A): output=$baseline_a_out pass=$pass_a"
    echo "Lexical-LatestMessage (Baseline-B): output=$baseline_b_out pass=$pass_b"
    echo "OneLink-L1: candidate_route=$candidate_route executed_route=$executed_route pass=$pass_l1 entity_hits=$entity_hits"
    echo "OneLink-L1 memory_context: $memory_context"
    echo ">>> VERDICT: L1=$( [[ "$pass_l1" == "true" ]] && echo WIN || echo LOSE ) | Lexical-Full=$( [[ "$pass_a" == "true" ]] && echo WIN || echo LOSE ) | Lexical-Latest=$( [[ "$pass_b" == "true" ]] && echo WIN || echo LOSE ) | L1-only-beat-both=$( [[ "$l1_win_lexical" == "true" ]] && echo YES || echo NO )"
    if [[ -n "$min_hits" && "$min_hits" != "null" ]]; then
      echo ">>> entity_hits check: $entity_hits >= $min_hits OK"
    fi
  done < <(jq -c '.cases[]' "$file")

  echo "suite summary | total=$suite_total | OneLink-L1 pass=$suite_l1/$suite_total | L1-only-beat-both-lexical=$suite_l1_beats_both"
  if [[ "$suite_l1" -ne "$suite_total" ]]; then
    echo "benchmark v2.1 failed: suite $suite_name has failing OneLink-L1 cases" >&2
    exit 1
  fi
}

if [[ ! -d "$DATA_DIR" ]]; then
  echo "benchmark v2.1 failed: data directory not found: $DATA_DIR" >&2
  exit 1
fi

echo "== benchmark v2.1 =="
echo "data_dir=$DATA_DIR"
echo "lexical_baselines: Baseline-A = Lexical-FullTranscript, Baseline-B = Lexical-LatestMessage (local scaffold, not production model)"
echo "See: repo/tests/integration/ASMR_LITE_BENCHMARK_V2.1.md"

run_suite "$DATA_DIR/l1_beats_lexical.json"
run_suite "$DATA_DIR/entity_observable.json"

echo "benchmark v2.1 passed"
