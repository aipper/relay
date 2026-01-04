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

PKG_DIR="$ROOT/dist/relay-client-$VERSION-$OS-$ARCH"
BIN_DIR="$PKG_DIR/bin"

run mkdir -p "$BIN_DIR"

echo "[package-client] building rust release binaries..."
run cargo build --release -p relay-hostd
run cargo build --release -p relay-cli

run cp "$ROOT/target/release/relay-hostd" "$BIN_DIR/"
run cp "$ROOT/target/release/relay" "$BIN_DIR/"

run cp "$ROOT/scripts/install-shims.sh" "$PKG_DIR/install-shims.sh"

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

if [[ -z "$HOST_TOKEN" ]]; then
  HOST_TOKEN="devtoken"
  echo "[hostd-up] warning: --host-token not set; using devtoken (change for production)" >&2
fi

case "$SERVER_HTTP" in
  https://*) SERVER_BASE_URL="wss://${SERVER_HTTP#https://}" ;;
  http://*) SERVER_BASE_URL="ws://${SERVER_HTTP#http://}" ;;
  ws://*|wss://*) SERVER_BASE_URL="$SERVER_HTTP" ;;
  *) echo "invalid --server: $SERVER_HTTP (expected http(s):// or ws(s)://)" >&2; exit 2 ;;
esac

DATA_DIR="${HOME}/.relay"
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

Logs:
  $HOSTD_LOG_PATH

Press Ctrl-C to stop.
OUT

(
  export SERVER_BASE_URL="$SERVER_BASE_URL"
  export HOST_ID="$HOST_ID"
  export HOST_TOKEN="$HOST_TOKEN"
  export LOCAL_UNIX_SOCKET="$SOCK_PATH"
  export SPOOL_DB_PATH="$SPOOL_DB_PATH"
  export HOSTD_LOG_PATH="$HOSTD_LOG_PATH"
  export RUST_LOG="$RUST_LOG_LEVEL"
  exec "$ROOT/bin/relay-hostd"
) >>"$HOSTD_LOG_PATH" 2>&1
EOF

cat >"$PKG_DIR/install-hostd-systemd-user.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

usage() {
  cat <<'USAGE'
Install relay-hostd as a Linux systemd *user* service.

Usage:
  ./install-hostd-systemd-user.sh --server http://<server>:8787 --host-token <token> [--host-id <id>]

Notes:
  - Requires: systemd user sessions (systemctl --user).
  - Stores env at: ~/.relay/hostd.env
  - Installs unit at: ~/.config/systemd/user/relay-hostd.service
USAGE
}

SERVER_HTTP=""
HOST_ID=""
HOST_TOKEN=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) usage; exit 0 ;;
    --server) SERVER_HTTP="${2:-}"; shift 2 ;;
    --host-id) HOST_ID="${2:-}"; shift 2 ;;
    --host-token) HOST_TOKEN="${2:-}"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; usage; exit 2 ;;
  esac
done

if [[ -z "$SERVER_HTTP" || -z "$HOST_TOKEN" ]]; then
  echo "missing --server and/or --host-token" >&2
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
mkdir -p "$DATA_DIR" "$BIN_DIR" "${HOME}/.config/systemd/user"

install -m 0755 "$ROOT/bin/relay-hostd" "$BIN_DIR/relay-hostd"
install -m 0755 "$ROOT/bin/relay" "$BIN_DIR/relay"

SOCK_PATH="${LOCAL_UNIX_SOCKET:-$DATA_DIR/relay-hostd.sock}"
SPOOL_DB_PATH="${SPOOL_DB_PATH:-$DATA_DIR/hostd-spool.db}"
HOSTD_LOG_PATH="${HOSTD_LOG_PATH:-$DATA_DIR/hostd.log}"
RUST_LOG_LEVEL="${RUST_LOG:-warn}"

cat >"$DATA_DIR/hostd.env" <<EOF
SERVER_BASE_URL=$SERVER_BASE_URL
HOST_ID=$HOST_ID
HOST_TOKEN=$HOST_TOKEN
LOCAL_UNIX_SOCKET=$SOCK_PATH
SPOOL_DB_PATH=$SPOOL_DB_PATH
HOSTD_LOG_PATH=$HOSTD_LOG_PATH
RUST_LOG=$RUST_LOG_LEVEL
EOF
chmod 0600 "$DATA_DIR/hostd.env"

cat >"${HOME}/.config/systemd/user/relay-hostd.service" <<EOF
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
ProtectSystem=strict
ProtectHome=false
ReadWritePaths=%h/.relay

[Install]
WantedBy=default.target
EOF

systemctl --user daemon-reload
systemctl --user enable --now relay-hostd

echo "[ok] relay-hostd installed and started"
echo "logs: journalctl --user -u relay-hostd -f"
echo "sock: $SOCK_PATH"
EOF

run chmod +x \
  "$PKG_DIR/hostd-up.sh" \
  "$PKG_DIR/install-shims.sh" \
  "$PKG_DIR/install-hostd-systemd-user.sh"

cat >"$PKG_DIR/README.txt" <<'EOF'
relay client package

Contents:
  - bin/relay-hostd : host daemon (run on the machine that runs codex/claude/iflow)
  - bin/relay       : local CLI (talks to hostd via unix socket)
  - hostd-up.sh     : start hostd and connect to a remote relay-server
  - install-hostd-systemd-user.sh: install hostd as a Linux systemd user service
  - install-shims.sh: install codex/claude/iflow command shims to run via relay

Quick start:
  1) Start hostd:
     ./hostd-up.sh --server http://<your-vps>:8787 --host-token <token>

  2) Start a run (example):
     ./bin/relay codex --cwd /path/to/project

  3) Optional (Linux): install hostd as a user service:
     ./install-hostd-systemd-user.sh --server http://<your-vps>:8787 --host-token <token>

  3) Optional: install shims so `codex` in any project dir uses relay:
     ./install-shims.sh --auto-path
EOF

echo "[package-client] ok: $PKG_DIR"
