#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

usage() {
  cat <<'EOF'
relay dev up (server + hostd)

Starts relay-server and relay-hostd for local dev/demo.
Default credentials: admin / 123456

Usage:
  scripts/dev-up.sh [--port 8787] [--keep-tmp] [--rust-log info] [--no-build]

Options:
  --port <port>      Bind server to 127.0.0.1:<port> (default: 8787)
  --keep-tmp         Keep temp dir (logs + db files) after exit
  --rust-log <level> RUST_LOG for server/hostd (default: warn)
  --no-build         Skip upfront cargo build (not recommended on first run)
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

PORT="8787"
KEEP_TMP="0"
RUST_LOG_LEVEL="warn"
DO_BUILD="1"

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) usage; exit 0 ;;
    --port) PORT="${2:-}"; shift 2 ;;
    --keep-tmp) KEEP_TMP="1"; shift ;;
    --rust-log) RUST_LOG_LEVEL="${2:-}"; shift 2 ;;
    --no-build) DO_BUILD="0"; shift ;;
    *) echo "unknown arg: $1" >&2; usage; exit 2 ;;
  esac
done

if [[ -z "$PORT" ]]; then
  echo "--port requires a value" >&2
  exit 2
fi

need cargo
need curl

TMP_BASE="$ROOT/.relay-tmp"
mkdir -p "$TMP_BASE"
TMP_DIR="$(mktemp -d "$TMP_BASE/dev.XXXXXXXX" 2>/dev/null || mktemp -d -t relay-dev)"
DB_PATH="$TMP_DIR/server.db"
SPOOL_DB_PATH="$TMP_DIR/hostd-spool.db"
SOCK_PATH="$TMP_BASE/relay-hostd-dev-$PORT.sock"
SERVER_LOG="$TMP_DIR/server.log"
HOSTD_LOG="$TMP_DIR/hostd.log"
SERVER_BIN="$ROOT/target/debug/relay-server"
HOSTD_BIN="$ROOT/target/debug/relay-hostd"

cleanup() {
  if [[ -n "${HOSTD_PID:-}" ]]; then kill "$HOSTD_PID" >/dev/null 2>&1 || true; fi
  if [[ -n "${SERVER_PID:-}" ]]; then kill "$SERVER_PID" >/dev/null 2>&1 || true; fi
  wait "${HOSTD_PID:-}" >/dev/null 2>&1 || true
  wait "${SERVER_PID:-}" >/dev/null 2>&1 || true
  rm -f "$SOCK_PATH" >/dev/null 2>&1 || true
  if [[ "$KEEP_TMP" != "1" ]]; then
    rm -rf "$TMP_DIR" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

echo "[dev] tmp=$TMP_DIR"
echo "[dev] db=$DB_PATH"
echo "[dev] spool=$SPOOL_DB_PATH"
echo "[dev] sock=$SOCK_PATH"
echo "[dev] logs: $SERVER_LOG $HOSTD_LOG"

ADMIN_USERNAME="admin"
ADMIN_PASSWORD="123456"
JWT_SECRET="dev"
BIND_ADDR="127.0.0.1:$PORT"

if [[ "$DO_BUILD" == "1" ]]; then
  echo "[dev] building (first run may take a while)..."
  run cargo build -p relay-server
  run cargo build -p relay-hostd
fi

if [[ -x "$SERVER_BIN" ]]; then
  ADMIN_PASSWORD_HASH="$("$SERVER_BIN" --hash-password "$ADMIN_PASSWORD")"
else
  ADMIN_PASSWORD_HASH="$(run cargo run -q -p relay-server -- --hash-password "$ADMIN_PASSWORD")"
fi

rm -f "$SOCK_PATH" >/dev/null 2>&1 || true

(
  export JWT_SECRET="$JWT_SECRET"
  export ADMIN_USERNAME="$ADMIN_USERNAME"
  export ADMIN_PASSWORD_HASH="$ADMIN_PASSWORD_HASH"
  export DATABASE_URL="sqlite:$DB_PATH"
  export BIND_ADDR="$BIND_ADDR"
  export RUST_LOG="$RUST_LOG_LEVEL"
  if [[ -x "$SERVER_BIN" ]]; then
    exec "$SERVER_BIN"
  else
    exec cargo run -q -p relay-server
  fi
) >"$SERVER_LOG" 2>&1 &
SERVER_PID=$!

for _ in $(seq 1 200); do
  if ! kill -0 "$SERVER_PID" >/dev/null 2>&1; then
    echo "[dev] server exited early; logs:" >&2
    tail -n 200 "$SERVER_LOG" >&2 || true
    exit 1
  fi
  if curl --silent "http://127.0.0.1:$PORT/health" >/dev/null 2>&1; then
    break
  fi
  sleep 0.1
done
curl --silent "http://127.0.0.1:$PORT/health" >/dev/null 2>&1 || {
  echo "[dev] server health not ready; logs:" >&2
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
  if [[ -x "$HOSTD_BIN" ]]; then
    exec "$HOSTD_BIN"
  else
    exec cargo run -q -p relay-hostd
  fi
) >"$HOSTD_LOG" 2>&1 &
HOSTD_PID=$!

for _ in $(seq 1 600); do
  if ! kill -0 "$HOSTD_PID" >/dev/null 2>&1; then
    echo "[dev] hostd exited early; logs:" >&2
    tail -n 200 "$HOSTD_LOG" >&2 || true
    exit 1
  fi
  [[ -S "$SOCK_PATH" ]] && break
  sleep 0.1
done
[[ -S "$SOCK_PATH" ]] || {
  echo "[dev] hostd unix socket not ready: $SOCK_PATH" >&2
  echo "[dev] hostd logs:" >&2
  tail -n 200 "$HOSTD_LOG" >&2 || true
  echo "[dev] hint: if this is the first run, compilation may still be in progress; re-run or use --no-build to see compile output live." >&2
  exit 1
}

cat <<EOF
[dev] up

Server:
  http://127.0.0.1:$PORT
  health: curl -s http://127.0.0.1:$PORT/health

Credentials (dev):
  username: $ADMIN_USERNAME
  password: $ADMIN_PASSWORD

Hostd local API:
  unix socket: $SOCK_PATH

CLI quick check (run manually):
  cd cli
  bun run dev login --server http://127.0.0.1:$PORT --username $ADMIN_USERNAME --password '$ADMIN_PASSWORD'

Logs:
  server: $SERVER_LOG
  hostd:  $HOSTD_LOG

Press Ctrl-C to stop.
EOF

wait "$SERVER_PID" "$HOSTD_PID"
