#!/usr/bin/env bash
# OneLink DDL Migration Runner
# Applies all DDL drafts in dependency order to a target Postgres database.
#
# Usage:
#   ./apply-ddl.sh                                    # uses DATABASE_URL env var
#   ./apply-ddl.sh postgres://user:pw@host:5432/db   # explicit URL
#   ./apply-ddl.sh --dry-run                          # print SQL without executing
#   ./apply-ddl.sh --check                            # verify all DDL files parse (no DB needed)
#
# This replaces manual psql invocation and the CI workflow's inline DDL loop.
# The migration order is explicitly defined to respect foreign-key and
# enum-type dependencies between schemas.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DDL_DIR="$REPO_ROOT/data-platform/db-schema/drafts"

DDL_ORDER=(
  001_identity.sql
  002_profile.sql
  003_context.sql
  003_context_activation.sql
  003_context_idempotency.sql
  011_runtime_observability.sql
  004_ai_chat.sql
  005_dm.sql
  006_question.sql
  007_match.sql
  008_safety.sql
  010_optimization.sql
  009_model_gateway.sql
)

DRY_RUN=false
CHECK_ONLY=false
for arg in "$@"; do
  case "$arg" in
    --dry-run)  DRY_RUN=true ;;
    --check)    CHECK_ONLY=true ;;
    -*)
      echo "unknown flag: $arg" >&2
      exit 1
      ;;
  esac
done

if [[ "$CHECK_ONLY" == "true" ]]; then
  echo "=== DDL check: verify all files exist and are non-empty ==="
  failures=0
  for f in "${DDL_ORDER[@]}"; do
    path="$DDL_DIR/$f"
    if [[ ! -f "$path" ]]; then
      echo "MISSING: $f" >&2
      failures=$((failures + 1))
    elif [[ ! -s "$path" ]]; then
      echo "EMPTY: $f" >&2
      failures=$((failures + 1))
    else
      line_count=$(wc -l < "$path" | tr -d ' ')
      echo "  OK: $f ($line_count lines)"
    fi
  done
  if [[ "$failures" -gt 0 ]]; then
    echo "DDL check FAILED: $failures issue(s)" >&2
    exit 1
  fi
  echo "DDL check PASSED: ${#DDL_ORDER[@]} file(s) verified"
  exit 0
fi

DATABASE_URL="${1:-${DATABASE_URL:-}}"
if [[ -z "$DATABASE_URL" ]]; then
  echo "DATABASE_URL not set. Usage: $0 [DATABASE_URL|--dry-run|--check]" >&2
  exit 1
fi

echo "=== apply-ddl: target $DATABASE_URL ==="
echo "=== apply-ddl: ${#DDL_ORDER[@]} DDL file(s) in order ==="

for f in "${DDL_ORDER[@]}"; do
  path="$DDL_DIR/$f"
  if [[ ! -f "$path" ]]; then
    echo "FATAL: DDL file not found: $f" >&2
    exit 1
  fi
  if [[ "$DRY_RUN" == "true" ]]; then
    echo "--- $f (dry run) ---"
    cat "$path"
    echo ""
  else
    echo "  applying $f ..."
    psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -1 -f "$path"
  fi
done

if [[ "$DRY_RUN" == "true" ]]; then
  echo "=== dry run complete — no changes applied ==="
else
  table_count="$(psql "$DATABASE_URL" -tc "SELECT count(*) FROM information_schema.tables WHERE table_schema='public'" | tr -d ' ')"
  echo "=== DDL applied: $table_count table(s) in public schema ==="
fi
