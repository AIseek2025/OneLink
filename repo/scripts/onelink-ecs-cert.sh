#!/usr/bin/env bash
set -euo pipefail

EMAIL="${ONELINK_CERT_EMAIL:-admin@onelink.cool}"
DOMAIN="${ONELINK_DOMAIN:-onelink.cool}"
CERTBOT_ROOT="${ONELINK_CERTBOT_ROOT:-/var/www/certbot}"
CURRENT_DIR="${ONELINK_CURRENT_DIR:-/opt/onelink/current}"
REPO_DIR="${ONELINK_REPO_DIR:-$CURRENT_DIR/repo}"
HTTPS_CONF_SRC="$REPO_DIR/deploy/ecs/nginx/onelink.cool.https.conf"
NGINX_CONF="${ONELINK_NGINX_CONF:-/etc/nginx/conf.d/onelink.cool.conf}"

fail() {
  echo "[cert] ERROR: $*" >&2
  exit 1
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "missing command: $1"
}

require_cmd certbot
require_cmd nginx
require_cmd dig

sudo mkdir -p "$CERTBOT_ROOT"

echo "[cert] checking DNS"
resolved="$(dig +short "$DOMAIN" | tail -n1 || true)"
[[ -n "$resolved" ]] || fail "domain does not resolve yet: $DOMAIN"
echo "[cert] $DOMAIN resolves to $resolved"

echo "[cert] requesting certificate"
sudo certbot certonly \
  --webroot -w "$CERTBOT_ROOT" \
  -d "$DOMAIN" \
  --non-interactive \
  --agree-tos \
  -m "$EMAIL"

echo "[cert] installing https nginx config"
sudo cp "$HTTPS_CONF_SRC" "$NGINX_CONF"
sudo nginx -t
sudo systemctl reload nginx

echo "[cert] done"
