#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

usage() {
  cat <<'EOF'
Initialize docker/server.env for relay-server (VPS).

Creates/updates docker/server.env with:
  - JWT_SECRET (random)
  - ADMIN_USERNAME (default: admin)
  - ADMIN_PASSWORD_HASH (derived from an interactive password prompt)

Usage:
  bash scripts/docker-init.sh
  bash scripts/docker-init.sh --admin admin --env docker/server.env
  bash scripts/docker-init.sh --up

Options:
  --env <path>     Env file path (default: docker/server.env)
  --admin <name>   Admin username (default: admin)
  --up             Run 'docker compose up -d --build' after writing env
EOF
}

ENV_PATH="$ROOT/docker/server.env"
ADMIN_USERNAME="admin"
DO_UP="0"

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) usage; exit 0 ;;
    --env) ENV_PATH="${2:-}"; shift 2 ;;
    --admin) ADMIN_USERNAME="${2:-}"; shift 2 ;;
    --up) DO_UP="1"; shift ;;
    *) echo "unknown arg: $1" >&2; usage; exit 2 ;;
  esac
done

if [[ -z "$ENV_PATH" ]]; then
  echo "--env requires a value" >&2
  exit 2
fi

if [[ -z "$ADMIN_USERNAME" ]]; then
  echo "--admin requires a value" >&2
  exit 2
fi

need() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing dependency: $1" >&2
    exit 1
  fi
}

compose() {
  if command -v docker >/dev/null 2>&1; then
    if docker compose version >/dev/null 2>&1; then
      docker compose "$@"
      return
    fi
  fi
  if command -v docker-compose >/dev/null 2>&1; then
    docker-compose "$@"
    return
  fi
  echo "missing dependency: docker compose" >&2
  exit 1
}

mask_line() {
  local k="$1"
  local v="$2"
  printf '%s=%s\n' "$k" "$v"
}

set_kv() {
  local key="$1"
  local value="$2"
  local tmp
  tmp="$(mktemp)"
  if [[ -f "$ENV_PATH" ]]; then
    # Remove existing key lines (KEY=... or KEY: ...), keep others.
    awk -v k="$key" 'BEGIN{FS="="} $0 ~ "^"k"[=]" {next} {print}' "$ENV_PATH" >"$tmp"
  else
    cp "$ROOT/docker/server.env.example" "$tmp"
  fi
  {
    echo ""
    mask_line "$key" "$value"
  } >>"$tmp"
  install -m 0600 "$tmp" "$ENV_PATH"
  rm -f "$tmp"
}

echo "[docker-init] env: $ENV_PATH"

need bash
need awk

JWT_SECRET="$(bash "$ROOT/scripts/gen-jwt-secret.sh")"
set_kv "JWT_SECRET" "$JWT_SECRET"
set_kv "ADMIN_USERNAME" "$ADMIN_USERNAME"

echo "[docker-init] set JWT_SECRET and ADMIN_USERNAME"
echo "[docker-init] enter a new admin password to generate ADMIN_PASSWORD_HASH"
read -r -s -p "Admin password: " ADMIN_PASSWORD
echo ""
read -r -s -p "Confirm password: " ADMIN_PASSWORD_CONFIRM
echo ""

if [[ -z "$ADMIN_PASSWORD" ]]; then
  echo "error: empty password" >&2
  exit 2
fi
if [[ "$ADMIN_PASSWORD" != "$ADMIN_PASSWORD_CONFIRM" ]]; then
  echo "error: passwords do not match" >&2
  exit 2
fi

echo "[docker-init] building image (needed to hash password)..."
compose build relay-server

ADMIN_PASSWORD_HASH="$(
  compose run --rm --entrypoint /app/relay-server relay-server -- --hash-password "$ADMIN_PASSWORD"
)"
if [[ -z "$ADMIN_PASSWORD_HASH" ]]; then
  echo "error: failed to derive ADMIN_PASSWORD_HASH" >&2
  exit 1
fi

set_kv "ADMIN_PASSWORD_HASH" "$ADMIN_PASSWORD_HASH"

# Best-effort: remove any plaintext ADMIN_PASSWORD line if present.
tmp="$(mktemp)"
awk 'BEGIN{FS="="} $0 ~ "^ADMIN_PASSWORD[=]" {next} {print}' "$ENV_PATH" >"$tmp"
install -m 0600 "$tmp" "$ENV_PATH"
rm -f "$tmp"

echo "[docker-init] wrote ADMIN_PASSWORD_HASH (removed ADMIN_PASSWORD if present)"

if [[ "$DO_UP" == "1" ]]; then
  echo "[docker-init] starting..."
  compose up -d --build
  echo "[docker-init] ok"
fi

