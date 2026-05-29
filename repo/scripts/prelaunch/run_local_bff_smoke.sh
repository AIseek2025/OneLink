#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
WORKSPACE_ROOT="$(cd "$REPO_ROOT/.." && pwd)"

REPORT_MD="${ONELINK_LOCAL_BFF_SMOKE_REPORT:-$WORKSPACE_ROOT/reports/prelaunch/platform/api_smoke_report.local.md}"

cat <<EOF
[local-bff-smoke] 说明:
- 该脚本使用本地 mock downstream + 独立 bff 进程
- 用于生成临时 local business smoke 证据
- 输出文件:
  - $REPORT_MD
EOF

cd "$WORKSPACE_ROOT"
python3 repo/scripts/prelaunch_bff_smoke.py \
  --workspace-root "$WORKSPACE_ROOT" \
  --repo-root "$REPO_ROOT" \
  --report-md "$REPORT_MD"
