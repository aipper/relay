# Codex CLI + relay MCP bridge (MVP)

This repo can run Codex CLI under `hostd` and expose filesystem/git/shell tools to Codex via MCP, with approvals handled in the relay PWA.

## How it works

1. You start a run via `relay codex ...` (or the Bun CLI). `hostd` launches Codex with env like `RELAY_RUN_ID` and `RELAY_HOSTD_SOCK`.
2. Codex starts an MCP stdio server process: `relay mcp`.
3. `relay mcp` detects the env and forwards tool calls to the `hostd` unix socket API.
4. `hostd` emits `tool.call` / `tool.result`. For `fs_write` and `bash`, `hostd` emits `run.permission_requested` and waits for approve/deny.

## Configure Codex MCP server

When you start Codex via `relay-hostd`, hostd injects a per-run `--config` that registers `relay mcp` as an MCP server (no persistent changes to `~/.codex/config.toml`).

To opt out, set `RELAY_CODEX_DISABLE_RELAY_MCP=1` in the `relay-hostd` environment.

If your `relay` binary is not in `PATH`, hostd will try to use a sibling `relay` next to `relay-hostd`. You can override the command via `RELAY_MCP_COMMAND=/abs/path/to/relay`.

### Optional: persistent global registration

Build `relay`:

```sh
cargo build -p relay-cli
```

Binary path:

- debug: `target/debug/relay`
- release: `target/release/relay`

Register the MCP server in Codex:

```sh
codex mcp add relay -- /ABS/PATH/TO/relay mcp
```

Verify:

```sh
codex mcp list --json
codex mcp get relay --json
```

Rollback:

```sh
codex mcp remove relay
```

## End-to-end verification

Start dev stack:

```sh
scripts/dev-up.sh --port 8787 --rust-log info
```

Start a Codex run (prints `run_id`):

```sh
target/debug/relay codex --sock ./.relay-tmp/relay-hostd-dev-8787.sock --cwd /path/to/project
```

In Codex, explicitly instruct it to use the MCP tools from `relay` for any file writes or shell commands.

In the PWA, open the run and confirm:

- `fs_read` / `fs_search` / `git_status` / `git_diff` produce `tool.call` + `tool.result`.
- `fs_write` / `bash` produces `run.permission_requested` (approve/deny in session detail), then `tool.result`.

## Tools exposed to Codex (MVP)

- `fs_read`, `fs_search`, `git_status`, `git_diff`
- `fs_write` (requires approval), `bash` (requires approval)
