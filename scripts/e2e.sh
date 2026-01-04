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

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

NO_BUILD="${NO_BUILD:-0}"

TMP_BASE="$ROOT/.relay-tmp"
mkdir -p "$TMP_BASE"
TMP_DIR="$(mktemp -d "$TMP_BASE/e2e.XXXXXXXX" 2>/dev/null || mktemp -d -t relay-e2e)"
KEEP_TMP="${KEEP_TMP:-0}"
cleanup() {
  if [[ -n "${HOSTD_PID:-}" ]]; then kill "$HOSTD_PID" >/dev/null 2>&1 || true; fi
  if [[ -n "${SERVER_PID:-}" ]]; then kill "$SERVER_PID" >/dev/null 2>&1 || true; fi
  wait "${HOSTD_PID:-}" >/dev/null 2>&1 || true
  wait "${SERVER_PID:-}" >/dev/null 2>&1 || true
  if [[ -n "${SOCK_PATH:-}" ]]; then rm -f "$SOCK_PATH" >/dev/null 2>&1 || true; fi
  if [[ "$KEEP_TMP" != "1" ]]; then
    rm -rf "$TMP_DIR" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

PORT="$(
  node -e 'const net=require("net");const s=net.createServer();s.listen(0,"127.0.0.1",()=>{console.log(s.address().port);s.close();});'
)"

ADMIN_USERNAME="${ADMIN_USERNAME:-admin}"
ADMIN_PASSWORD="${ADMIN_PASSWORD:-pass}"
JWT_SECRET="${JWT_SECRET:-dev}"
RUST_LOG="${RUST_LOG:-warn}"

DB_PATH="$TMP_DIR/server.db"
SPOOL_DB_PATH="$TMP_DIR/hostd-spool.db"
SOCK_PATH="$TMP_DIR/relay-hostd.sock"
SERVER_LOG="$TMP_DIR/server.log"
HOSTD_LOG="$TMP_DIR/hostd.log"
SERVER_BIN="$ROOT/target/debug/relay-server"
HOSTD_BIN="$ROOT/target/debug/relay-hostd"

echo "[e2e] temp=$TMP_DIR port=$PORT"

if [[ "$NO_BUILD" != "1" ]]; then
  cargo build -p relay-server >/dev/null
  cargo build -p relay-hostd >/dev/null
fi

if [[ -x "$SERVER_BIN" ]]; then
  ADMIN_PASSWORD_HASH="$("$SERVER_BIN" --hash-password "$ADMIN_PASSWORD")"
else
  ADMIN_PASSWORD_HASH="$(cargo run -p relay-server -- --hash-password "$ADMIN_PASSWORD")"
fi

if [[ -x "$SERVER_BIN" ]]; then
  env \
    JWT_SECRET="$JWT_SECRET" \
    ADMIN_USERNAME="$ADMIN_USERNAME" \
    ADMIN_PASSWORD_HASH="$ADMIN_PASSWORD_HASH" \
    DATABASE_URL="sqlite:$DB_PATH" \
    BIND_ADDR="127.0.0.1:$PORT" \
    RUST_LOG="$RUST_LOG" \
    "$SERVER_BIN" >"$SERVER_LOG" 2>&1 &
else
  env \
    JWT_SECRET="$JWT_SECRET" \
    ADMIN_USERNAME="$ADMIN_USERNAME" \
    ADMIN_PASSWORD_HASH="$ADMIN_PASSWORD_HASH" \
    DATABASE_URL="sqlite:$DB_PATH" \
    BIND_ADDR="127.0.0.1:$PORT" \
    RUST_LOG="$RUST_LOG" \
    cargo run -p relay-server >"$SERVER_LOG" 2>&1 &
fi
SERVER_PID=$!

for _ in $(seq 1 1200); do
  if ! kill -0 "$SERVER_PID" >/dev/null 2>&1; then
    echo "[e2e] server exited early; logs:" >&2
    tail -n 200 "$SERVER_LOG" >&2 || true
    exit 1
  fi
  if curl --silent "http://127.0.0.1:$PORT/health" >/dev/null 2>&1; then
    break
  fi
  sleep 0.1
done
curl --silent "http://127.0.0.1:$PORT/health" >/dev/null 2>&1 || {
  echo "[e2e] server health not ready; logs:" >&2
  tail -n 200 "$SERVER_LOG" >&2 || true
  echo "[e2e] hint: first run may still be compiling; re-run or inspect server.log with KEEP_TMP=1" >&2
  exit 1
}

	if [[ -x "$HOSTD_BIN" ]]; then
	  env \
	    SERVER_BASE_URL="ws://127.0.0.1:$PORT" \
	    HOST_ID="host-dev" \
	    HOST_TOKEN="devtoken" \
	    LOCAL_UNIX_SOCKET="$SOCK_PATH" \
	    SPOOL_DB_PATH="$SPOOL_DB_PATH" \
	    HOSTD_LOG_PATH="$HOSTD_LOG" \
	    RUST_LOG="$RUST_LOG" \
	    "$HOSTD_BIN" >"$HOSTD_LOG" 2>&1 &
	else
	  env \
	    SERVER_BASE_URL="ws://127.0.0.1:$PORT" \
	    HOST_ID="host-dev" \
	    HOST_TOKEN="devtoken" \
	    LOCAL_UNIX_SOCKET="$SOCK_PATH" \
	    SPOOL_DB_PATH="$SPOOL_DB_PATH" \
	    HOSTD_LOG_PATH="$HOSTD_LOG" \
	    RUST_LOG="$RUST_LOG" \
	    cargo run -p relay-hostd >"$HOSTD_LOG" 2>&1 &
	fi
HOSTD_PID=$!

for _ in $(seq 1 1200); do
  if ! kill -0 "$HOSTD_PID" >/dev/null 2>&1; then
    echo "[e2e] hostd exited early; logs:" >&2
    tail -n 200 "$HOSTD_LOG" >&2 || true
    exit 1
  fi
  [[ -S "$SOCK_PATH" ]] && break
  sleep 0.1
done
[[ -S "$SOCK_PATH" ]] || {
  echo "[e2e] hostd unix socket not ready: $SOCK_PATH" >&2
  echo "[e2e] hostd logs:" >&2
  tail -n 200 "$HOSTD_LOG" >&2 || true
  echo "[e2e] hint: first run may still be compiling; re-run or inspect hostd.log with KEEP_TMP=1" >&2
  exit 1
}

LOGIN_JSON="$(
  curl --silent --show-error \
    "http://127.0.0.1:$PORT/auth/login" \
    -H 'content-type: application/json' \
    --data-binary "{\"username\":\"$ADMIN_USERNAME\",\"password\":\"$ADMIN_PASSWORD\"}"
)"
TOKEN="$(node -e 'const fs=require("fs");const s=fs.readFileSync(0,"utf8");console.log(JSON.parse(s).access_token);' <<<"$LOGIN_JSON")"

HOST_INFO_OUT="$(
  bun run cli/src/index.ts ws-rpc-host-info \
    --server "http://127.0.0.1:$PORT" \
    --token "$TOKEN" \
    --host-id "host-dev"
)"
node -e '
const fs=require("fs");
const raw=fs.readFileSync(0,"utf8");
const m=JSON.parse(raw);
if (m?.type !== "rpc.response") process.exit(2);
if (m?.data?.ok !== true) process.exit(3);
if (m?.data?.rpc_type !== "rpc.host.info") process.exit(4);
if (!m?.data?.result?.host_id) process.exit(5);
' <<<"$HOST_INFO_OUT"
echo "[e2e] ok: ws-rpc host info"

HOST_DOCTOR_OUT="$(
  bun run cli/src/index.ts ws-rpc-host-doctor \
    --server "http://127.0.0.1:$PORT" \
    --token "$TOKEN" \
    --host-id "host-dev"
)"
node -e '
const fs=require("fs");
const raw=fs.readFileSync(0,"utf8");
const m=JSON.parse(raw);
if (m?.type !== "rpc.response") process.exit(2);
if (m?.data?.ok !== true) process.exit(3);
if (m?.data?.rpc_type !== "rpc.host.doctor") process.exit(4);
if (!Array.isArray(m?.data?.result?.deps)) process.exit(5);
' <<<"$HOST_DOCTOR_OUT"
echo "[e2e] ok: ws-rpc host doctor"

HOST_CAP_OUT="$(
  bun run cli/src/index.ts ws-rpc-host-capabilities \
    --server "http://127.0.0.1:$PORT" \
    --token "$TOKEN" \
    --host-id "host-dev"
)"
node -e '
const fs=require("fs");
const raw=fs.readFileSync(0,"utf8");
const m=JSON.parse(raw);
if (m?.type !== "rpc.response") process.exit(2);
if (m?.data?.ok !== true) process.exit(3);
if (m?.data?.rpc_type !== "rpc.host.capabilities") process.exit(4);
if (!Array.isArray(m?.data?.result?.supported_rpc)) process.exit(5);
' <<<"$HOST_CAP_OUT"
echo "[e2e] ok: ws-rpc host capabilities"

HOST_LOGS_OUT="$(
  bun run cli/src/index.ts ws-rpc-host-logs-tail \
    --server "http://127.0.0.1:$PORT" \
    --token "$TOKEN" \
    --host-id "host-dev" \
    --lines 50 \
    --max-bytes 200000
)"
node -e '
const fs=require("fs");
const raw=fs.readFileSync(0,"utf8");
const m=JSON.parse(raw);
if (m?.type !== "rpc.response") process.exit(2);
if (m?.data?.ok !== true) process.exit(3);
if (m?.data?.rpc_type !== "rpc.host.logs.tail") process.exit(4);
if (typeof m?.data?.result?.path !== "string") process.exit(5);
if (typeof m?.data?.result?.text !== "string") process.exit(6);
' <<<"$HOST_LOGS_OUT"
echo "[e2e] ok: ws-rpc host logs tail"

SESSIONS_JSON="$(
  curl --silent --show-error \
    "http://127.0.0.1:$PORT/sessions" \
    -H "Authorization: Bearer $TOKEN"
)"
node -e '
const fs = require("fs");
const raw = fs.readFileSync(0, "utf8");
const rows = JSON.parse(raw);
if (!Array.isArray(rows)) process.exit(2);
' <<<"$SESSIONS_JSON"
echo "[e2e] ok: sessions api"

RECENT_JSON="$(
  curl --silent --show-error \
    "http://127.0.0.1:$PORT/sessions/recent?limit=5" \
    -H "Authorization: Bearer $TOKEN"
)"
node -e '
const fs = require("fs");
const raw = fs.readFileSync(0, "utf8");
const rows = JSON.parse(raw);
if (!Array.isArray(rows)) process.exit(2);
if (rows.length > 5) process.exit(3);
' <<<"$RECENT_JSON"
echo "[e2e] ok: sessions recent api"

RUN_JSON="$(
  bun run cli/src/index.ts ws-start-run \
    --server "http://127.0.0.1:$PORT" \
    --token "$TOKEN" \
    --host-id "host-dev" \
    --tool "codex" \
    --cmd "bash scripts/mock-codex.sh"
)"
RUN_ID="$(node -e 'const fs=require("fs");const s=fs.readFileSync(0,"utf8");console.log(JSON.parse(s).run_id);' <<<"$RUN_JSON")"
echo "[e2e] run_id=$RUN_ID"

SESSION_JSON="$(
  curl --silent --show-error \
    "http://127.0.0.1:$PORT/sessions/$RUN_ID" \
    -H "Authorization: Bearer $TOKEN"
)"
node -e '
const fs = require("fs");
const raw = fs.readFileSync(0, "utf8");
const row = JSON.parse(raw);
if (!row || typeof row !== "object") process.exit(2);
if (typeof row.id !== "string" || row.id.length === 0) process.exit(3);
if (typeof row.host_id !== "string" || row.host_id.length === 0) process.exit(4);
' <<<"$SESSION_JSON"
echo "[e2e] ok: session get api"

# Trigger at least one tool call via hostd local unix API, then assert it is persisted and
# visible via messages rendering (system role).
curl --silent --show-error \
  --unix-socket "$SOCK_PATH" \
  "http://localhost/runs/$RUN_ID/fs/search?q=mock-codex" \
  >/dev/null

for _ in $(seq 1 200); do
  TOOL_CALLS="$(sqlite3 "$DB_PATH" "select count(*) from events where run_id='$RUN_ID' and type='tool.call' and instr(data_json,'\"tool\":\"fs.search\"')>0;")"
  TOOL_RESULTS="$(sqlite3 "$DB_PATH" "select count(*) from events where run_id='$RUN_ID' and type='tool.result' and instr(data_json,'\"tool\":\"fs.search\"')>0;")"
  if [[ "$TOOL_CALLS" -ge "1" && "$TOOL_RESULTS" -ge "1" ]]; then
    break
  fi
  sleep 0.05
done
TOOL_CALLS="$(sqlite3 "$DB_PATH" "select count(*) from events where run_id='$RUN_ID' and type='tool.call' and instr(data_json,'\"tool\":\"fs.search\"')>0;")"
TOOL_RESULTS="$(sqlite3 "$DB_PATH" "select count(*) from events where run_id='$RUN_ID' and type='tool.result' and instr(data_json,'\"tool\":\"fs.search\"')>0;")"
if [[ "$TOOL_CALLS" -lt "1" || "$TOOL_RESULTS" -lt "1" ]]; then
  echo "[e2e] expected tool.call/tool.result for fs.search, got call=$TOOL_CALLS result=$TOOL_RESULTS" >&2
  exit 1
fi
echo "[e2e] ok: tool events persisted"

# Trigger one WS-RPC tool call via the app websocket and assert it is persisted.
RPC_OUT="$(
  bun run cli/src/index.ts ws-rpc-fs-search \
    --server "http://127.0.0.1:$PORT" \
    --token "$TOKEN" \
    --run "$RUN_ID" \
    --q "mock-codex"
)"
RPC_OK="$(node -e 'const fs=require("fs");const s=fs.readFileSync(0,"utf8");const m=JSON.parse(s);process.stdout.write(String(Boolean(m?.data?.ok)));' <<<"$RPC_OUT")"
if [[ "$RPC_OK" != "true" ]]; then
  echo "[e2e] ws-rpc-fs-search failed: $RPC_OUT" >&2
  exit 1
fi

for _ in $(seq 1 200); do
  RPC_TOOL_CALLS="$(sqlite3 "$DB_PATH" "select count(*) from events where run_id='$RUN_ID' and type='tool.call' and instr(data_json,'\"tool\":\"rpc.fs.search\"')>0;")"
  RPC_TOOL_RESULTS="$(sqlite3 "$DB_PATH" "select count(*) from events where run_id='$RUN_ID' and type='tool.result' and instr(data_json,'\"tool\":\"rpc.fs.search\"')>0;")"
  if [[ "$RPC_TOOL_CALLS" -ge "1" && "$RPC_TOOL_RESULTS" -ge "1" ]]; then
    break
  fi
  sleep 0.05
done
RPC_TOOL_CALLS="$(sqlite3 "$DB_PATH" "select count(*) from events where run_id='$RUN_ID' and type='tool.call' and instr(data_json,'\"tool\":\"rpc.fs.search\"')>0;")"
RPC_TOOL_RESULTS="$(sqlite3 "$DB_PATH" "select count(*) from events where run_id='$RUN_ID' and type='tool.result' and instr(data_json,'\"tool\":\"rpc.fs.search\"')>0;")"
if [[ "$RPC_TOOL_CALLS" -lt "1" || "$RPC_TOOL_RESULTS" -lt "1" ]]; then
  echo "[e2e] expected WS-RPC tool.call/tool.result for rpc.fs.search, got call=$RPC_TOOL_CALLS result=$RPC_TOOL_RESULTS" >&2
  exit 1
fi
echo "[e2e] ok: ws-rpc tool events"

RUNS_LIST_OUT="$(
  bun run cli/src/index.ts ws-rpc-runs-list \
    --server "http://127.0.0.1:$PORT" \
    --token "$TOKEN" \
    --run "$RUN_ID"
)"
node -e '
const fs=require("fs");
const raw=fs.readFileSync(0,"utf8");
let m;
try { m = JSON.parse(raw); } catch (e) { console.error("bad json:", raw); process.exit(2); }
if (m?.type !== "rpc.response") { console.error("bad type:", m?.type); process.exit(3); }
if (m?.data?.ok !== true) { console.error("rpc not ok:", m?.data); process.exit(4); }
const runs = m?.data?.result?.runs;
if (!Array.isArray(runs)) { console.error("missing runs:", m?.data?.result); process.exit(5); }
' <<<"$RUNS_LIST_OUT" || { echo "[e2e] ws-rpc-runs-list failed: $RUNS_LIST_OUT" >&2; exit 1; }
echo "[e2e] ok: ws-rpc runs list"

FS_LIST_OUT="$(
  bun run cli/src/index.ts ws-rpc-fs-list \
    --server "http://127.0.0.1:$PORT" \
    --token "$TOKEN" \
    --run "$RUN_ID" \
    --path "."
)"
FS_LIST_OK="$(node -e 'const fs=require("fs");const s=fs.readFileSync(0,"utf8");const m=JSON.parse(s);process.stdout.write(String(Boolean(m?.data?.ok)));' <<<"$FS_LIST_OUT")"
if [[ "$FS_LIST_OK" != "true" ]]; then
  echo "[e2e] ws-rpc-fs-list failed: $FS_LIST_OUT" >&2
  exit 1
fi
for _ in $(seq 1 200); do
  LIST_TOOL_CALLS="$(sqlite3 "$DB_PATH" "select count(*) from events where run_id='$RUN_ID' and type='tool.call' and instr(data_json,'\"tool\":\"rpc.fs.list\"')>0;")"
  LIST_TOOL_RESULTS="$(sqlite3 "$DB_PATH" "select count(*) from events where run_id='$RUN_ID' and type='tool.result' and instr(data_json,'\"tool\":\"rpc.fs.list\"')>0;")"
  if [[ "$LIST_TOOL_CALLS" -ge "1" && "$LIST_TOOL_RESULTS" -ge "1" ]]; then
    break
  fi
  sleep 0.05
done
LIST_TOOL_CALLS="$(sqlite3 "$DB_PATH" "select count(*) from events where run_id='$RUN_ID' and type='tool.call' and instr(data_json,'\"tool\":\"rpc.fs.list\"')>0;")"
LIST_TOOL_RESULTS="$(sqlite3 "$DB_PATH" "select count(*) from events where run_id='$RUN_ID' and type='tool.result' and instr(data_json,'\"tool\":\"rpc.fs.list\"')>0;")"
if [[ "$LIST_TOOL_CALLS" -lt "1" || "$LIST_TOOL_RESULTS" -lt "1" ]]; then
  echo "[e2e] expected WS-RPC tool.call/tool.result for rpc.fs.list, got call=$LIST_TOOL_CALLS result=$LIST_TOOL_RESULTS" >&2
  exit 1
fi
echo "[e2e] ok: ws-rpc fs list"

# Send input over the same WS path used by the PWA, twice with same input_id (idempotency).
REQUEST_ID="$(
  for _ in $(seq 1 100); do
    ROWS="$(sqlite3 -json "$DB_PATH" "select data_json from events where run_id='$RUN_ID' and type='run.permission_requested' order by id desc limit 1;")"
    if [[ "$ROWS" != "[]" && -n "$ROWS" ]]; then
      ID="$(node -e 'const fs=require("fs");const rows=JSON.parse(fs.readFileSync(0,"utf8")||"[]");const dj=rows[0]?.data_json; if(!dj) process.exit(2); console.log(JSON.parse(dj).request_id||"");' <<<"$ROWS" || true)"
      if [[ -n "$ID" ]]; then
        echo "$ID"
        break
      fi
    fi
    sleep 0.05
  done
)"
[[ -n "$REQUEST_ID" ]] || { echo "[e2e] missing permission request_id" >&2; exit 1; }

bun run cli/src/index.ts ws-approve --server "http://127.0.0.1:$PORT" --token "$TOKEN" --run "$RUN_ID" --request-id "$REQUEST_ID" >/dev/null
bun run cli/src/index.ts ws-approve --server "http://127.0.0.1:$PORT" --token "$TOKEN" --run "$RUN_ID" --request-id "$REQUEST_ID" >/dev/null

sleep 0.5

COUNT="$(sqlite3 "$DB_PATH" "select count(*) from events where run_id='$RUN_ID' and type='run.input' and input_id='$REQUEST_ID';")"
if [[ "$COUNT" != "1" ]]; then
  echo "[e2e] expected 1 run.input for input_id, got $COUNT" >&2
  exit 1
fi

EXTRA_INPUT_ID="$(node -e 'const { randomUUID } = require("crypto"); console.log(randomUUID());')"
bun run cli/src/index.ts ws-send-input \
  --server "http://127.0.0.1:$PORT" \
  --token "$TOKEN" \
  --run "$RUN_ID" \
  --input-id "$EXTRA_INPUT_ID" \
  --text $'hello\n' >/dev/null

for _ in $(seq 1 100); do
  EXTRA_COUNT="$(sqlite3 "$DB_PATH" "select count(*) from events where run_id='$RUN_ID' and type='run.input' and input_id='$EXTRA_INPUT_ID';")"
  if [[ "$EXTRA_COUNT" == "1" ]]; then
    break
  fi
  sleep 0.05
done
EXTRA_COUNT="$(sqlite3 "$DB_PATH" "select count(*) from events where run_id='$RUN_ID' and type='run.input' and input_id='$EXTRA_INPUT_ID';")"
if [[ "$EXTRA_COUNT" != "1" ]]; then
  echo "[e2e] expected 1 extra run.input for ws-send-input, got $EXTRA_COUNT" >&2
  exit 1
fi

OUT_COUNT="$(sqlite3 "$DB_PATH" "select count(*) from events where run_id='$RUN_ID' and type='run.output' and instr(data_json,'echo: hello') > 0;")"
if [[ "$OUT_COUNT" -lt "1" ]]; then
  echo "[e2e] expected run.output to include mock echo for hello" >&2
  exit 1
fi
echo "[e2e] ok: ws-send-input"

MESSAGES_JSON="$(
  curl --silent --show-error \
    "http://127.0.0.1:$PORT/sessions/$RUN_ID/messages?limit=200" \
    -H "Authorization: Bearer $TOKEN"
)"
node -e '
const fs = require("fs");
const raw = fs.readFileSync(0, "utf8");
const msgs = JSON.parse(raw);
if (!Array.isArray(msgs)) process.exit(2);
const roles = new Set(msgs.map((m) => m && typeof m === "object" ? m.role : null));
if (!roles.has("assistant") || !roles.has("user")) {
  console.error("missing expected roles in messages:", Array.from(roles));
  process.exit(3);
}
const kinds = new Set(msgs.map((m) => m && typeof m === "object" ? m.kind : null));
if (!kinds.has("tool.call") || !kinds.has("tool.result")) {
  console.error("missing expected tool kinds in messages:", Array.from(kinds));
  process.exit(4);
}
' <<<"$MESSAGES_JSON"
echo "[e2e] ok: messages api"

echo "[e2e] ok: input idempotency"

# Shim recursion smoke: ensure hostd does not execute the shim itself (would recurse).
#
# This test uses an isolated HOME under the temp dir and a fake "real codex" binary.
SHIM_HOME="$TMP_DIR/shim-home"
SHIM_BIN="$SHIM_HOME/.local/bin"
REAL_BIN="$SHIM_HOME/real-bin"
SHIM_SOCK="$TMP_DIR/relay-hostd-shim.sock"
mkdir -p "$SHIM_BIN" "$REAL_BIN"

cat >"$REAL_BIN/codex" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
echo "[fake-codex] ready"
while IFS= read -r line; do
  echo "[fake-codex] echo: $line"
done
EOF
chmod +x "$REAL_BIN/codex"

# Install shim for codex only; it will record the real binary path into ~/.relay/bin-map.json (under SHIM_HOME).
HOME="$SHIM_HOME" PATH="$REAL_BIN:$PATH" bash scripts/install-shims.sh --dir "$SHIM_BIN" --tools codex >/dev/null

# Start a second hostd with PATH preferring the shim, but runner should use bin-map.json to execute REAL_BIN/codex.
if [[ -x "$HOSTD_BIN" ]]; then
  env \
    SERVER_BASE_URL="ws://127.0.0.1:$PORT" \
    HOST_ID="host-shim" \
    HOST_TOKEN="devtoken" \
    LOCAL_UNIX_SOCKET="$SHIM_SOCK" \
    SPOOL_DB_PATH="$TMP_DIR/hostd-spool-shim.db" \
    RUST_LOG="$RUST_LOG" \
    HOME="$SHIM_HOME" \
    PATH="$SHIM_BIN:$REAL_BIN:$PATH" \
    "$HOSTD_BIN" >"$TMP_DIR/hostd-shim.log" 2>&1 &
else
  env \
    SERVER_BASE_URL="ws://127.0.0.1:$PORT" \
    HOST_ID="host-shim" \
    HOST_TOKEN="devtoken" \
    LOCAL_UNIX_SOCKET="$SHIM_SOCK" \
    SPOOL_DB_PATH="$TMP_DIR/hostd-spool-shim.db" \
    RUST_LOG="$RUST_LOG" \
    HOME="$SHIM_HOME" \
    PATH="$SHIM_BIN:$REAL_BIN:$PATH" \
    cargo run -p relay-hostd >"$TMP_DIR/hostd-shim.log" 2>&1 &
fi
HOSTD_SHIM_PID=$!

for _ in $(seq 1 300); do
  if ! kill -0 "$HOSTD_SHIM_PID" >/dev/null 2>&1; then
    echo "[e2e] hostd(shim) exited early; logs:" >&2
    tail -n 200 "$TMP_DIR/hostd-shim.log" >&2 || true
    exit 1
  fi
  [[ -S "$SHIM_SOCK" ]] && break
  sleep 0.1
done
[[ -S "$SHIM_SOCK" ]] || {
  echo "[e2e] hostd(shim) unix socket not ready: $SHIM_SOCK" >&2
  tail -n 200 "$TMP_DIR/hostd-shim.log" >&2 || true
  exit 1
}

RUN2_JSON="$(
  curl --silent --show-error \
    --unix-socket "$SHIM_SOCK" \
    -X POST \
    "http://localhost/runs" \
    -H 'content-type: application/json' \
    --data-binary "{\"tool\":\"codex\",\"cmd\":\"\",\"cwd\":\"$ROOT\"}"
)"
RUN2_ID="$(node -e 'const fs=require("fs");const s=fs.readFileSync(0,"utf8");console.log(JSON.parse(s).run_id);' <<<"$RUN2_JSON")"
[[ -n "$RUN2_ID" ]] || { echo "[e2e] missing run_id from shim hostd" >&2; exit 1; }

SHIM_INPUT_ID="$(node -e 'const { randomUUID } = require("crypto"); console.log(randomUUID());')"
curl --silent --show-error \
  "http://127.0.0.1:$PORT/runs/$RUN2_ID/input" \
  -H "Authorization: Bearer $TOKEN" \
  -H 'content-type: application/json' \
  --data-binary "{\"input_id\":\"$SHIM_INPUT_ID\",\"actor\":\"e2e\",\"text\":\"hello\\n\"}" \
  >/dev/null

for _ in $(seq 1 200); do
  READY_COUNT="$(sqlite3 "$DB_PATH" "select count(*) from events where run_id='$RUN2_ID' and type='run.output' and instr(data_json,'[fake-codex] ready') > 0;")"
  ECHO_COUNT="$(sqlite3 "$DB_PATH" "select count(*) from events where run_id='$RUN2_ID' and type='run.output' and instr(data_json,'[fake-codex] echo: hello') > 0;")"
  if [[ "$READY_COUNT" -ge "1" && "$ECHO_COUNT" -ge "1" ]]; then
    break
  fi
  sleep 0.05
done
READY_COUNT="$(sqlite3 "$DB_PATH" "select count(*) from events where run_id='$RUN2_ID' and type='run.output' and instr(data_json,'[fake-codex] ready') > 0;")"
ECHO_COUNT="$(sqlite3 "$DB_PATH" "select count(*) from events where run_id='$RUN2_ID' and type='run.output' and instr(data_json,'[fake-codex] echo: hello') > 0;")"
if [[ "$READY_COUNT" -lt "1" || "$ECHO_COUNT" -lt "1" ]]; then
  echo "[e2e] shim recursion smoke failed (expected fake-codex output)" >&2
  echo "[e2e] hostd(shim) logs:" >&2
  tail -n 200 "$TMP_DIR/hostd-shim.log" >&2 || true
  exit 1
fi
kill "$HOSTD_SHIM_PID" >/dev/null 2>&1 || true
wait "$HOSTD_SHIM_PID" >/dev/null 2>&1 || true
echo "[e2e] ok: shim recursion guard"

echo "[e2e] ok: db=$DB_PATH spool=$SPOOL_DB_PATH"
