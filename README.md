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

### Remote start (WebSocket RPC)

You can start a run on a specific host via WS-RPC. If `--cmd` is omitted, it defaults to the selected tool name (e.g. `codex`).

### Codex / Claude / iFlow (real)

If `codex` / `claude` / `iflow` is installed on the host machine, you can start it as a run by using:

```sh
cd cli
bun run dev codex --sock ./.relay-tmp/relay-hostd-dev-8787.sock
```

To override the binary path on the host, set `RELAY_CODEX_BIN=/path/to/codex` in the hostd environment (Codex only).

Similarly:
- `RELAY_CLAUDE_BIN=/path/to/claude`
- `RELAY_IFLOW_BIN=/path/to/iflow`

### Run `codex` / `claude` / `iflow` directly in any project (no Bun)

If you use the packaged binaries (or have `relay` in PATH) and a background `relay-hostd` running,
you can install command shims so that running `codex` in a project directory starts a relay run
with `cwd = $PWD`:

```sh
bash scripts/install-shims.sh --auto-path
```

This writes:
- shims to `~/.local/bin/` (make sure it's in PATH)
- a binary map at `~/.relay/bin-map.json` so hostd can execute the *real* binaries and avoid recursion

Default unix socket path:
- `~/.relay/relay-hostd.sock` (unless `LOCAL_UNIX_SOCKET` overrides it)

Uninstall:

```sh
bash scripts/install-shims.sh --uninstall
```

If you used `--auto-path`, uninstall also removes the PATH block it added (with a backup of your rc file).

### Offline replay (hostd spool)

`hostd` persists its outgoing events to a local SQLite spool DB and replays them on reconnect.

- Configure with `SPOOL_DB_PATH` (default: `data/hostd-spool.db`)

## E2E Smoke Test (dev)

Runs `relay-server` + `relay-hostd`, starts a dummy interactive run, sends input twice (same `input_id`), and asserts idempotency in SQLite:

```sh
scripts/e2e.sh
```

## Messages API (M5)

The server can expose a minimal chat-style message stream derived from `events`:

- `GET /runs/:run_id/messages?limit=200[&before_id=...]` (Bearer auth)

This is used by the web UI's `Messages` section for a threaded view.

## Packaging (quick path)

Creates a distributable directory under `./dist/` with `up.sh` (starts server + hostd, default password `123456`):

```sh
bash scripts/package.sh --no-web
```

If you have Bun deps installed (or want to try building web assets), you can include web static assets:

```sh
bash scripts/package.sh --with-web
```

The packaged directory includes a Rust CLI binary (`bin/relay`) so you can run:

```sh
./dist/<package>/up.sh --port 8787
./dist/<package>/bin/relay codex --cwd /path/to/project
```

For "run `codex` in any project dir" style usage, the packaged directory includes `install-shims.sh`:

```sh
./dist/<package>/install-shims.sh --auto-path
```

## Deploy relay-server on a VPS (Docker)

This deploys the central `relay-server` and serves the PWA (static assets) from the same container. `relay-hostd` should run on each client machine that needs to run Codex/Claude/iFlow.

See `docs/deploy.md` for a full VPS + client guide.

1) Create env file:

```sh
cp docker/server.env.example docker/server.env
```

2) Edit `docker/server.env` (set a strong `JWT_SECRET`; set `ADMIN_PASSWORD` or `ADMIN_PASSWORD_HASH`).

3) Start:

```sh
docker compose up -d --build
```

4) Verify:

```sh
curl -s http://127.0.0.1:8787/health
```

PWA:
- Open `http://<your-vps>:8787/` in a browser.

## Package relay-hostd + relay CLI for a client machine (no Bun)

Builds a self-contained client bundle under `./dist/` with:
- `bin/relay-hostd` (daemon)
- `bin/relay` (CLI)
- `hostd-up.sh` (connects to a remote server)
- `install-shims.sh` (optional codex/claude/iflow shims)

```sh
bash scripts/package-client.sh
```
