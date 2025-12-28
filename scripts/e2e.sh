#!/usr/bin/env bash
set -euo pipefail

# End-to-end smoke test (dev): server + hostd + run + remote input + DB assertions.
#
# Requirements: cargo, curl, sqlite3, bun, node
#
# Usage:
#   scripts/e2e.sh

need() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing dependency: $1" >&2
    exit 1
  fi
}

need cargo
need curl
need sqlite3
need bun
need node

TMP_DIR="$(mktemp -d 2>/dev/null || mktemp -d -t relay-e2e)"
cleanup() {
  if [[ -n "${HOSTD_PID:-}" ]]; then kill "$HOSTD_PID" >/dev/null 2>&1 || true; fi
  if [[ -n "${SERVER_PID:-}" ]]; then kill "$SERVER_PID" >/dev/null 2>&1 || true; fi
  wait "${HOSTD_PID:-}" >/dev/null 2>&1 || true
  wait "${SERVER_PID:-}" >/dev/null 2>&1 || true
  rm -rf "$TMP_DIR" >/dev/null 2>&1 || true
}
trap cleanup EXIT

PORT="$(
  node -e 'const net=require("net");const s=net.createServer();s.listen(0,"127.0.0.1",()=>{console.log(s.address().port);s.close();});'
)"

ADMIN_USERNAME="${ADMIN_USERNAME:-admin}"
ADMIN_PASSWORD="${ADMIN_PASSWORD:-pass}"
JWT_SECRET="${JWT_SECRET:-dev}"

DB_PATH="$TMP_DIR/server.db"
SPOOL_DB_PATH="$TMP_DIR/hostd-spool.db"
SOCK_PATH="$TMP_DIR/relay-hostd.sock"

echo "[e2e] temp=$TMP_DIR port=$PORT"

ADMIN_PASSWORD_HASH="$(cargo run -q -p relay-server -- --hash-password "$ADMIN_PASSWORD")"

JWT_SECRET="$JWT_SECRET" \
ADMIN_USERNAME="$ADMIN_USERNAME" \
ADMIN_PASSWORD_HASH="$ADMIN_PASSWORD_HASH" \
DATABASE_URL="sqlite:$DB_PATH" \
BIND_ADDR="127.0.0.1:$PORT" \
RUST_LOG=warn \
cargo run -q -p relay-server &
SERVER_PID=$!

for _ in $(seq 1 50); do
  if curl --silent "http://127.0.0.1:$PORT/health" >/dev/null 2>&1; then
    break
  fi
  sleep 0.1
done

SERVER_BASE_URL="ws://127.0.0.1:$PORT" \
HOST_ID="host-dev" \
HOST_TOKEN="devtoken" \
LOCAL_UNIX_SOCKET="$SOCK_PATH" \
SPOOL_DB_PATH="$SPOOL_DB_PATH" \
RUST_LOG=warn \
cargo run -q -p relay-hostd &
HOSTD_PID=$!

for _ in $(seq 1 100); do
  [[ -S "$SOCK_PATH" ]] && break
  sleep 0.1
done
[[ -S "$SOCK_PATH" ]] || { echo "[e2e] hostd unix socket not ready: $SOCK_PATH" >&2; exit 1; }

RUN_JSON="$(
  curl --silent --show-error --unix-socket "$SOCK_PATH" \
    http://localhost/runs \
    -H 'content-type: application/json' \
    --data-binary '{"tool":"codex","cmd":"echo Proceed?; cat","cwd":null}'
)"
RUN_ID="$(node -e 'const fs=require("fs");const s=fs.readFileSync(0,"utf8");console.log(JSON.parse(s).run_id);' <<<"$RUN_JSON")"
echo "[e2e] run_id=$RUN_ID"

LOGIN_JSON="$(
  curl --silent --show-error \
    "http://127.0.0.1:$PORT/auth/login" \
    -H 'content-type: application/json' \
    --data-binary "{\"username\":\"$ADMIN_USERNAME\",\"password\":\"$ADMIN_PASSWORD\"}"
)"
TOKEN="$(node -e 'const fs=require("fs");const s=fs.readFileSync(0,"utf8");console.log(JSON.parse(s).access_token);' <<<"$LOGIN_JSON")"

# Send input over the same WS path used by the PWA, twice with same input_id (idempotency).
INPUT_ID="$(node -e 'console.log(crypto.randomUUID())')"
bun run cli/src/index.ts ws-send-input --server "http://127.0.0.1:$PORT" --token "$TOKEN" --run "$RUN_ID" --text $'y\n' --input-id "$INPUT_ID" >/dev/null
bun run cli/src/index.ts ws-send-input --server "http://127.0.0.1:$PORT" --token "$TOKEN" --run "$RUN_ID" --text $'y\n' --input-id "$INPUT_ID" >/dev/null

sleep 0.5

COUNT="$(sqlite3 "$DB_PATH" "select count(*) from events where run_id='$RUN_ID' and type='run.input' and input_id='$INPUT_ID';")"
if [[ "$COUNT" != "1" ]]; then
  echo "[e2e] expected 1 run.input for input_id, got $COUNT" >&2
  exit 1
fi

echo "[e2e] ok: input idempotency"
echo "[e2e] ok: db=$DB_PATH spool=$SPOOL_DB_PATH"

