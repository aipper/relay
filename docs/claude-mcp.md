# Claude Code CLI + relay MCP bridge (MVP)

This repo supports running Claude Code CLI under `hostd` and exposing filesystem/git/shell tools to Claude via MCP, with approvals handled in the relay PWA.

## How it works

1. You start a run via `relay claude ...` (or the Bun CLI). `hostd` launches Claude with env like `RELAY_RUN_ID` and `RELAY_HOSTD_SOCK`.
2. Claude starts an MCP stdio server process: `relay mcp`.
3. `relay mcp` detects the env and forwards tool calls to the `hostd` unix socket API.
4. `hostd` emits `tool.call` / `tool.result`. For `fs_write` and `bash`, `hostd` emits `run.permission_requested` and waits for approve/deny.

## Configure Claude MCP server

When you start Claude via `relay-hostd`, hostd will (best-effort) inject a per-run `--mcp-config` to register `relay mcp` as an MCP server (no persistent changes).

- If your installed Claude CLI does **not** support `--mcp-config`, you can either upgrade Claude, or use the “persistent registration” steps below.
- To opt out of auto-injection, set `RELAY_CLAUDE_DISABLE_RELAY_MCP=1` in the `relay-hostd` environment.
- If your `relay` binary is not in `PATH`, hostd will try to use a sibling `relay` next to `relay-hostd`. You can override the command via `RELAY_MCP_COMMAND=/abs/path/to/relay`.

### Optional: persistent registration

Build `relay`:

```sh
cargo build -p relay-cli
```

Binary path:

- debug: `target/debug/relay`
- release: `target/release/relay`

Add MCP server (project scope recommended):

```sh
claude mcp add --scope project --transport stdio relay -- /ABS/PATH/TO/relay mcp
```

Verify:

```sh
claude mcp list
```

If your Claude CLI uses different flags, run `claude mcp add --help` and configure an stdio server named `relay` that executes: `/ABS/PATH/TO/relay mcp`.

## End-to-end verification

Start dev stack:

```sh
scripts/dev-up.sh --port 8787 --rust-log info
```

Start a Claude run (prints `run_id`):

```sh
target/debug/relay claude --sock ./.relay-tmp/relay-hostd-dev-8787.sock --cwd /path/to/project
```

In the PWA, open the run and confirm:

- `fs_read` / `fs_search` / `git_status` / `git_diff` produce `tool.call` + `tool.result`.
- `fs_write` / `bash` produces `run.permission_requested` (approve/deny in session detail), then `tool.result`.

## Tools exposed to Claude (MVP)

- `fs_read`, `fs_search`, `git_status`, `git_diff`
- `fs_write` (requires approval), `bash` (requires approval)

## Local fallback mode (no hostd)

`relay mcp` can run without `hostd` for read-only local operations:

```sh
relay mcp --root /path/to/project
```
