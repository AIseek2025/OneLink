#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${ONELINK_ROOT_DIR:-/opt/onelink}"
CURRENT_DIR="${ONELINK_CURRENT_DIR:-$ROOT_DIR/current}"
REPO_DIR="${ONELINK_REPO_DIR:-$CURRENT_DIR/repo}"
SHARED_DIR="${ONELINK_SHARED_DIR:-$ROOT_DIR/shared}"
ENV_FILE="${ONELINK_ENV_FILE:-$SHARED_DIR/onelink.env}"
WEB_ROOT="${ONELINK_WEB_ROOT:-/var/www/onelink/current}"
CERTBOT_ROOT="${ONELINK_CERTBOT_ROOT:-/var/www/certbot}"
NGINX_CONF="${ONELINK_NGINX_CONF:-/etc/nginx/conf.d/onelink.cool.conf}"

fail() {
  echo "[preflight] ERROR: $*" >&2
  exit 1
}

check_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "missing command: $1"
  echo "[preflight] command ok: $1"
}

require_file() {
  [[ -f "$1" ]] || fail "missing file: $1"
  echo "[preflight] file ok: $1"
}

require_dir() {
  [[ -d "$1" ]] || fail "missing dir: $1"
  echo "[preflight] dir ok: $1"
}

require_env_key() {
  local key="$1"
  grep -Eq "^${key}=" "$ENV_FILE" || fail "missing env key in $ENV_FILE: $key"
}

echo "[preflight] checking commands"
for cmd in nginx certbot docker node npm cargo rustc curl rsync; do
  check_cmd "$cmd"
done

echo "[preflight] checking directories"
require_dir "$CURRENT_DIR"
require_dir "$REPO_DIR"
require_dir "$SHARED_DIR"

echo "[preflight] checking deploy assets"
require_file "$REPO_DIR/deploy/ecs/systemd/onelink@.service"
require_file "$REPO_DIR/deploy/ecs/nginx/onelink.cool.http.conf"
require_file "$REPO_DIR/deploy/ecs/nginx/onelink.cool.https.conf"

echo "[preflight] checking env file"
require_file "$ENV_FILE"
for key in ONELINK_ENV INTERNAL_SHARED_SECRET DATABASE_URL IDENTITY_SERVICE_BASE_URL PROFILE_SERVICE_BASE_URL AI_CHAT_SERVICE_BASE_URL QUESTION_SERVICE_BASE_URL MATCH_SERVICE_BASE_URL SAFETY_SERVICE_BASE_URL DM_SERVICE_BASE_URL BFF_BASE_URL APP_PORT CORS_ALLOWED_ORIGINS; do
  require_env_key "$key"
done

grep -q '^ONELINK_ENV=production$' "$ENV_FILE" || fail "ONELINK_ENV must be production"
grep -q 'change-me' "$ENV_FILE" && fail "env file still contains placeholder value: change-me"
grep -q 'dev-only-shared-secret' "$ENV_FILE" && fail "env file still uses dev-only shared secret"

echo "[preflight] checking writable dirs"
mkdir -p "$WEB_ROOT" "$CERTBOT_ROOT" /var/log/onelink "$SHARED_DIR/services"
[[ -w "$WEB_ROOT" || -w "$(dirname "$WEB_ROOT")" ]] || echo "[preflight] note: web root may require sudo"

echo "[preflight] checking nginx syntax"
sudo nginx -t >/dev/null
echo "[preflight] nginx syntax ok"

if [[ -f "$NGINX_CONF" ]]; then
  echo "[preflight] note: existing nginx conf detected: $NGINX_CONF"
else
  echo "[preflight] note: nginx conf not installed yet: $NGINX_CONF"
fi

echo "[preflight] completed successfully"
