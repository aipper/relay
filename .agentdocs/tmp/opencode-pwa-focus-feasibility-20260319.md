# OpenCode-first PWA feasibility review (2026-03-19)

## Summary

- Feasibility: **high**, provided the project treats OpenCode session/todo APIs as first-class data instead of only replaying generic `run.output`.
- Current repo already has a strong base: `hostd` supports `opencode` structured mode, per-run `model` override, config-derived model discovery, and JSONL event mapping; `web` already exposes `opencode` in the start form and has a model picker; `server` already exposes generic `/sessions` and `/sessions/:id/messages`.
- Current repo is **not yet OpenCode-native**: the server/web domain model still centers on relay `run/session` aliases, not OpenCode's own `sessionID`; todo state is local-only in `web/src/App.svelte` and has no server-backed sync or auto-complete mechanism.

## Confirmed OpenCode capabilities (external evidence)

### Sessions and conversation history

- `opencode session list`
- `opencode run --continue`
- `opencode run --session <id>`
- `opencode run --session <id> --fork`
- `GET /session/:id/message`
- `POST /sessions`
- `POST /sessions/{sessionID}/fork`

Confirmed via Context7 / official docs snapshots retrieved during this session:

- CLI docs show `--continue`, `--session`, `--fork`, `--model`, `--format json`.
- Server / SDK docs show session create/list/get/messages/fork/share/update/archive/delete.

### Models

- Global config supports `model` and `small_model`.
- CLI supports per-run `--model provider/model` override.
- Command config can override `model` per command.
- `GET /config` exposes current global default model.

### Todos / task planning

- Official docs expose `GET /session/:id/todo`.
- OpenCode prompt/tooling includes task management via TodoWrite-style tools.
- Public examples and docs indicate todos are session-bound, not generic output-only hints.

## Current relay repository support (local evidence)

### hostd

- `hostd/src/run_manager.rs`
  - Reads OpenCode config from `XDG_CONFIG_HOME/opencode/opencode.json` or `~/.config/opencode/opencode.json`.
  - Extracts model choices via `opencode_model_choices()`.
  - Creates a temporary structured config that forces `share = disabled`, removes `plugin`, and optionally overrides `model`.
  - Stores in-memory `opencode_session_id` for session reuse within a run lifecycle.
  - Maps OpenCode JSONL events into relay events:
    - `text` -> `run.output`
    - `tool_use` -> `tool.call` + `tool.result`
    - `error` -> `run.output` on stderr
- `hostd/src/main.rs`
  - `rpc.host.info` returns OpenCode status, `models`, `default_model`, and `models_error`.
  - `rpc.run.start` accepts optional `model`.
- `hostd/src/local_api.rs`
  - local `POST /runs` accepts optional `model`.

### server

- `server/src/main.rs`
  - Exposes `/sessions`, `/sessions/recent`, `/sessions/:session_id`, `/sessions/:session_id/messages`, `/hosts`, `/ws/app`, `/ws/host`.
  - These are relay-generic session/message APIs, not OpenCode-native APIs.
- `server/src/db.rs`
  - Persists `hosts`, `runs`, `events`.
  - No `opencode_sessions`, `session_todos`, or other OpenCode-native tables exist.

### web / PWA

- `web/src/App.svelte`
  - Start form includes `opencode`.
  - Loads host-provided `models/default_model/models_error`.
  - Passes selected `model` into run start.
  - Session detail uses structured messages view by default.
  - Todo UI exists, but is stored in `localStorage` under `relay.todo.${runId}`.
  - Suggestions are extracted from output text (`TODO:` / `- [ ]`), not from structured task/todo APIs.
- `web/src/lib/blocks/reduce.ts`
  - Keeps `run.output` visible for `runTool === "opencode"`.
  - Supports pairing tool lifecycle blocks.

### cli

- `cli/src/index.ts`
  - Supports `--tool opencode` and `--model provider/model`.
  - Does not yet expose OpenCode-first operations such as session list/get/messages/todos.

## Main gaps blocking an OpenCode-first product

1. **OpenCode session identity is not persisted as a first-class field**.
   - `hostd` can reuse `opencode_session_id` in memory, but the value is not promoted into relay server/web/cli APIs.

2. **Server data model is still relay-generic**.
   - `/sessions` is a relay alias over `runs`, not a mapped OpenCode session resource.

3. **Todo state is browser-local only**.
   - No server-backed todo storage.
   - No sync across devices.
   - No automatic completion driven by structured OpenCode task/todo state.

4. **No OpenCode-native PWA affordance for session switching/new session/fork/history export**.
   - The pieces exist separately, but the information architecture is still generic run-centric.

5. **CLI is not OpenCode-first**.
   - It can launch OpenCode, but cannot inspect or manage OpenCode sessions/todos/history as first-class commands.

## Recommendation

- Treat this as a **requirements change first**, not just an implementation detail.
- The next implementation wave should elevate these concepts to first-class product primitives:
  - OpenCode `sessionID`
  - session list / switch / create / fork / history
  - session-scoped todo list
  - auto-complete driven by structured task/todo updates rather than raw output parsing

## Verdict

- The requested direction is implementable.
- The shortest safe path is **not** to replace relay with the official OpenCode web UI; it is to keep relay's host/server/PWA architecture and upgrade its domain model to mirror OpenCode's structured session/todo capabilities.
