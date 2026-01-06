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
  --reset-password  Prompt for a new admin password and overwrite ADMIN_PASSWORD_HASH
  --rotate-jwt      Generate a new JWT_SECRET (will invalidate existing tokens)
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
RESET_PASSWORD="0"
ROTATE_JWT="0"
ADMIN_USERNAME_SET="0"
CONTAINER_NAME_SET="0"
EXTERNAL_NET_NAME_SET="0"
CREATE_NET_SET="0"
RUST_LOG_SET="0"
PUBLISH_PORTS_SET="0"
DO_UP_SET="0"

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) usage; exit 0 ;;
    --env) ENV_PATH="${2:-}"; shift 2 ;;
    --admin) ADMIN_USERNAME="${2:-}"; ADMIN_USERNAME_SET="1"; shift 2 ;;
    --up) DO_UP="1"; DO_UP_SET="1"; shift ;;
    --container-name) CONTAINER_NAME="${2:-}"; CONTAINER_NAME_SET="1"; shift 2 ;;
    --network) EXTERNAL_NET_NAME="${2:-}"; EXTERNAL_NET_NAME_SET="1"; shift 2 ;;
    --no-create-network) CREATE_NET_IF_MISSING="0"; CREATE_NET_SET="1"; shift ;;
    --rust-log) RUST_LOG_LEVEL="${2:-}"; RUST_LOG_SET="1"; shift 2 ;;
    --publish-ports) PUBLISH_PORTS="1"; PUBLISH_PORTS_SET="1"; shift ;;
    --no-ports) PUBLISH_PORTS="0"; PUBLISH_PORTS_SET="1"; shift ;;
    --reset-password) RESET_PASSWORD="1"; shift ;;
    --rotate-jwt) ROTATE_JWT="1"; shift ;;
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

fail() {
  echo "error: $*" >&2
  exit 1
}

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
    *) fail "invalid input: $val (expected y/n)" ;;
  esac
}

prompt_secret_confirm() {
  local var="$1"
  local label="$2"
  local v1=""
  local v2=""
  read -r -s -p "$label: " v1
  echo ""
  read -r -s -p "Confirm: " v2
  echo ""
  [[ -n "$v1" ]] || fail "empty input"
  [[ "$v1" == "$v2" ]] || fail "inputs do not match"
  printf -v "$var" '%s' "$v1"
}

read_env_value() {
  local key="$1"
  local file="$2"
  [[ -f "$file" ]] || return 1
  awk -v k="$key" -F= '$0 ~ ("^"k"=") {sub("^"k"=",""); print; exit}' "$file"
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

EXISTING_JWT_SECRET="$(read_env_value "JWT_SECRET" "$ENV_PATH" || true)"
EXISTING_ADMIN_USERNAME="$(read_env_value "ADMIN_USERNAME" "$ENV_PATH" || true)"
EXISTING_ADMIN_PASSWORD_HASH="$(read_env_value "ADMIN_PASSWORD_HASH" "$ENV_PATH" || true)"
EXISTING_RUST_LOG="$(read_env_value "RUST_LOG" "$ENV_PATH" || true)"

HAS_EXISTING_PASSWORD_HASH="0"
if [[ -n "$EXISTING_ADMIN_PASSWORD_HASH" ]]; then
  HAS_EXISTING_PASSWORD_HASH="1"
fi

if [[ "$ADMIN_USERNAME_SET" != "1" ]]; then
  prompt ADMIN_USERNAME "Admin username" "${EXISTING_ADMIN_USERNAME:-$ADMIN_USERNAME}"
fi
if [[ "$RUST_LOG_SET" != "1" ]]; then
  prompt RUST_LOG_LEVEL "RUST_LOG" "${EXISTING_RUST_LOG:-$RUST_LOG_LEVEL}"
fi
if [[ "$EXTERNAL_NET_NAME_SET" != "1" ]]; then
  prompt EXTERNAL_NET_NAME "External docker network name (blank to skip)" "$EXTERNAL_NET_NAME"
fi
if [[ "$CONTAINER_NAME_SET" != "1" ]]; then
  prompt CONTAINER_NAME "Container name" "$CONTAINER_NAME"
fi

if [[ "$PUBLISH_PORTS_SET" != "1" ]]; then
  if [[ -n "$EXTERNAL_NET_NAME" ]]; then
    prompt_yn PUBLISH_PORTS "Publish 8787 to host?" "N"
  else
    prompt_yn PUBLISH_PORTS "Publish 8787 to host?" "y"
  fi
fi

if [[ -n "$EXTERNAL_NET_NAME" && "$CREATE_NET_SET" != "1" ]]; then
  prompt_yn CREATE_NET_IF_MISSING "Create external network if missing?" "y"
fi

if [[ "$DO_UP_SET" != "1" ]]; then
  prompt_yn DO_UP "Run docker compose up after init?" "y"
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

WILL_ROTATE_JWT="0"
if [[ "$ROTATE_JWT" == "1" || -z "$EXISTING_JWT_SECRET" ]]; then
  WILL_ROTATE_JWT="1"
fi

WILL_RESET_PASSWORD_HASH="0"
if [[ "$RESET_PASSWORD" == "1" || "$HAS_EXISTING_PASSWORD_HASH" != "1" ]]; then
  WILL_RESET_PASSWORD_HASH="1"
fi

if [[ "$WILL_RESET_PASSWORD_HASH" == "1" ]]; then
  echo "[docker-init] enter a new admin password to generate ADMIN_PASSWORD_HASH"
  prompt_secret_confirm ADMIN_PASSWORD "Admin password"
fi

if [[ -z "${EXTERNAL_NET_NAME}" && "$PUBLISH_PORTS" == "0" ]]; then
  echo "error: internal-only mode requires an external network (e.g. your caddy network)" >&2
  echo "hint: re-run and set --network caddy (or answer the network prompt), or use --publish-ports" >&2
  exit 2
fi

echo ""
echo "[docker-init] summary"
echo "  env:            $ENV_PATH"
echo "  admin username: $ADMIN_USERNAME"
echo "  RUST_LOG:       $RUST_LOG_LEVEL"
echo "  container_name: $CONTAINER_NAME"
echo "  network:        ${EXTERNAL_NET_NAME:-<none>}"
if [[ "$PUBLISH_PORTS" == "1" ]]; then
  echo "  ports:          publish"
else
  echo "  ports:          internal-only"
fi
if [[ "$DO_UP" == "1" ]]; then
  echo "  up:             yes"
else
  echo "  up:             no"
fi
if [[ "$WILL_ROTATE_JWT" == "1" ]]; then
  echo "  JWT_SECRET:     generate new"
else
  echo "  JWT_SECRET:     keep existing"
fi
if [[ "$WILL_RESET_PASSWORD_HASH" == "1" ]]; then
  echo "  admin password: set/update"
else
  echo "  admin password: keep existing hash"
fi
prompt_yn CONFIRM "Proceed?" "y"
[[ "$CONFIRM" == "1" ]] || fail "aborted"

if [[ "$WILL_ROTATE_JWT" == "1" ]]; then
  JWT_SECRET="$(bash "$ROOT/scripts/gen-jwt-secret.sh")"
  set_kv "JWT_SECRET" "$JWT_SECRET"
fi
set_kv "ADMIN_USERNAME" "$ADMIN_USERNAME"
set_kv "RUST_LOG" "$RUST_LOG_LEVEL"

if [[ "$WILL_RESET_PASSWORD_HASH" == "1" ]]; then
  echo "[docker-init] building image (needed to hash password)..."
  compose build relay-server

  ADMIN_PASSWORD_HASH="$(
    compose run --rm --entrypoint /app/relay-server relay-server --hash-password "$ADMIN_PASSWORD"
  )"
  [[ -n "$ADMIN_PASSWORD_HASH" ]] || fail "failed to derive ADMIN_PASSWORD_HASH"

  set_kv "ADMIN_PASSWORD_HASH" "$ADMIN_PASSWORD_HASH"

  # Best-effort: remove any plaintext ADMIN_PASSWORD line if present.
  tmp="$(mktemp)"
  awk 'BEGIN{FS="="} $0 ~ "^ADMIN_PASSWORD[=]" {next} {print}' "$ENV_PATH" >"$tmp"
  install -m 0600 "$tmp" "$ENV_PATH"
  rm -f "$tmp"

  echo "[docker-init] wrote ADMIN_PASSWORD_HASH (removed ADMIN_PASSWORD if present)"
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
