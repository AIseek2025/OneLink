#!/usr/bin/env bash
set -euo pipefail

CAPACITY_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$CAPACITY_DIR/../.." && pwd)"
RESULTS_DIR="$REPO_ROOT/reports/capacity_results"
mkdir -p "$RESULTS_DIR"

MODEL_GATEWAY_URL="${MODEL_GATEWAY_URL:-http://127.0.0.1:8090}"
IDENTITY_URL="${IDENTITY_URL:-http://127.0.0.1:8081}"
BFF_URL="${BFF_URL:-http://127.0.0.1:8083}"
INTERNAL_TOKEN="${INTERNAL_SHARED_SECRET:-onelink-dev-internal-token}"
TIMESTAMP="$(date +%Y%m%d_%H%M%S)"

echo "=== OneLink Fault Injection ==="
echo "timestamp=$TIMESTAMP"
echo "model_gateway_url=$MODEL_GATEWAY_URL"
echo "identity_url=$IDENTITY_URL"
echo "bff_url=$BFF_URL"
echo ""

write_result() {
    local name="$1"
    local description="$2"
    local expected_degradation="$3"
    local status_code="$4"
    local latency_ms="$5"
    local actual_degradation="$6"
    local response_file="$7"
    local extra_json="${8:-{}}"
    local result_file="$RESULTS_DIR/${TIMESTAMP}_fault_${name}.json"

    python3 - "$name" "$description" "$expected_degradation" "$status_code" "$latency_ms" "$actual_degradation" "$TIMESTAMP" "$MODEL_GATEWAY_URL" "$response_file" "$extra_json" "$result_file" <<'PY'
import json
import pathlib
import sys

name, description, expected, status_code, latency_ms, actual, timestamp, base_url, response_file, extra_json, result_file = sys.argv[1:]
response_path = pathlib.Path(response_file)
response_text = response_path.read_text() if response_path.exists() else ""
payload = {
    "fault_name": name,
    "description": description,
    "expected_degradation": expected,
    "actual_degradation": actual,
    "status_code": int(status_code),
    "latency_ms": int(latency_ms),
    "recovery_steps": "inspect /ready, confirm fallback path, restart or wait for breaker recovery window",
    "timestamp": timestamp,
    "base_url": base_url,
    "response_excerpt": response_text[:400],
}
payload.update(json.loads(extra_json))
pathlib.Path(result_file).write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n")
print(json.dumps(payload, ensure_ascii=False))
PY
    echo "  result=$result_file"
}

invoke_gateway_fault() {
    local name="$1"
    local simulated_error="$2"
    local expected_degradation="$3"
    local response_file
    response_file="$(mktemp)"
    local output
    output="$(curl -sS -o "$response_file" -w "%{http_code} %{time_total}" \
        -X POST "$MODEL_GATEWAY_URL/internal/v1/invoke" \
        -H "Content-Type: application/json" \
        -H "x-internal-token: $INTERNAL_TOKEN" \
        -d "{\"capability_name\":\"chat.respond\",\"simulate_provider_error\":\"$simulated_error\",\"payload\":{\"context\":{\"case\":\"$name\"}}}")"
    local status_code
    local time_total
    status_code="$(echo "$output" | awk '{print $1}')"
    time_total="$(echo "$output" | awk '{print $2}')"
    local latency_ms
    latency_ms="$(python3 - "$time_total" <<'PY'
import sys
print(int(round(float(sys.argv[1]) * 1000)))
PY
)"
    local actual_degradation
    actual_degradation="$(python3 - "$response_file" <<'PY'
import json
import pathlib
import sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
if payload.get("degraded") and payload.get("fallback_reason") == "provider_error":
    print("provider_error_fallback")
else:
    print("unexpected_response")
PY
)"
    write_result "$name" "Simulated provider fault via internal invoke" "$expected_degradation" "$status_code" "$latency_ms" "$actual_degradation" "$response_file"
    rm -f "$response_file"
}

echo "=== Provider Fault Injections ==="
echo ""

invoke_gateway_fault "provider_timeout" "timeout" "fallback_response_with_degraded_marking"
invoke_gateway_fault "provider_rate_limit" "rate_limit" "fallback_response_with_degraded_marking"

response_file="$(mktemp)"
output="$(curl -sS -o "$response_file" -w "%{http_code} %{time_total}" \
    -X POST "$MODEL_GATEWAY_URL/internal/v1/invoke" \
    -H "Content-Type: application/json" \
    -H "x-internal-token: $INTERNAL_TOKEN" \
    -d '{"capability_name":"chat.respond","estimated_tokens":999999999,"payload":{"context":{"case":"budget_exceeded"}}}')"
status_code="$(echo "$output" | awk '{print $1}')"
time_total="$(echo "$output" | awk '{print $2}')"
latency_ms="$(python3 - "$time_total" <<'PY'
import sys
print(int(round(float(sys.argv[1]) * 1000)))
PY
)"
actual_degradation="$(python3 - "$response_file" <<'PY'
import json
import pathlib
import sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
if payload.get("degraded") and payload.get("fallback_reason") == "token_budget_exceeded":
    print("budget_fallback")
else:
    print("unexpected_response")
PY
)"
write_result \
    "budget_exceeded" \
    "Oversized token request should trigger budget fallback" \
    "fallback_reason_token_budget_exceeded" \
    "$status_code" \
    "$latency_ms" \
    "$actual_degradation" \
    "$response_file"
rm -f "$response_file"

echo ""
echo "--- Fault Injection: provider_5xx ---"
response_file="$(mktemp)"
for _ in 1 2 3 4 5; do
    curl -sS -o /dev/null \
        -X POST "$MODEL_GATEWAY_URL/internal/v1/invoke" \
        -H "Content-Type: application/json" \
        -H "x-internal-token: $INTERNAL_TOKEN" \
        -d '{"capability_name":"chat.respond","simulate_provider_error":"provider_5xx","payload":{"context":{"case":"provider_5xx"}}}' >/dev/null
done
output="$(curl -sS -o "$response_file" -w "%{http_code} %{time_total}" "$MODEL_GATEWAY_URL/ready")"
status_code="$(echo "$output" | awk '{print $1}')"
time_total="$(echo "$output" | awk '{print $2}')"
latency_ms="$(python3 - "$time_total" <<'PY'
import sys
print(int(round(float(sys.argv[1]) * 1000)))
PY
)"
actual_degradation="$(python3 - "$response_file" <<'PY'
import json
import pathlib
import sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
if payload.get("status") == "degraded":
    print("circuit_breaker_open")
else:
    print("unexpected_ready_state")
PY
)"
write_result \
    "provider_5xx" \
    "Repeated simulated provider 5xx should open the chat.respond circuit" \
    "ready_endpoint_reports_degraded" \
    "$status_code" \
    "$latency_ms" \
    "$actual_degradation" \
    "$response_file"
rm -f "$response_file"

echo ""
echo "=== Downstream Degradation ==="
echo ""

echo "--- Checking all downstream /ready endpoints during fault state ---"
for svc_url in "$IDENTITY_URL" "$BFF_URL" "http://127.0.0.1:8082" "http://127.0.0.1:8084" "http://127.0.0.1:8085" "http://127.0.0.1:8086" "http://127.0.0.1:8087" "http://127.0.0.1:8088" "http://127.0.0.1:8089"; do
    svc_name="$(echo "$svc_url" | sed 's|http://127.0.0.1:||')"
    for endpoint in /health /ready /metrics; do
        curl_out="$(curl -sS -o /dev/null -w "%{http_code}" "$svc_url$endpoint" 2>/dev/null || echo "000")"
        echo "  $svc_name$endpoint => $curl_out"
    done
done

EMAIL="fault-$(date +%s)@example.com"
PASS="dev-password"
TOKEN="$(curl -sS -X POST "$IDENTITY_URL/api/v1/identity/register" \
    -H "Content-Type: application/json" \
    -d "{\"provider\":\"email\",\"email\":\"$EMAIL\",\"password\":\"$PASS\",\"primary_region\":\"CN\",\"primary_language\":\"zh\"}" \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["session"]["token"])')"
response_file="$(mktemp)"
output="$(curl -sS -o "$response_file" -w "%{http_code} %{time_total}" \
    "$BFF_URL/api/v1/bff/recommendations" \
    -H "Authorization: Bearer $TOKEN")"
status_code="$(echo "$output" | awk '{print $1}')"
time_total="$(echo "$output" | awk '{print $2}')"
latency_ms="$(python3 - "$time_total" <<'PY'
import sys
print(int(round(float(sys.argv[1]) * 1000)))
PY
)"
actual_degradation="$(python3 - "$response_file" <<'PY'
import json
import pathlib
import sys
payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
if payload.get("degraded") and payload.get("reason") == "match_service_unavailable":
    print("downstream_degraded_response")
else:
    print("unexpected_response")
PY
)"
write_result \
    "match_service_unavailable" \
    "BFF recommendations should degrade when match-service recommendations are unavailable" \
    "degraded_response_with_reason_match_service_unavailable" \
    "$status_code" \
    "$latency_ms" \
    "$actual_degradation" \
    "$response_file"
rm -f "$response_file"

echo "=== Fault Injection Complete ==="
echo "results_dir=$RESULTS_DIR"
echo "timestamp=$TIMESTAMP"
