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
HTTP_CONF_SRC="$REPO_DIR/deploy/ecs/nginx/onelink.cool.http.conf"
HTTPS_CONF_SRC="$REPO_DIR/deploy/ecs/nginx/onelink.cool.https.conf"
SYSTEMD_TEMPLATE_SRC="$REPO_DIR/deploy/ecs/systemd/onelink@.service"
SYSTEMD_TEMPLATE_DST="/etc/systemd/system/onelink@.service"
SERVICE_ENV_DIR="$SHARED_DIR/services"

PACKAGES=(
  identity-service
  profile-service
  ai-chat-service
  question-service
  match-service
  safety-service
  dm-service
  model-gateway
  context-service
  bff
  onelink-app-server
  onelink-migration-runner
)

RUNTIME_SERVICES=(
  identity-service
  profile-service
  ai-chat-service
  question-service
  match-service
  safety-service
  dm-service
  model-gateway
  context-service
  bff
  onelink-app-server
)

fail() {
  echo "[deploy] ERROR: $*" >&2
  exit 1
}

require_file() {
  [[ -f "$1" ]] || fail "missing file: $1"
}

write_service_envs() {
  mkdir -p "$SERVICE_ENV_DIR"

  cat >"$SERVICE_ENV_DIR/identity-service.env" <<'EOF'
PORT=18101
EOF
  cat >"$SERVICE_ENV_DIR/profile-service.env" <<'EOF'
PORT=18102
EOF
  cat >"$SERVICE_ENV_DIR/ai-chat-service.env" <<'EOF'
PORT=18105
EOF
  cat >"$SERVICE_ENV_DIR/question-service.env" <<'EOF'
PORT=18106
EOF
  cat >"$SERVICE_ENV_DIR/match-service.env" <<'EOF'
PORT=18107
EOF
  cat >"$SERVICE_ENV_DIR/safety-service.env" <<'EOF'
PORT=18108
EOF
  cat >"$SERVICE_ENV_DIR/dm-service.env" <<'EOF'
PORT=18109
EOF
  cat >"$SERVICE_ENV_DIR/model-gateway.env" <<'EOF'
PORT=18110
EOF
  cat >"$SERVICE_ENV_DIR/context-service.env" <<'EOF'
PORT=18111
EOF
  cat >"$SERVICE_ENV_DIR/bff.env" <<'EOF'
PORT=18113
EOF
  cat >"$SERVICE_ENV_DIR/onelink-app-server.env" <<'EOF'
APP_PORT=18121
EOF
}

install_nginx_conf() {
  local src
  if [[ -f /etc/letsencrypt/live/onelink.cool/fullchain.pem && -f /etc/letsencrypt/live/onelink.cool/privkey.pem ]]; then
    src="$HTTPS_CONF_SRC"
  else
    src="$HTTP_CONF_SRC"
  fi
  sudo cp "$src" "$NGINX_CONF"
  sudo nginx -t
  sudo systemctl reload nginx
}

build_web() {
  echo "[deploy] building web"
  cd "$REPO_DIR/apps/web"
  npm ci
  npm run build
  sudo mkdir -p "$WEB_ROOT"
  sudo rsync -a --delete dist/ "$WEB_ROOT/"
  sudo find "$WEB_ROOT" -type d -exec chmod 755 {} \;
  sudo find "$WEB_ROOT" -type f -exec chmod 644 {} \;
}

build_rust() {
  echo "[deploy] building rust packages"
  cd "$REPO_DIR"
  local args=()
  for pkg in "${PACKAGES[@]}"; do
    args+=(-p "$pkg")
  done
  cargo build --release "${args[@]}"
}

run_migrations() {
  echo "[deploy] running migrations"
  set -a
  # shellcheck disable=SC1090
  source "$ENV_FILE"
  set +a
  cargo run --release -p onelink-migration-runner -- "$DATABASE_URL"
}

install_systemd() {
  echo "[deploy] installing systemd template"
  sudo cp "$SYSTEMD_TEMPLATE_SRC" "$SYSTEMD_TEMPLATE_DST"
  sudo systemctl daemon-reload
}

restart_services() {
  echo "[deploy] restarting onelink services"
  for svc in "${RUNTIME_SERVICES[@]}"; do
    sudo systemctl enable "onelink@${svc}.service" >/dev/null
    sudo systemctl restart "onelink@${svc}.service"
  done
}

check_health() {
  echo "[deploy] checking local health"
  curl -fsS http://127.0.0.1:18113/health >/dev/null
  curl -fsS http://127.0.0.1:18113/ready >/dev/null
  curl -fsS http://127.0.0.1:18121/api/v1/bff/health >/dev/null
  curl -fsS http://127.0.0.1:18121/api/v1/bff/ready >/dev/null
  echo "[deploy] local health ok"
}

echo "[deploy] validating inputs"
require_file "$ENV_FILE"
require_file "$HTTP_CONF_SRC"
require_file "$HTTPS_CONF_SRC"
require_file "$SYSTEMD_TEMPLATE_SRC"
grep -q '^ONELINK_ENV=production$' "$ENV_FILE" || fail "ONELINK_ENV must be production"
grep -q 'change-me' "$ENV_FILE" && fail "env file still contains placeholder values"
grep -q 'dev-only-shared-secret' "$ENV_FILE" && fail "env file still uses dev-only secret"

echo "[deploy] preparing directories"
sudo mkdir -p "$ROOT_DIR/releases" "$SHARED_DIR" "$SERVICE_ENV_DIR" "$WEB_ROOT" "$CERTBOT_ROOT" /var/log/onelink

write_service_envs
build_web
build_rust
cd "$REPO_DIR"
run_migrations
install_systemd
restart_services
install_nginx_conf
check_health

echo "[deploy] completed successfully"
