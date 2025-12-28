# relay (Rust + Bun + PWA)

Run AI coding CLIs (Codex / Claude / iFlow, etc.) on host machines, and monitor/control them remotely from a mobile-friendly PWA.

## Components

- `server/`: central server (auth, routing, SQLite storage, WebSocket)
- `hostd/`: host daemon (PTY runner, event spool, outbound WS to server)
- `cli/`: Bun CLI (starts runs via `hostd`)
- `web/`: Svelte PWA (live runs/logs, approve/deny/input)

## Quick start (dev)

This repo is under active development. See `docs/protocol.md` for the event model.

### Server

1. Create `conf/env` from `conf/env.example`
2. Generate an Argon2 password hash:

```sh
cargo run -p relay-server -- --hash-password 'your-password'
```

3. Run:

```sh
set -a; source conf/env; set +a
cargo run -p relay-server
```

### Host daemon (skeleton)

```sh
SERVER_BASE_URL=ws://127.0.0.1:8787 HOST_ID=host-dev HOST_TOKEN=dev-token LOCAL_UNIX_SOCKET=/tmp/relay-hostd.sock cargo run -p relay-hostd
```

### Web (Svelte PWA skeleton)

```sh
cd web
bun install
bun run dev
```

### CLI (skeleton)

```sh
cd cli
bun run dev login --server http://127.0.0.1:8787 --username admin --password '...'
```

### Remote input (WebSocket)

After you have a `run_id` and a login token:

```sh
cd cli
bun run dev ws-send-input --server http://127.0.0.1:8787 --token <jwt> --run <run_id> --text "y\n"
```

### Offline replay (hostd spool)

`hostd` persists its outgoing events to a local SQLite spool DB and replays them on reconnect.

- Configure with `SPOOL_DB_PATH` (default: `data/hostd-spool.db`)

## E2E Smoke Test (dev)

Runs `relay-server` + `relay-hostd`, starts a dummy interactive run, sends input twice (same `input_id`), and asserts idempotency in SQLite:

```sh
scripts/e2e.sh
```
