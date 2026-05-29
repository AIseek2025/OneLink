#!/usr/bin/env bash
# ASMR-Lite benchmark v2.1:
# - 不替代 v1 / v2：本脚本只跑 v2.1 增补套件（歧视性样本 + entity 断言 + query 侧极性）
# - 对照组命名（与 v2 脚本内函数一致）：
#     Baseline-A  = Lexical-FullTranscript（全量 setup 拼接词法规则）
#     Baseline-B  = Lexical-LatestMessage（仅最后一条 setup）
# - OneLink-L1  = context-service POST /internal/context/build + observability
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ASMR_BENCH_RUN_TAG="benchmark-v21"
ASMR_BENCH_SHAPE_ERR="benchmark v2.1 failed"
export ASMR_BENCH_RUN_TAG ASMR_BENCH_SHAPE_ERR
# shellcheck source=lib/asmr-benchmark-v2-common.sh
source "$SCRIPT_DIR/lib/asmr-benchmark-v2-common.sh"

require_cmd curl
require_cmd jq

DATA_DIR="${ASMR_BENCHMARK_V2_1_DATA_DIR:-$SCRIPT_DIR/../tests/integration/asmr_benchmark_v2_1}"
INTERNAL_SHARED_SECRET="${INTERNAL_SHARED_SECRET:-onelink-dev-internal-token}"
PASS="dev-password"
export INTERNAL_SHARED_SECRET PASS

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
  agent_id="${ASMR_BENCH_RUN_TAG}-agent-$RANDOM"

  if [[ -z "$token" || "$token" == "null" || -z "$conv" || "$conv" == "null" ]]; then
    echo "benchmark v2.1 failed: unable to initialize suite $suite_name" >&2
    exit 1
  fi

  local index=0
  while IFS= read -r message; do
    send_message "$token" "$conv" "$message" "${ASMR_BENCH_RUN_TAG}-$suite_name-setup-$index"
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

    local build obs memory_context task_context candidate_route executed_route entity_hits query_pref
    build=$(build_context "$user_id" "$conv" "$query" "$agent_id")
    obs=$(read_observability)
    assert_observation_shape "$obs"

    memory_context=$(echo "$build" | jq -r '.memory_context')
    task_context=$(echo "$build" | jq -r '.task_context')
    candidate_route=$(echo "$obs" | jq -r '.routing.last_observation.candidate_route')
    executed_route=$(echo "$obs" | jq -r '.routing.last_observation.executed_route')
    entity_hits=$(echo "$obs" | jq -r '.routing.last_observation.entity_hits')
    query_pref=$(echo "$obs" | jq -r '.routing.last_observation.query_preference_polarity')

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

    local exp_qp
    exp_qp=$(echo "$case_json" | jq -r '(.benchmark_v2_1 // {}) | .expect_query_preference_polarity // empty')
    if [[ -n "$exp_qp" && "$exp_qp" != "null" ]]; then
      if [[ "$query_pref" != "$exp_qp" ]]; then
        echo "benchmark v2.1 failed: case $case_id expected query_preference_polarity=$exp_qp got $query_pref" >&2
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
    echo "OneLink-L1: candidate_route=$candidate_route executed_route=$executed_route pass=$pass_l1 entity_hits=$entity_hits query_preference_polarity=$query_pref"
    echo "OneLink-L1 memory_context: $memory_context"
    echo ">>> VERDICT: L1=$( [[ "$pass_l1" == "true" ]] && echo WIN || echo LOSE ) | Lexical-Full=$( [[ "$pass_a" == "true" ]] && echo WIN || echo LOSE ) | Lexical-Latest=$( [[ "$pass_b" == "true" ]] && echo WIN || echo LOSE ) | L1-only-beat-both=$( [[ "$l1_win_lexical" == "true" ]] && echo YES || echo NO )"
    if [[ -n "$min_hits" && "$min_hits" != "null" ]]; then
      echo ">>> entity_hits check: $entity_hits >= $min_hits OK"
    fi
    if [[ -n "$exp_qp" && "$exp_qp" != "null" ]]; then
      echo ">>> query_preference_polarity check: $query_pref == $exp_qp OK"
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
run_suite "$DATA_DIR/query_polarity_open.json"

echo "benchmark v2.1 passed"
