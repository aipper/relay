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
  - RUST_LOG (default: info)

Usage:
  bash scripts/docker-init.sh
  bash scripts/docker-init.sh --admin admin --env docker/server.env
  bash scripts/docker-init.sh --up

Options:
  --env <path>     Env file path (default: docker/server.env)
  --admin <name>   Admin username (default: admin)
  --up             Run 'docker compose up -d --build' after writing env
  --container-name <name> Set container_name in docker-compose.override.yml (default: relay-server)
  --network <name> Attach relay-server to an external docker network (e.g. caddy)
  --no-create-network Do not create the external network if missing
  --rust-log <level> Set RUST_LOG (default: info)
  --no-ports        Do not publish 8787 to the host (internal Docker networking only; recommended with --network)
  --publish-ports   Publish 8787:8787 to the host (default if no --network is set)
EOF
}

ENV_PATH="$ROOT/docker/server.env"
ADMIN_USERNAME="admin"
DO_UP="0"
OVERRIDE_PATH="$ROOT/docker-compose.override.yml"
CONTAINER_NAME="relay-server"
EXTERNAL_NET_NAME=""
CREATE_NET_IF_MISSING="1"
RUST_LOG_LEVEL="${RUST_LOG_LEVEL:-info}"
PUBLISH_PORTS=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) usage; exit 0 ;;
    --env) ENV_PATH="${2:-}"; shift 2 ;;
    --admin) ADMIN_USERNAME="${2:-}"; shift 2 ;;
    --up) DO_UP="1"; shift ;;
    --container-name) CONTAINER_NAME="${2:-}"; shift 2 ;;
    --network) EXTERNAL_NET_NAME="${2:-}"; shift 2 ;;
    --no-create-network) CREATE_NET_IF_MISSING="0"; shift ;;
    --rust-log) RUST_LOG_LEVEL="${2:-}"; shift 2 ;;
    --publish-ports) PUBLISH_PORTS="1"; shift ;;
    --no-ports) PUBLISH_PORTS="0"; shift ;;
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
set_kv "RUST_LOG" "$RUST_LOG_LEVEL"

echo "[docker-init] set JWT_SECRET, ADMIN_USERNAME, and RUST_LOG"
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
  compose run --rm --entrypoint /app/relay-server relay-server --hash-password "$ADMIN_PASSWORD"
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

prompt() {
  local var="$1"
  local label="$2"
  local default="${3:-}"
  local val=""
  if [[ -n "$default" ]]; then
    read -r -p "$label [$default]: " val
    val="${val:-$default}"
  else
    read -r -p "$label: " val
  fi
  printf -v "$var" '%s' "$val"
}

prompt_yn() {
  local var="$1"
  local label="$2"
  local default="${3:-N}"
  local val=""
  read -r -p "$label [y/${default}]: " val
  val="${val:-$default}"
  case "${val}" in
    y|Y|yes|YES) printf -v "$var" '1' ;;
    n|N|no|NO) printf -v "$var" '0' ;;
    *) echo "invalid input: $val (expected y/n)" >&2; exit 2 ;;
  esac
}

echo "[docker-init] docker compose override (for reverse proxy network / container_name)"
if [[ -z "${EXTERNAL_NET_NAME}" ]]; then
  prompt EXTERNAL_NET_NAME "External docker network name (blank to skip)" ""
fi
if [[ -z "${CONTAINER_NAME}" ]]; then
  CONTAINER_NAME="relay-server"
fi

if [[ -z "$PUBLISH_PORTS" ]]; then
  # If an external proxy network is configured, default to internal-only (no host ports).
  # Otherwise default to publishing 8787 so the service remains reachable.
  if [[ -n "$EXTERNAL_NET_NAME" ]]; then
    PUBLISH_PORTS="0"
  else
    PUBLISH_PORTS="1"
  fi
fi

if [[ "$PUBLISH_PORTS" != "0" && "$PUBLISH_PORTS" != "1" ]]; then
  echo "invalid ports mode: $PUBLISH_PORTS (expected 0/1)" >&2
  exit 2
fi

if [[ -z "${EXTERNAL_NET_NAME}" && "$PUBLISH_PORTS" == "0" ]]; then
  echo "error: internal-only mode requires an external network (e.g. your caddy network)" >&2
  echo "hint: re-run and set --network caddy (or answer the network prompt), or use --publish-ports" >&2
  exit 2
fi

if [[ -n "$EXTERNAL_NET_NAME" ]]; then
  if ! docker network inspect "$EXTERNAL_NET_NAME" >/dev/null 2>&1; then
    if [[ "$CREATE_NET_IF_MISSING" == "1" ]]; then
      echo "[docker-init] creating external network: $EXTERNAL_NET_NAME"
      docker network create "$EXTERNAL_NET_NAME" >/dev/null
    else
      echo "[docker-init] warning: external network not found: $EXTERNAL_NET_NAME" >&2
      echo "hint: create it with: docker network create $EXTERNAL_NET_NAME" >&2
    fi
  fi
fi

{
  echo "services:"
  echo "  relay-server:"
  echo "    container_name: ${CONTAINER_NAME}"
  if [[ "$PUBLISH_PORTS" == "0" ]]; then
    echo "    ports: []"
  fi
  if [[ -n "$EXTERNAL_NET_NAME" ]]; then
    echo "    networks:"
    echo "      - default"
    echo "      - ${EXTERNAL_NET_NAME}"
  fi
  if [[ -n "$EXTERNAL_NET_NAME" ]]; then
    echo ""
    echo "networks:"
    echo "  ${EXTERNAL_NET_NAME}:"
    echo "    external: true"
  fi
} >"$OVERRIDE_PATH"

echo "[docker-init] wrote: $OVERRIDE_PATH"
if [[ -n "$EXTERNAL_NET_NAME" ]]; then
  echo "[docker-init] relay-server will also join external network: $EXTERNAL_NET_NAME"
fi
if [[ "$PUBLISH_PORTS" == "0" ]]; then
  echo "[docker-init] ports are disabled (internal Docker networking only)"
fi

if [[ "$DO_UP" == "1" ]]; then
  echo "[docker-init] starting..."
  compose up -d --build
  echo "[docker-init] ok"
fi
