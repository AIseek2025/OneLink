#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
WORKSPACE_ROOT="$(cd "$REPO_ROOT/.." && pwd)"

LOCAL_PREPROD_URL="${ONELINK_LOCAL_PREPROD_URL:-http://127.0.0.1:8083}"
LOCAL_WEB_URL="${ONELINK_LOCAL_PREPROD_WEB_URL:-file://$REPO_ROOT/apps/web/dist/index.html}"
LOCAL_BFF_BASE_URL="${ONELINK_LOCAL_PREPROD_BFF_BASE_URL:-http://127.0.0.1:8083/api/v1/bff}"
LOCAL_API_BASE_URL="${ONELINK_LOCAL_PREPROD_API_BASE_URL:-http://127.0.0.1:8083}"
PACKAGE_VERSION="$(
  export REPO_ROOT
  python3 - <<'PY'
import json
import os
from pathlib import Path
pkg = Path(os.environ["REPO_ROOT"]) / "apps" / "mobile" / "package.json"
try:
    payload = json.loads(pkg.read_text(encoding="utf-8"))
    print(payload.get("version", "0.0.0-local"))
except Exception:
    print("0.0.0-local")
PY
)"

LOCAL_CANDIDATE_VERSION="${ONELINK_LOCAL_PREPROD_VERSION:-local-shared-env-${PACKAGE_VERSION}}"
LOCAL_IMAGE_DIGEST="${ONELINK_LOCAL_PREPROD_IMAGE_DIGEST:-local-worktree-no-digest}"
LOCAL_ASSET_MANIFEST="${ONELINK_LOCAL_PREPROD_ASSET_MANIFEST:-{\"mode\":\"temporary_local_shared_env\",\"web_entry\":\"$LOCAL_WEB_URL\",\"bff_base\":\"$LOCAL_BFF_BASE_URL\"}}"
LOCAL_DASHBOARD_URL="${ONELINK_LOCAL_PREPROD_DASHBOARD_URL:-file://$WORKSPACE_ROOT/reports/prelaunch/platform/metrics_and_alerts.md}"
LOCAL_ALERT_URL="${ONELINK_LOCAL_PREPROD_ALERT_URL:-file://$WORKSPACE_ROOT/reports/prelaunch/platform/metrics_and_alerts.md}"
LOCAL_APPROVAL_URL="${ONELINK_LOCAL_PREPROD_APPROVAL_URL:-file://$WORKSPACE_ROOT/reports/prelaunch/platform/release_runbook.md}"
LOCAL_RELEASE_OWNER="${ONELINK_LOCAL_PREPROD_RELEASE_OWNER:-codemaster-local-owner}"
LOCAL_ROLLBACK_OWNER="${ONELINK_LOCAL_PREPROD_ROLLBACK_OWNER:-codemaster-local-owner}"
LOCAL_SMOKE_USER="${ONELINK_LOCAL_PREPROD_SMOKE_USER:-local-smoke-user-placeholder}"
LOCAL_SMOKE_PASSWORD="${ONELINK_LOCAL_PREPROD_SMOKE_PASSWORD:-local-smoke-password-placeholder}"

OUTPUT_JSON="$WORKSPACE_ROOT/reports/prelaunch/platform/raw/preprod_evidence_collection.local.json"
OUTPUT_MD="$WORKSPACE_ROOT/reports/prelaunch/platform/raw/preprod_evidence_collection.local.md"

cat <<EOF
[local-preprod] 说明:
- 这是一套临时 local shared-env 输入，不等价于真实 pre-prod.
- 输出文件:
  - $OUTPUT_JSON
  - $OUTPUT_MD
- 预期用途:
  - 先把 collector 从 "全部 missing" 推进到 "本地可探测 / 可复跑"
  - 不用于关闭正式 pre-prod blocker
- 如需覆盖默认值，请使用 ONELINK_LOCAL_PREPROD_* 变量，而不是正式 ONELINK_PREPROD_* 变量
EOF

export ONELINK_PREPROD_URL="$LOCAL_PREPROD_URL"
export ONELINK_PREPROD_WEB_URL="$LOCAL_WEB_URL"
export ONELINK_PREPROD_BFF_BASE_URL="$LOCAL_BFF_BASE_URL"
export ONELINK_PREPROD_API_BASE_URL="$LOCAL_API_BASE_URL"
export ONELINK_PREPROD_VERSION="$LOCAL_CANDIDATE_VERSION"
export ONELINK_PREPROD_IMAGE_DIGEST="$LOCAL_IMAGE_DIGEST"
export ONELINK_PREPROD_ASSET_MANIFEST="$LOCAL_ASSET_MANIFEST"
export ONELINK_PREPROD_DASHBOARD_URL="$LOCAL_DASHBOARD_URL"
export ONELINK_PREPROD_ALERT_URL="$LOCAL_ALERT_URL"
export ONELINK_PREPROD_APPROVAL_URL="$LOCAL_APPROVAL_URL"
export ONELINK_PREPROD_RELEASE_OWNER="$LOCAL_RELEASE_OWNER"
export ONELINK_PREPROD_ROLLBACK_OWNER="$LOCAL_ROLLBACK_OWNER"
export ONELINK_PREPROD_SMOKE_USER="$LOCAL_SMOKE_USER"
export ONELINK_PREPROD_SMOKE_PASSWORD="$LOCAL_SMOKE_PASSWORD"

cd "$WORKSPACE_ROOT"
python3 repo/scripts/prelaunch/collect_preprod_evidence.py \
  --workspace-root "$WORKSPACE_ROOT" \
  --output-json "$OUTPUT_JSON" \
  --output-md "$OUTPUT_MD"
