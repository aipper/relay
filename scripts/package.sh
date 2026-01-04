#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

usage() {
  cat <<'EOF'
relay packaging helper (quick path)

Builds release binaries and creates a self-contained directory under ./dist/
with an up.sh script that starts server + hostd.

Usage:
  scripts/package.sh [--no-web] [--with-web]

Options:
  --no-web     Do not build/copy web assets (skip WEB_DIST_DIR).
  --with-web   Try to build web assets even if deps are missing (may need network).
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

NO_WEB="0"
WITH_WEB="0"

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) usage; exit 0 ;;
    --no-web) NO_WEB="1"; shift ;;
    --with-web) WITH_WEB="1"; shift ;;
    *) echo "unknown arg: $1" >&2; usage; exit 2 ;;
  esac
done

need cargo
need node

VERSION="$(cargo metadata --no-deps --format-version 1 | node -e 'const fs=require("fs");const m=JSON.parse(fs.readFileSync(0,"utf8"));const p=m.packages.find((x)=>x.name==="relay-server");process.stdout.write(p?.version||"0.0.0");')"
OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

PKG_DIR="$ROOT/dist/relay-$VERSION-$OS-$ARCH"
BIN_DIR="$PKG_DIR/bin"

run mkdir -p "$BIN_DIR"

echo "[package] building rust release binaries..."
run cargo build --release -p relay-server
run cargo build --release -p relay-hostd
run cargo build --release -p relay-cli

run cp "$ROOT/target/release/relay-server" "$BIN_DIR/"
run cp "$ROOT/target/release/relay-hostd" "$BIN_DIR/"
run cp "$ROOT/target/release/relay" "$BIN_DIR/"

run cp "$ROOT/scripts/dev-up.sh" "$PKG_DIR/dev-up.sh"
run cp "$ROOT/scripts/mock-codex.sh" "$PKG_DIR/mock-codex.sh"
run cp "$ROOT/scripts/install-shims.sh" "$PKG_DIR/install-shims.sh"

WEB_BUILT="0"
if [[ "$NO_WEB" == "0" ]]; then
  if [[ -d "$ROOT/web/node_modules" || "$WITH_WEB" == "1" ]]; then
    if command -v bun >/dev/null 2>&1; then
      echo "[package] building web assets..."
      if [[ "$WITH_WEB" == "1" ]]; then
        (cd web && run bun install && run bun run build)
      else
        (cd web && run bun run build)
      fi
      run mkdir -p "$PKG_DIR/web"
      run cp -R "$ROOT/web/dist" "$PKG_DIR/web/dist"
      WEB_BUILT="1"
    else
      echo "[package] skipping web build: missing bun" >&2
    fi
  else
    echo "[package] skipping web build: missing web/node_modules (use --with-web to force)" >&2
  fi
fi

cat >"$PKG_DIR/up.sh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

usage() {
  cat <<'USAGE'
relay up (packaged)

Starts relay-server and relay-hostd from this directory.
Default credentials: admin / 123456

Usage:
  ./up.sh [--port 8787] [--bind 127.0.0.1] [--rust-log info]
USAGE
}

need() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing dependency: $1" >&2
    exit 1
  fi
}

PORT="8787"
BIND="127.0.0.1"
RUST_LOG_LEVEL="warn"
while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) usage; exit 0 ;;
    --port) PORT="${2:-}"; shift 2 ;;
    --bind) BIND="${2:-}"; shift 2 ;;
    --rust-log) RUST_LOG_LEVEL="${2:-}"; shift 2 ;;
    *) echo "unknown arg: $1" >&2; usage; exit 2 ;;
  esac
done

need curl

DATA_DIR="$ROOT/data"
mkdir -p "$DATA_DIR"

DB_PATH="$DATA_DIR/server.db"
SPOOL_DB_PATH="$DATA_DIR/hostd-spool.db"
SOCK_PATH="${HOME}/.relay/relay-hostd.sock"
SERVER_LOG="$DATA_DIR/server.log"
HOSTD_LOG="$DATA_DIR/hostd.log"

SERVER_BIN="$ROOT/bin/relay-server"
HOSTD_BIN="$ROOT/bin/relay-hostd"

cleanup() {
  if [[ -n "${HOSTD_PID:-}" ]]; then kill "$HOSTD_PID" >/dev/null 2>&1 || true; fi
  if [[ -n "${SERVER_PID:-}" ]]; then kill "$SERVER_PID" >/dev/null 2>&1 || true; fi
  wait "${HOSTD_PID:-}" >/dev/null 2>&1 || true
  wait "${SERVER_PID:-}" >/dev/null 2>&1 || true
  rm -f "$SOCK_PATH" >/dev/null 2>&1 || true
}
trap cleanup EXIT

ADMIN_USERNAME="admin"
ADMIN_PASSWORD="123456"
JWT_SECRET="dev"
BIND_ADDR="$BIND:$PORT"

ADMIN_PASSWORD_HASH="$("$SERVER_BIN" --hash-password "$ADMIN_PASSWORD")"

rm -f "$SOCK_PATH" >/dev/null 2>&1 || true

(
  export JWT_SECRET="$JWT_SECRET"
  export ADMIN_USERNAME="$ADMIN_USERNAME"
  export ADMIN_PASSWORD_HASH="$ADMIN_PASSWORD_HASH"
  export DATABASE_URL="sqlite:$DB_PATH"
  export BIND_ADDR="$BIND_ADDR"
  export RUST_LOG="$RUST_LOG_LEVEL"
  if [[ -d "$ROOT/web/dist" ]]; then
    export WEB_DIST_DIR="$ROOT/web/dist"
  fi
  exec "$SERVER_BIN"
) >"$SERVER_LOG" 2>&1 &
SERVER_PID=$!

for _ in $(seq 1 200); do
  if ! kill -0 "$SERVER_PID" >/dev/null 2>&1; then
    echo "[up] server exited early; logs:" >&2
    tail -n 200 "$SERVER_LOG" >&2 || true
    exit 1
  fi
  if curl --silent "http://127.0.0.1:$PORT/health" >/dev/null 2>&1; then
    break
  fi
  sleep 0.1
done
curl --silent "http://127.0.0.1:$PORT/health" >/dev/null 2>&1 || {
  echo "[up] server health not ready; logs:" >&2
  tail -n 200 "$SERVER_LOG" >&2 || true
  exit 1
}

(
  export SERVER_BASE_URL="ws://127.0.0.1:$PORT"
  export HOST_ID="host-dev"
  export HOST_TOKEN="devtoken"
  export LOCAL_UNIX_SOCKET="$SOCK_PATH"
  export SPOOL_DB_PATH="$SPOOL_DB_PATH"
  export HOSTD_LOG_PATH="$HOSTD_LOG"
  export RUST_LOG="$RUST_LOG_LEVEL"
  mkdir -p "$(dirname "$SOCK_PATH")" >/dev/null 2>&1 || true
  exec "$HOSTD_BIN"
) >"$HOSTD_LOG" 2>&1 &
HOSTD_PID=$!

for _ in $(seq 1 300); do
  if ! kill -0 "$HOSTD_PID" >/dev/null 2>&1; then
    echo "[up] hostd exited early; logs:" >&2
    tail -n 200 "$HOSTD_LOG" >&2 || true
    exit 1
  fi
  [[ -S "$SOCK_PATH" ]] && break
  sleep 0.1
done
[[ -S "$SOCK_PATH" ]] || {
  echo "[up] hostd unix socket not ready: $SOCK_PATH" >&2
  tail -n 200 "$HOSTD_LOG" >&2 || true
  exit 1
}

cat <<OUT
[up] ok

Server:
  http://$BIND:$PORT

Credentials:
  username: $ADMIN_USERNAME
  password: $ADMIN_PASSWORD

Hostd local API:
  unix socket: $SOCK_PATH

Packaged CLI:
  $ROOT/bin/relay codex --sock "$SOCK_PATH" [--cwd /path/to/project]

Logs:
  server: $SERVER_LOG
  hostd:  $HOSTD_LOG

Press Ctrl-C to stop.
OUT

wait "$SERVER_PID" "$HOSTD_PID"
EOF

chmod +x "$PKG_DIR/up.sh" >/dev/null 2>&1 || true

cat <<EOF
[package] ok
  dir: $PKG_DIR
  bin: $BIN_DIR/relay-server, $BIN_DIR/relay-hostd
  up:  $PKG_DIR/up.sh
  web: $([[ "$WEB_BUILT" == "1" ]] && echo "included" || echo "skipped")
EOF
