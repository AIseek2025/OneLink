#!/usr/bin/env bash
# ASMR-Lite benchmark v2:
# - 固定小数据集
# - 两类样本：Memory QA / Temporal & Update
# - 三路输出（命名映射，避免与「生产级 baseline」混淆）：
#     Baseline-A = Lexical-FullTranscript（全量 setup 拼接 + 本地 if/contains 规则）
#     Baseline-B = Lexical-LatestMessage（仅最后一条 setup）
#     OneLink-L1 = context-service /internal/context/build（当前确定性 L1）
# - 歧视性样本与 entity_hits 强断言见 benchmark-asmr-lite-v2.1.sh
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
DATA_DIR="${ASMR_BENCHMARK_V2_DATA_DIR:-$SCRIPT_DIR/../tests/integration/asmr_benchmark_v2}"
INTERNAL_SHARED_SECRET="${INTERNAL_SHARED_SECRET:-onelink-dev-internal-token}"
PASS="dev-password"

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
  local email="benchmark-v2-$(date +%s)-$RANDOM@example.com"
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
    -d "{\"user_id\":\"$user_id\",\"agent_id\":\"$agent_id\",\"conversation_id\":\"$conv\",\"input\":$(jq -Rn --arg v "$query" '$v'),\"task_type\":\"chat\",\"max_tokens\":8000,\"memory_limit\":6,\"summary_limit\":3,\"reply_style\":\"brief\",\"trace_id\":\"benchmark-v2-trace-$RANDOM\",\"retrieval_modes\":[\"structured\",\"temporal\"]}"
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
    echo "benchmark v2 failed: last_observation missing query-level or preference fields" >&2
    exit 1
  fi
}

run_suite() {
  local file="$1"
  local suite_name
  suite_name=$(jq -r '.suite' "$file")
  local setup_blob
  setup_blob=$(jq -c '.setup_messages' "$file")
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
  agent_id="benchmark-v2-agent-$RANDOM"

  if [[ -z "$token" || "$token" == "null" || -z "$conv" || "$conv" == "null" ]]; then
    echo "benchmark v2 failed: unable to initialize suite $suite_name" >&2
    echo "$reg" | jq .
    echo "$init" | jq .
    exit 1
  fi

  local index=0
  while IFS= read -r message; do
    send_message "$token" "$conv" "$message" "benchmark-v2-$suite_name-setup-$index"
    index=$((index + 1))
    sleep 1
  done < <(jq -r '.setup_messages[]' "$file")
  sleep 2

  local suite_total=0
  local suite_a=0
  local suite_b=0
  local suite_l1=0

  while IFS= read -r case_json; do
    local case_id query expected_json expected_route
    case_id=$(echo "$case_json" | jq -r '.id')
    query=$(echo "$case_json" | jq -r '.query')
    expected_json=$(echo "$case_json" | jq -c '.expected_contains')
    expected_route=$(echo "$case_json" | jq -r '.expected_candidate_route')

    local build obs memory_context task_context candidate_route executed_route
    build=$(build_context "$user_id" "$conv" "$query" "$agent_id")
    obs=$(read_observability)
    assert_observation_shape "$obs"

    memory_context=$(echo "$build" | jq -r '.memory_context')
    task_context=$(echo "$build" | jq -r '.task_context')
    candidate_route=$(echo "$obs" | jq -r '.routing.last_observation.candidate_route')
    executed_route=$(echo "$obs" | jq -r '.routing.last_observation.executed_route')

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

    suite_total=$((suite_total + 1))
    [[ "$pass_a" == "true" ]] && suite_a=$((suite_a + 1))
    [[ "$pass_b" == "true" ]] && suite_b=$((suite_b + 1))
    [[ "$pass_l1" == "true" ]] && suite_l1=$((suite_l1 + 1))

    echo "-- case: $case_id"
    echo "query: $query"
    echo "expected: $(echo "$expected_json" | jq -c .)"
    echo "Lexical-FullTranscript (Baseline-A): $baseline_a_out | pass=$pass_a"
    echo "Lexical-LatestMessage (Baseline-B): $baseline_b_out | pass=$pass_b"
    echo "OneLink-L1 route: candidate=$candidate_route executed=$executed_route | pass=$pass_l1"
    echo "OneLink-L1 memory_context: $memory_context"
    echo "OneLink-L1 task_context: $task_context"
    echo ">>> VERDICT: L1=$( [[ "$pass_l1" == "true" ]] && echo WIN || echo LOSE ) | Lexical-Full=$( [[ "$pass_a" == "true" ]] && echo WIN || echo LOSE ) | Lexical-Latest=$( [[ "$pass_b" == "true" ]] && echo WIN || echo LOSE )"
  done < <(jq -c '.cases[]' "$file")

  echo "suite score | Baseline-A=$suite_a/$suite_total | Baseline-B=$suite_b/$suite_total | OneLink-L1=$suite_l1/$suite_total"
  if [[ "$suite_l1" -ne "$suite_total" ]]; then
    echo "benchmark v2 failed: suite $suite_name has failing OneLink-L1 cases" >&2
    exit 1
  fi
}

if [[ ! -d "$DATA_DIR" ]]; then
  echo "benchmark v2 failed: data directory not found: $DATA_DIR" >&2
  exit 1
fi

echo "== benchmark v2 =="
echo "data_dir=$DATA_DIR"
echo "baselines=Baseline-A(Lexical-Full),Baseline-B(Lexical-Latest),OneLink-L1"
echo "tip: 跑 v2.1 以验证 L1 > 本地 lexical 与 entity_hits 断言 — scripts/benchmark-asmr-lite-v2.1.sh"

run_suite "$DATA_DIR/memory_qa.json"
run_suite "$DATA_DIR/temporal_update.json"

echo "benchmark v2 passed"
