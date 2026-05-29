#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
WORKSPACE_ROOT="$(cd "$REPO_ROOT/.." && pwd)"

SUMMARY_MD="${ONELINK_LOCAL_PRELAUNCH_SUITE_SUMMARY:-$WORKSPACE_ROOT/reports/prelaunch/platform/local_prelaunch_suite_summary.md}"
SUMMARY_JSON="${ONELINK_LOCAL_PRELAUNCH_SUITE_JSON:-$WORKSPACE_ROOT/reports/prelaunch/platform/local_prelaunch_suite_summary.json}"

mkdir -p "$(dirname "$SUMMARY_MD")"
chmod +x \
  "$REPO_ROOT/scripts/prelaunch/run_local_preprod_collector.sh" \
  "$REPO_ROOT/scripts/prelaunch/run_local_bff_smoke.sh" \
  "$REPO_ROOT/scripts/prelaunch/run_local_app_server_smoke.sh"

cat <<EOF
[local-prelaunch-suite] 说明:
- 顺序执行 local collector、local bff smoke、local app-server smoke
- 输出文件:
  - $SUMMARY_JSON
  - $SUMMARY_MD
EOF

cd "$WORKSPACE_ROOT"

"$REPO_ROOT/scripts/prelaunch/run_local_preprod_collector.sh"
"$REPO_ROOT/scripts/prelaunch/run_local_bff_smoke.sh"
"$REPO_ROOT/scripts/prelaunch/run_local_app_server_smoke.sh"

export WORKSPACE_ROOT SUMMARY_MD SUMMARY_JSON
python3 - <<'PY'
import json
import os
from pathlib import Path

workspace_root = Path(os.environ["WORKSPACE_ROOT"])
summary_md = Path(os.environ["SUMMARY_MD"])
summary_json = Path(os.environ["SUMMARY_JSON"])

collector_path = workspace_root / "reports" / "prelaunch" / "platform" / "raw" / "preprod_evidence_collection.local.json"
bff_md_path = workspace_root / "reports" / "prelaunch" / "platform" / "api_smoke_report.local.md"
app_md_path = workspace_root / "reports" / "prelaunch" / "platform" / "app_server_smoke.local.md"
app_json_path = workspace_root / "reports" / "prelaunch" / "platform" / "app_server_smoke.local.json"

collector = json.loads(collector_path.read_text(encoding="utf-8"))
app_smoke = json.loads(app_json_path.read_text(encoding="utf-8"))

bff_cases = []
for line in bff_md_path.read_text(encoding="utf-8").splitlines():
    if not line.startswith("| ") or line.startswith("| Case ") or line.startswith("| ---"):
        continue
    parts = [item.strip() for item in line.strip("|").split("|")]
    if len(parts) >= 3:
        bff_cases.append({"name": parts[0], "status": parts[1], "notes": parts[2]})

app_cases = []
for item in app_smoke.get("transcript") or []:
    response = item.get("response") or {}
    label = str(item.get("label") or "")
    status = int(response.get("status") or 0)
    expected_status = 403 if "blocked" in label.lower() else 200
    app_cases.append(
        {
            "name": label,
            "status": status,
            "expected_status": expected_status,
        }
    )

collector_status = collector.get("collection_status") or "unknown"
bff_pass = all(item.get("status") == "pass" for item in bff_cases)
app_pass = all(item.get("status") == item.get("expected_status") for item in app_cases)

payload = {
    "kind": "onelink_local_prelaunch_suite_summary",
    "workspace_root": str(workspace_root),
    "collector_status": collector_status,
    "collector_path": str(collector_path),
    "bff_smoke_status": "pass" if bff_pass else "blocked",
    "bff_smoke_path": str(bff_md_path),
    "bff_case_count": len(bff_cases),
    "app_server_smoke_status": "pass" if app_pass else "blocked",
    "app_server_smoke_md_path": str(app_md_path),
    "app_server_smoke_json_path": str(app_json_path),
    "app_server_case_count": len(app_cases),
    "suite_status": "pass" if collector_status == "pass" and bff_pass and app_pass else "blocked",
}

summary_json.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

lines = [
    "# Local Prelaunch Suite Summary",
    "",
    f"- suite_status: `{payload['suite_status']}`",
    f"- collector_status: `{payload['collector_status']}`",
    f"- bff_smoke_status: `{payload['bff_smoke_status']}`",
    f"- app_server_smoke_status: `{payload['app_server_smoke_status']}`",
    "",
    "## Artifacts",
    "",
    f"- collector: `{payload['collector_path']}`",
    f"- bff_smoke: `{payload['bff_smoke_path']}`",
    f"- app_server_smoke_md: `{payload['app_server_smoke_md_path']}`",
    f"- app_server_smoke_json: `{payload['app_server_smoke_json_path']}`",
    "",
    "## Coverage",
    "",
    f"- bff_case_count: `{payload['bff_case_count']}`",
    f"- app_server_case_count: `{payload['app_server_case_count']}`",
]

summary_md.write_text("\n".join(lines) + "\n", encoding="utf-8")
PY
