#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

usage() {
  cat <<'EOF'
relay client packaging helper

Builds release binaries for client-side usage (relay-hostd + relay CLI) and creates
a self-contained directory under ./dist/ with a hostd-up.sh script.

Usage:
  scripts/package-client.sh
EOF
}

need() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing dependency: $1" >&2
    exit 1
  fi
}

run() {
  echo "+ $*"
  "$@"
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

need cargo
need node

VERSION="$(cargo metadata --no-deps --format-version 1 | node -e 'const fs=require("fs");const m=JSON.parse(fs.readFileSync(0,"utf8"));const p=m.packages.find((x)=>x.name==="relay-hostd");process.stdout.write(p?.version||"0.0.0");')"
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
ARCH_ID="$ARCH"
case "$ARCH" in
  x86_64|amd64) ARCH_ID="x64" ;;
  aarch64|arm64) ARCH_ID="arm64" ;;
esac

PKG_DIR="$ROOT/dist/relay-client-$VERSION-$OS-$ARCH"
BIN_DIR="$PKG_DIR/bin"

run mkdir -p "$BIN_DIR"

echo "[package-client] building rust release binaries..."
run cargo build --release -p relay-hostd
run cargo build --release -p relay-cli

run cp "$ROOT/target/release/relay-hostd" "$BIN_DIR/"
run cp "$ROOT/target/release/relay" "$BIN_DIR/"

# Convenience: standalone artifacts for release hosting (used by `relay hostd install`).
run cp "$ROOT/target/release/relay-hostd" "$PKG_DIR/relay-hostd-$OS-$ARCH_ID"
run cp "$ROOT/target/release/relay" "$PKG_DIR/relay-$OS-$ARCH_ID"

run cp "$ROOT/scripts/install-shims.sh" "$PKG_DIR/install-shims.sh"

cat >"$PKG_DIR/client-init.sh" <<'CLIENT_INIT_EOF'
#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

usage() {
  cat <<'USAGE'
relay client init (Linux)

One-shot installer for relay-hostd + relay CLI on a client machine.
Supports:
  - systemd user service (default)
  - system-wide systemd service (--mode system)

Usage:
  ./client-init.sh
  ./client-init.sh --server http://<vps>:8787 --mode user
  sudo ./client-init.sh --mode system

Options:
  --server <url>     relay-server base URL (http(s)://...) for health check and WS base.
  --host-id <id>     Host identifier (default: host-<hostname>).
  --host-token <t>   Host token (optional; if omitted, generated and stored in the hostd config).
  --mode <m>         user|system (default: user).
  --sock <path>      Local unix socket path (default: user->~/.relay/relay-hostd.sock, system->/run/relay/relay-hostd.sock).
  --install-shims    Run ./install-shims.sh --auto-path after install (optional).
  --force            Overwrite existing installed binaries/units (use carefully).
USAGE
}

MODE="user"
SERVER_HTTP=""
HOST_ID=""
HOST_TOKEN="${HOST_TOKEN:-}"
SOCK_PATH=""
INSTALL_SHIMS="0"
FORCE="0"

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) usage; exit 0 ;;
    --server) SERVER_HTTP="${2:-}"; shift 2 ;;
    --host-id) HOST_ID="${2:-}"; shift 2 ;;
    --host-token) HOST_TOKEN="${2:-}"; shift 2 ;;
    --mode) MODE="${2:-}"; shift 2 ;;
    --sock) SOCK_PATH="${2:-}"; shift 2 ;;
    --install-shims) INSTALL_SHIMS="1"; shift ;;
    --force) FORCE="1"; shift ;;
    *) echo "unknown arg: $1" >&2; usage; exit 2 ;;
  esac
done

fail() {
  echo "error: $*" >&2
  exit 1
}

need() {
  command -v "$1" >/dev/null 2>&1 || fail "missing dependency: $1"
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

prompt_secret() {
  local var="$1"
  local label="$2"
  local val=""
  read -r -s -p "$label: " val
  echo ""
  printf -v "$var" '%s' "$val"
}

as_ws_base() {
  local u="$1"
  case "$u" in
    https://*) printf 'wss://%s' "${u#https://}" ;;
    http://*) printf 'ws://%s' "${u#http://}" ;;
    ws://*|wss://*) printf '%s' "$u" ;;
    *) return 1 ;;
  esac
}

check_health() {
  local u="$1"
  if ! command -v curl >/dev/null 2>&1; then
    echo "[client-init] warning: curl not found; skipping server health check" >&2
    return 0
  fi
  if ! curl -fsS "$u/health" >/dev/null 2>&1; then
    fail "server not reachable: $u/health (check URL, firewall, or reverse proxy)"
  fi
}

install_file() {
  local src="$1"
  local dst="$2"
  local mode="${3:-0755}"
  if [[ -e "$dst" && "$FORCE" != "1" ]]; then
    fail "destination exists: $dst (use --force to overwrite)"
  fi
  install -m "$mode" "$src" "$dst"
}

need bash
need install
need awk
need systemctl

case "$MODE" in
  user|system) ;;
  *) fail "--mode must be user|system (got: $MODE)" ;;
esac

if [[ -z "$SERVER_HTTP" ]]; then
  prompt SERVER_HTTP "relay-server URL (http(s)://host:port)" ""
fi
case "$SERVER_HTTP" in
  http://*|https://*) ;;
  *) fail "invalid --server: $SERVER_HTTP (expected http(s)://...)" ;;
esac

check_health "$SERVER_HTTP"

if [[ -z "$HOST_ID" ]]; then
  HOST_ID="host-$(hostname -s 2>/dev/null || hostname)"
fi

SERVER_BASE_URL="$(as_ws_base "$SERVER_HTTP")" || fail "failed to convert server URL to ws/wss: $SERVER_HTTP"

if [[ ! -x "$ROOT/bin/relay-hostd" ]]; then
  fail "missing binary: $ROOT/bin/relay-hostd"
fi
if [[ ! -x "$ROOT/bin/relay" ]]; then
  fail "missing binary: $ROOT/bin/relay"
fi

if [[ "$MODE" == "system" && "$(id -u)" -ne 0 ]]; then
  args=(--mode system --server "$SERVER_HTTP" --host-id "$HOST_ID")
  [[ -n "$HOST_TOKEN" ]] && args+=(--host-token "$HOST_TOKEN")
  [[ -n "$SOCK_PATH" ]] && args+=(--sock "$SOCK_PATH")
  [[ "$INSTALL_SHIMS" == "1" ]] && args+=(--install-shims)
  [[ "$FORCE" == "1" ]] && args+=(--force)
  exec sudo -k --preserve-env=PATH bash "$0" "${args[@]}"
fi

if [[ "$MODE" == "user" ]]; then
  if ! systemctl --user show-environment >/dev/null 2>&1; then
    fail "systemctl --user is not available (no user systemd session). Use '--mode system' or enable lingering."
  fi

  DATA_DIR="${HOME}/.relay"
  CONFIG_PATH="${DATA_DIR}/hostd.json"
  BIN_DIR="$DATA_DIR/bin"
  # Derive the unit dir from the *systemd user manager* environment when possible.
  # This avoids mismatches when the invoking shell has a different XDG_CONFIG_HOME.
  SYSTEMD_USER_CONFIG_HOME="$(systemctl --user show-environment 2>/dev/null | awk -F= '$1=="XDG_CONFIG_HOME"{print $2; exit}')"
  UNIT_BASE="${SYSTEMD_USER_CONFIG_HOME:-${XDG_CONFIG_HOME:-${HOME}/.config}}"
  UNIT_DIR="${UNIT_BASE%/}/systemd/user"
  mkdir -p "$DATA_DIR" "$BIN_DIR" "$UNIT_DIR"

  SOCK_PATH="${SOCK_PATH:-$DATA_DIR/relay-hostd.sock}"
  SPOOL_DB_PATH="${SPOOL_DB_PATH:-$DATA_DIR/hostd-spool.db}"
  HOSTD_LOG_PATH="${HOSTD_LOG_PATH:-$DATA_DIR/hostd.log}"
  RUST_LOG_LEVEL="${RUST_LOG:-warn}"
  PATH_ENV="${PATH:-}"

  install_file "$ROOT/bin/relay-hostd" "$BIN_DIR/relay-hostd" 0755
  install_file "$ROOT/bin/relay" "$BIN_DIR/relay" 0755

  {
    echo "ABRELAY_CONFIG=$CONFIG_PATH"
    echo "SERVER_BASE_URL=$SERVER_BASE_URL"
    echo "HOST_ID=$HOST_ID"
    [[ -n "$HOST_TOKEN" ]] && echo "HOST_TOKEN=$HOST_TOKEN"
    echo "LOCAL_UNIX_SOCKET=$SOCK_PATH"
    echo "SPOOL_DB_PATH=$SPOOL_DB_PATH"
    echo "HOSTD_LOG_PATH=$HOSTD_LOG_PATH"
    echo "RUST_LOG=$RUST_LOG_LEVEL"
    [[ -n "$PATH_ENV" ]] && echo "PATH=$PATH_ENV"
  } >"$DATA_DIR/hostd.env"
  chmod 0600 "$DATA_DIR/hostd.env"

  cat >"$UNIT_DIR/relay-hostd.service" <<EOF
[Unit]
Description=relay-hostd (user)
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
EnvironmentFile=%h/.relay/hostd.env
ExecStart=%h/.relay/bin/relay-hostd
Restart=always
RestartSec=2
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=full
ProtectHome=false

[Install]
WantedBy=default.target
EOF

  [[ -f "$UNIT_DIR/relay-hostd.service" ]] || fail "unit file not found: $UNIT_DIR/relay-hostd.service"
  systemctl --user daemon-reload
  if ! systemctl --user enable --now relay-hostd.service; then
    # Fallback #1: enable via path (systemctl supports absolute unit file paths).
    systemctl --user enable --now "$UNIT_DIR/relay-hostd.service" || true
    systemctl --user daemon-reload

    # Fallback #2: some environments ignore XDG_CONFIG_HOME for unit discovery; try ~/.config.
    if ! systemctl --user enable --now relay-hostd.service; then
      fallback_unit_dir="${HOME}/.config/systemd/user"
      if [[ "$fallback_unit_dir" != "$UNIT_DIR" ]]; then
        mkdir -p "$fallback_unit_dir"
        cp "$UNIT_DIR/relay-hostd.service" "$fallback_unit_dir/relay-hostd.service"
        systemctl --user daemon-reload
      fi
      systemctl --user enable --now relay-hostd.service
    fi
  fi

  echo "[ok] installed (user service)"
  echo "status: systemctl --user status relay-hostd"
  echo "logs:   journalctl --user -u relay-hostd -f"
  echo "sock:   $SOCK_PATH"
else
  # system mode (root)
  DATA_DIR="/var/lib/relay"
  RUN_DIR="/run/relay"
  ENV_DIR="/etc/relay"
  CONFIG_PATH="${DATA_DIR}/hostd.json"
  BIN_DIR="/usr/local/bin"
  UNIT_PATH="/etc/systemd/system/relay-hostd.service"
  mkdir -p "$DATA_DIR" "$RUN_DIR" "$ENV_DIR"

  SOCK_PATH="${SOCK_PATH:-$RUN_DIR/relay-hostd.sock}"
  SPOOL_DB_PATH="${SPOOL_DB_PATH:-$DATA_DIR/hostd-spool.db}"
  HOSTD_LOG_PATH="${HOSTD_LOG_PATH:-$DATA_DIR/hostd.log}"
  RUST_LOG_LEVEL="${RUST_LOG:-warn}"

  install_file "$ROOT/bin/relay-hostd" "$BIN_DIR/relay-hostd" 0755
  install_file "$ROOT/bin/relay" "$BIN_DIR/relay" 0755

  {
    echo "ABRELAY_CONFIG=$CONFIG_PATH"
    echo "SERVER_BASE_URL=$SERVER_BASE_URL"
    echo "HOST_ID=$HOST_ID"
    [[ -n "$HOST_TOKEN" ]] && echo "HOST_TOKEN=$HOST_TOKEN"
    echo "LOCAL_UNIX_SOCKET=$SOCK_PATH"
    echo "SPOOL_DB_PATH=$SPOOL_DB_PATH"
    echo "HOSTD_LOG_PATH=$HOSTD_LOG_PATH"
    echo "RUST_LOG=$RUST_LOG_LEVEL"
  } >"$ENV_DIR/hostd.env"
  chmod 0600 "$ENV_DIR/hostd.env"

  cat >"$UNIT_PATH" <<EOF
[Unit]
Description=relay-hostd (system)
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
EnvironmentFile=$ENV_DIR/hostd.env
ExecStart=$BIN_DIR/relay-hostd
Restart=always
RestartSec=2
NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
EOF

  systemctl daemon-reload
  if ! systemctl enable --now relay-hostd; then
    systemctl enable --now "$UNIT_PATH"
  fi

  echo "[ok] installed (system service)"
  echo "status: systemctl status relay-hostd"
  echo "logs:   journalctl -u relay-hostd -f"
  echo "sock:   $SOCK_PATH"
fi

if [[ "$INSTALL_SHIMS" == "1" ]]; then
  if [[ -x "$ROOT/install-shims.sh" ]]; then
    bash "$ROOT/install-shims.sh" --auto-path
  else
    echo "[client-init] warning: install-shims.sh not found; skipping" >&2
  fi
fi
CLIENT_INIT_EOF

cat >"$PKG_DIR/hostd-up.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

usage() {
  cat <<'USAGE'
relay-hostd up (packaged client)

Starts relay-hostd on this machine and connects outbound to a relay-server.

Usage:
  ./hostd-up.sh --server http://<server-host>:8787 [--host-id <id>] [--host-token <token>] [--sock <path>] [--rust-log warn]

Notes:
  - --server accepts http(s) URLs; it is converted to ws(s) for hostd.
  - hostd is designed to run on the same machine where codex/claude/iflow runs.
USAGE
}

SERVER_HTTP=""
HOST_ID="${HOST_ID:-}"
HOST_TOKEN="${HOST_TOKEN:-}"
SOCK_PATH="${LOCAL_UNIX_SOCKET:-${HOME}/.relay/relay-hostd.sock}"
RUST_LOG_LEVEL="${RUST_LOG:-warn}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) usage; exit 0 ;;
    --server) SERVER_HTTP="${2:-}"; shift 2 ;;
    --host-id) HOST_ID="${2:-}"; shift 2 ;;
    --host-token) HOST_TOKEN="${2:-}"; shift 2 ;;
    --sock) SOCK_PATH="${2:-}"; shift 2 ;;
    --rust-log) RUST_LOG_LEVEL="${2:-}"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; usage; exit 2 ;;
  esac
done

if [[ -z "$SERVER_HTTP" ]]; then
  echo "missing --server http(s)://..." >&2
  exit 2
fi

if [[ -z "$HOST_ID" ]]; then
  HOST_ID="host-$(hostname -s 2>/dev/null || hostname)"
fi

case "$SERVER_HTTP" in
  https://*) SERVER_BASE_URL="wss://${SERVER_HTTP#https://}" ;;
  http://*) SERVER_BASE_URL="ws://${SERVER_HTTP#http://}" ;;
  ws://*|wss://*) SERVER_BASE_URL="$SERVER_HTTP" ;;
  *) echo "invalid --server: $SERVER_HTTP (expected http(s):// or ws(s)://)" >&2; exit 2 ;;
esac

DATA_DIR="${HOME}/.relay"
CONFIG_PATH="${DATA_DIR}/hostd.json"
mkdir -p "$DATA_DIR" "$(dirname "$SOCK_PATH")"

SPOOL_DB_PATH="${SPOOL_DB_PATH:-$DATA_DIR/hostd-spool.db}"
HOSTD_LOG_PATH="${HOSTD_LOG_PATH:-$DATA_DIR/hostd.log}"

rm -f "$SOCK_PATH" >/dev/null 2>&1 || true

cat <<OUT
[hostd-up] starting

Server:
  $SERVER_HTTP
Host:
  id: $HOST_ID
  token: (hidden)
Local API:
  unix socket: $SOCK_PATH
Config:
  $CONFIG_PATH

Logs:
  $HOSTD_LOG_PATH

Press Ctrl-C to stop.
OUT

(
  export ABRELAY_CONFIG="$CONFIG_PATH"
  export SERVER_BASE_URL="$SERVER_BASE_URL"
  export HOST_ID="$HOST_ID"
  [[ -n "$HOST_TOKEN" ]] && export HOST_TOKEN="$HOST_TOKEN"
  export LOCAL_UNIX_SOCKET="$SOCK_PATH"
  export SPOOL_DB_PATH="$SPOOL_DB_PATH"
  export HOSTD_LOG_PATH="$HOSTD_LOG_PATH"
  export RUST_LOG="$RUST_LOG_LEVEL"
  exec "$ROOT/bin/relay-hostd"
) >>"$HOSTD_LOG_PATH" 2>&1
EOF

cat >"$PKG_DIR/install-hostd-systemd-user.sh" <<'INSTALL_HOSTD_SYSTEMD_USER_SH_EOF'
#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

usage() {
  cat <<'USAGE'
Install relay-hostd as a Linux systemd *user* service.

Usage:
  ./install-hostd-systemd-user.sh --server http://<server>:8787 [--host-id <id>] [--host-token <token>]

Notes:
  - Requires: systemd user sessions (systemctl --user).
  - Stores env at: ~/.relay/hostd.env
  - Installs unit at: $XDG_CONFIG_HOME/systemd/user/relay-hostd.service (default: ~/.config/systemd/user/relay-hostd.service)
USAGE
}

SERVER_HTTP=""
HOST_ID=""
HOST_TOKEN="${HOST_TOKEN:-}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) usage; exit 0 ;;
    --server) SERVER_HTTP="${2:-}"; shift 2 ;;
    --host-id) HOST_ID="${2:-}"; shift 2 ;;
    --host-token) HOST_TOKEN="${2:-}"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; usage; exit 2 ;;
  esac
done

if [[ -z "$SERVER_HTTP" ]]; then
  echo "missing --server" >&2
  usage
  exit 2
fi

if ! command -v systemctl >/dev/null 2>&1; then
  echo "systemctl not found; this helper supports Linux systemd only" >&2
  exit 1
fi

if ! systemctl --user show-environment >/dev/null 2>&1; then
  echo "systemctl --user is not available (no user systemd session?)" >&2
  echo "hint: on headless servers you may need lingering: loginctl enable-linger <user>" >&2
  exit 1
fi

if [[ -z "$HOST_ID" ]]; then
  HOST_ID="host-$(hostname -s 2>/dev/null || hostname)"
fi

case "$SERVER_HTTP" in
  https://*) SERVER_BASE_URL="wss://${SERVER_HTTP#https://}" ;;
  http://*) SERVER_BASE_URL="ws://${SERVER_HTTP#http://}" ;;
  ws://*|wss://*) SERVER_BASE_URL="$SERVER_HTTP" ;;
  *) echo "invalid --server: $SERVER_HTTP (expected http(s):// or ws(s)://)" >&2; exit 2 ;;
esac

DATA_DIR="${HOME}/.relay"
BIN_DIR="$DATA_DIR/bin"
SYSTEMD_USER_CONFIG_HOME="$(systemctl --user show-environment 2>/dev/null | awk -F= '$1=="XDG_CONFIG_HOME"{print $2; exit}')"
UNIT_BASE="${SYSTEMD_USER_CONFIG_HOME:-${XDG_CONFIG_HOME:-${HOME}/.config}}"
UNIT_DIR="${UNIT_BASE%/}/systemd/user"
mkdir -p "$DATA_DIR" "$BIN_DIR" "$UNIT_DIR"

install -m 0755 "$ROOT/bin/relay-hostd" "$BIN_DIR/relay-hostd"
install -m 0755 "$ROOT/bin/relay" "$BIN_DIR/relay"

CONFIG_PATH="${DATA_DIR}/hostd.json"
SOCK_PATH="${LOCAL_UNIX_SOCKET:-$DATA_DIR/relay-hostd.sock}"
SPOOL_DB_PATH="${SPOOL_DB_PATH:-$DATA_DIR/hostd-spool.db}"
HOSTD_LOG_PATH="${HOSTD_LOG_PATH:-$DATA_DIR/hostd.log}"
RUST_LOG_LEVEL="${RUST_LOG:-warn}"
PATH_ENV="${PATH:-}"

{
  echo "ABRELAY_CONFIG=$CONFIG_PATH"
  echo "SERVER_BASE_URL=$SERVER_BASE_URL"
  echo "HOST_ID=$HOST_ID"
  [[ -n "$HOST_TOKEN" ]] && echo "HOST_TOKEN=$HOST_TOKEN"
  echo "LOCAL_UNIX_SOCKET=$SOCK_PATH"
  echo "SPOOL_DB_PATH=$SPOOL_DB_PATH"
  echo "HOSTD_LOG_PATH=$HOSTD_LOG_PATH"
  echo "RUST_LOG=$RUST_LOG_LEVEL"
  [[ -n "$PATH_ENV" ]] && echo "PATH=$PATH_ENV"
} >"$DATA_DIR/hostd.env"
chmod 0600 "$DATA_DIR/hostd.env"

cat >"$UNIT_DIR/relay-hostd.service" <<EOF
[Unit]
Description=relay-hostd (runs local CLI sessions and connects to relay-server)
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
EnvironmentFile=%h/.relay/hostd.env
ExecStart=%h/.relay/bin/relay-hostd
Restart=always
RestartSec=2
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=full
ProtectHome=false

[Install]
WantedBy=default.target
EOF

[[ -f "$UNIT_DIR/relay-hostd.service" ]] || { echo "unit file not found: $UNIT_DIR/relay-hostd.service" >&2; exit 1; }
systemctl --user daemon-reload
if ! systemctl --user enable --now relay-hostd.service; then
  systemctl --user enable --now "$UNIT_DIR/relay-hostd.service" || true
  systemctl --user daemon-reload
  systemctl --user enable --now relay-hostd.service
fi

echo "[ok] relay-hostd installed and started"
echo "logs: journalctl --user -u relay-hostd -f"
echo "sock: $SOCK_PATH"
INSTALL_HOSTD_SYSTEMD_USER_SH_EOF

run chmod +x \
  "$PKG_DIR/client-init.sh" \
  "$PKG_DIR/hostd-up.sh" \
  "$PKG_DIR/install-shims.sh" \
  "$PKG_DIR/install-hostd-systemd-user.sh"

cat >"$PKG_DIR/README.txt" <<'EOF'
relay client package

Contents:
  - bin/relay-hostd : host daemon (run on the machine that runs codex/claude/iflow)
  - bin/relay       : local CLI (talks to hostd via unix socket)
  - client-init.sh  : one-shot installer (Linux, systemd user/system)
  - hostd-up.sh     : start hostd and connect to a remote relay-server
  - install-hostd-systemd-user.sh: install hostd as a Linux systemd user service
  - install-shims.sh: install codex/claude/iflow command shims to run via relay

Quick start:
  1) Start hostd:
     ./hostd-up.sh --server http://<your-vps>:8787

  2) Start a run (example):
     ./bin/relay codex --cwd /path/to/project

  3) Optional (Linux): one-shot install (recommended):
     ./client-init.sh --server http://<your-vps>:8787

  3) Optional (Linux): install hostd as a user service:
     ./install-hostd-systemd-user.sh --server http://<your-vps>:8787

  3) Optional: install shims so `codex` in any project dir uses relay:
     ./install-shims.sh --auto-path
EOF

echo "[package-client] ok: $PKG_DIR"
