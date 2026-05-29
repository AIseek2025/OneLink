#!/usr/bin/env bash
set -euo pipefail

CAPACITY_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$CAPACITY_DIR/../.." && pwd)"
RESULTS_DIR="$REPO_ROOT/reports/capacity_results"
mkdir -p "$RESULTS_DIR"

SAMPLES="${SAMPLES:-5}"
WARMUP_SECS="${WARMUP_SECS:-1}"
INTERNAL_TOKEN="${INTERNAL_SHARED_SECRET:-onelink-dev-internal-token}"
TIMESTAMP="$(date +%Y%m%d_%H%M%S)"

IDENTITY_URL="${IDENTITY_URL:-http://127.0.0.1:8081}"
PROFILE_URL="${PROFILE_URL:-http://127.0.0.1:8082}"
BFF_URL="${BFF_URL:-http://127.0.0.1:8083}"
SAFETY_URL="${SAFETY_URL:-http://127.0.0.1:8084}"
AI_CHAT_URL="${AI_CHAT_URL:-http://127.0.0.1:8085}"
QUESTION_URL="${QUESTION_URL:-http://127.0.0.1:8086}"
MATCH_URL="${MATCH_URL:-http://127.0.0.1:8087}"
DM_URL="${DM_URL:-http://127.0.0.1:8088}"
CONTEXT_URL="${CONTEXT_URL:-http://127.0.0.1:8089}"
MODEL_GATEWAY_URL="${MODEL_GATEWAY_URL:-http://127.0.0.1:8090}"

echo "=== OneLink Capacity Benchmark ==="
echo "timestamp=$TIMESTAMP"
echo "samples=$SAMPLES"
echo "warmup_secs=$WARMUP_SECS"
echo "identity_url=$IDENTITY_URL"
echo "profile_url=$PROFILE_URL"
echo "bff_url=$BFF_URL"
echo "safety_url=$SAFETY_URL"
echo "ai_chat_url=$AI_CHAT_URL"
echo "question_url=$QUESTION_URL"
echo "match_url=$MATCH_URL"
echo "dm_url=$DM_URL"
echo "context_url=$CONTEXT_URL"
echo "model_gateway_url=$MODEL_GATEWAY_URL"
echo ""

sleep "$WARMUP_SECS"

run_case() {
    local name="$1"
    local url="$2"
    local expected_codes="$3"
    local auth_mode="${4:-none}"
    local result_file="$RESULTS_DIR/${TIMESTAMP}_${name}.json"
    local latency_file
    latency_file="$(mktemp)"
    local status_file
    status_file="$(mktemp)"
    trap 'rm -f "$latency_file" "$status_file"' RETURN

    echo "--- Benchmark: $name ---"
    echo "  url=$url expected=$expected_codes"

    local i
    for i in $(seq 1 "$SAMPLES"); do
        local curl_args=("-sS" "-o" "/dev/null" "-w" "%{http_code} %{time_total}")
        if [ "$auth_mode" = "internal" ]; then
            curl_args+=(-H "x-internal-token: $INTERNAL_TOKEN")
        fi
        local output
        output="$(curl "${curl_args[@]}" "$url" 2>/dev/null || echo "000 0")"
        local status_code
        local time_total
        status_code="$(echo "$output" | awk '{print $1}')"
        time_total="$(echo "$output" | awk '{print $2}')"
        printf '%s\n' "$status_code" >>"$status_file"
        python3 - "$time_total" >>"$latency_file" <<'PY'
import sys
print(int(round(float(sys.argv[1]) * 1000)))
PY
    done

    python3 - "$name" "$url" "$expected_codes" "$TIMESTAMP" "$SAMPLES" "$result_file" "$latency_file" "$status_file" <<'PY'
import json
import math
import pathlib
import statistics
import sys

name, url, expected_codes, timestamp, samples, result_file, latency_file, status_file = sys.argv[1:]
expected = {code.strip() for code in expected_codes.split(",") if code.strip()}
latencies = [int(line.strip()) for line in pathlib.Path(latency_file).read_text().splitlines() if line.strip()]
statuses = [line.strip() for line in pathlib.Path(status_file).read_text().splitlines() if line.strip()]
successes = sum(1 for code in statuses if code in expected)

def percentile(values, pct):
    if not values:
        return None
    ordered = sorted(values)
    idx = max(0, min(len(ordered) - 1, math.ceil((pct / 100.0) * len(ordered)) - 1))
    return ordered[idx]

payload = {
    "benchmark": name,
    "timestamp": timestamp,
    "sample_count": int(samples),
    "url": url,
    "expected_http_codes": sorted(expected),
    "statuses": statuses,
    "success_count": successes,
    "error_count": len(statuses) - successes,
    "success_rate": round(successes / len(statuses), 4) if statuses else 0.0,
    "latency_ms": {
        "min": min(latencies) if latencies else None,
        "p50": percentile(latencies, 50),
        "p95": percentile(latencies, 95),
        "p99": percentile(latencies, 99),
        "max": max(latencies) if latencies else None,
        "avg": round(statistics.fmean(latencies), 2) if latencies else None,
    },
}
pathlib.Path(result_file).write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")
print(json.dumps(payload, ensure_ascii=False))
PY
    echo "  result=$result_file"
    echo ""

    rm -f "$latency_file" "$status_file"
    trap - RETURN
}

run_case "identity_health" "$IDENTITY_URL/health" "200"
run_case "identity_ready" "$IDENTITY_URL/ready" "200"
run_case "identity_metrics" "$IDENTITY_URL/metrics" "200"
run_case "identity_internal_health_detail" "$IDENTITY_URL/internal/identity/health-detail" "200" "internal"
run_case "profile_health" "$PROFILE_URL/health" "200"
run_case "profile_ready" "$PROFILE_URL/ready" "200"
run_case "profile_metrics" "$PROFILE_URL/metrics" "200"
run_case "bff_health" "$BFF_URL/health" "200"
run_case "bff_ready" "$BFF_URL/ready" "200"
run_case "bff_metrics" "$BFF_URL/metrics" "200"
run_case "ai_chat_health" "$AI_CHAT_URL/health" "200"
run_case "ai_chat_ready" "$AI_CHAT_URL/ready" "200"
run_case "ai_chat_metrics" "$AI_CHAT_URL/metrics" "200"
run_case "question_health" "$QUESTION_URL/health" "200"
run_case "question_ready" "$QUESTION_URL/ready" "200"
run_case "question_metrics" "$QUESTION_URL/metrics" "200"
run_case "context_health" "$CONTEXT_URL/health" "200"
run_case "context_ready" "$CONTEXT_URL/ready" "200"
run_case "context_metrics" "$CONTEXT_URL/metrics" "200"
run_case "match_health" "$MATCH_URL/health" "200"
run_case "match_ready" "$MATCH_URL/ready" "200"
run_case "match_metrics" "$MATCH_URL/metrics" "200"
run_case "match_internal_health_detail" "$MATCH_URL/internal/match/health-detail" "200" "internal"
run_case "dm_health" "$DM_URL/health" "200"
run_case "dm_ready" "$DM_URL/ready" "200"
run_case "dm_metrics" "$DM_URL/metrics" "200"
run_case "dm_internal_health_detail" "$DM_URL/internal/dm/health-detail" "200" "internal"
run_case "safety_health" "$SAFETY_URL/health" "200"
run_case "safety_ready" "$SAFETY_URL/ready" "200"
run_case "safety_metrics" "$SAFETY_URL/metrics" "200"
run_case "safety_internal_health_detail" "$SAFETY_URL/internal/safety/health-detail" "200" "internal"
run_case "model_gateway_health" "$MODEL_GATEWAY_URL/health" "200"
run_case "model_gateway_ready" "$MODEL_GATEWAY_URL/ready" "200"
run_case "model_gateway_metrics" "$MODEL_GATEWAY_URL/metrics" "200"

echo "=== Capacity Benchmark Complete ==="
echo "results_dir=$RESULTS_DIR"
echo "timestamp=$TIMESTAMP"
