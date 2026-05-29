#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
WORKSPACE_ROOT="$(cd "$REPO_ROOT/.." && pwd)"
CARGO_BIN="${CARGO_BIN:-/Users/brando/.cargo/bin/cargo}"

OUTPUT_JSON="${ONELINK_LOCAL_APP_SERVER_SMOKE_JSON:-$WORKSPACE_ROOT/reports/prelaunch/platform/app_server_smoke.local.json}"
OUTPUT_MD="${ONELINK_LOCAL_APP_SERVER_SMOKE_MD:-$WORKSPACE_ROOT/reports/prelaunch/platform/app_server_smoke.local.md}"

mkdir -p "$(dirname "$OUTPUT_JSON")"

cat <<EOF
[local-app-server-smoke] 说明:
- 该脚本运行 app-server 自带 smoke_compliance
- 输出文件:
  - $OUTPUT_JSON
  - $OUTPUT_MD
EOF

cd "$REPO_ROOT"
"$CARGO_BIN" run -p onelink-app-server --bin smoke_compliance >"$OUTPUT_JSON"

export OUTPUT_JSON OUTPUT_MD
python3 - <<'PY'
import json
import os
from pathlib import Path

output_json = Path(os.environ["OUTPUT_JSON"])
output_md = Path(os.environ["OUTPUT_MD"])
payload = json.loads(output_json.read_text(encoding="utf-8"))

topology = payload.get("topology") or {}
transcript = payload.get("transcript") or []
upstream_log = payload.get("upstream_request_log") or {}

lines = [
    "# App Server Smoke Report",
    "",
    f"- smoke_test: `{payload.get('smoke_test') or 'unknown'}`",
    f"- verification_mode: `{payload.get('verification_mode') or 'unknown'}`",
    f"- timestamp: `{payload.get('timestamp') or 'unknown'}`",
    f"- total_requests: `{payload.get('total_requests') or 0}`",
    "",
    "## Topology",
    "",
    f"- identity_service: `{topology.get('identity_service') or 'unknown'}`",
    f"- profile_service: `{topology.get('profile_service') or 'unknown'}`",
    f"- bff_service: `{topology.get('bff_service') or 'unknown'}`",
    f"- app_server: `{topology.get('app_server') or 'unknown'}`",
    "",
    "## Transcript",
    "",
    "| Label | Status | Response |",
    "| --- | --- | --- |",
]

for item in transcript:
    label = str(item.get("label") or "")
    response = item.get("response") or {}
    status = response.get("status")
    body = json.dumps(response.get("body"), ensure_ascii=False, sort_keys=True)
    lines.append(f"| {label} | {status} | {body} |")

lines.extend(
    [
        "",
        "## Upstream Coverage",
        "",
        "| Service | Requests |",
        "| --- | --- |",
        f"| identity_service | {len(upstream_log.get('identity_service') or [])} |",
        f"| profile_service | {len(upstream_log.get('profile_service') or [])} |",
    ]
)

output_md.write_text("\n".join(lines) + "\n", encoding="utf-8")
PY
