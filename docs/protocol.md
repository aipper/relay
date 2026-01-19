# Protocol (MVP)

This document defines the minimal wire protocol between:

- `hostd` ↔ `server` (WebSocket, outbound from host to server)
- `web` ↔ `server` (WebSocket, from browser/PWA to server)

## Identifiers

- `host_id`: unique host machine identifier
- `run_id`: unique run/session identifier (scoped globally)
- `seq`: monotonically increasing per `run_id` event sequence
- `input_id`: client-generated UUID to make inputs idempotent

## Event Envelope

All WebSocket messages use a JSON envelope:

```json
{
  "type": "run.output",
  "ts": "2025-01-01T00:00:00Z",
  "host_id": "host_...",
  "run_id": "run_...",
  "seq": 123,
  "data": {}
}
```

Rules:

- Fields may be added over time; unknown fields must be ignored.
- `type` is a stable string; do not rename existing types.

## WebSocket Endpoints (MVP)

- App/PWA: `GET /ws/app?token=<access_token>`
- Host daemon: `GET /ws/host?host_id=<id>&host_token=<token>`

Notes:

- Browsers cannot set custom WS headers; the app uses query params for auth in MVP.
- The host connection is outbound from `hostd` to `server` (works behind NAT).
- Host authentication uses **TOFU (Trust On First Use)**:
  - On first successful connection for a given `host_id`, the server stores `sha256(host_token)` in the DB.
  - Subsequent connections for the same `host_id` must present the same `host_token` (token rotation requires a new `host_id` or deleting the host record).
  - `host_token` is never stored in plaintext.

## HTTP Endpoints (MVP)

- `POST /auth/login` → `{ "access_token": "..." }`
- `POST /runs/:run_id/input` (Bearer auth) → forwards `run.send_input` to the owning host

## Events (hostd → server → web)

### `run.started`

`data`:

- `tool`: `codex | claude | iflow | ...`
- `cwd`: working directory path
- `command`: full command line (string)

### `run.output`

`data`:

- `stream`: `stdout | stderr`
- `text`: output chunk (utf-8)

### `run.awaiting_input`

`data`:

- `reason`: `permission | choice | prompt | unknown`
- `prompt`: optional short prompt extracted from output
- `request_id`: optional UUID for structured approvals (see `run.permission_requested`)

### `run.exited`

`data`:

- `exit_code`: integer

### `run.input` (recorded)

This is emitted after an input is accepted and written to the PTY.

`data`:

- `actor`: `web | cli | system`
- `input_id`: UUID (for idempotency/auditing)
- `text_redacted`: redacted input text for UI replay
- `text_sha256`: sha256 of the raw input (hex)

## Commands (web/cli → server → hostd)

### `run.send_input`

`data`:

- `input_id`: UUID
- `text`: raw text to write to PTY stdin (should usually end with `\n`)

### `run.stop`

`data`:

- `signal`: `term | kill` (default `term`)

## Acknowledgements (server ↔ hostd)

### `run.ack`

Used for offline spool replay.

`data`:

- `run_id`: the run being acknowledged
- `last_seq`: last persisted `seq` for this `run_id`

Delivery:

- Server sends `run.ack` back to the connected host after persisting a host→server event with a `seq`.
- Host uses this to delete spooled events up to `last_seq` and to resume replay after reconnect.

## Approvals (M3)

This section adds a structured approval flow on top of `run.awaiting_input`.

### `run.permission_requested` (hostd → server → web)

Emitted when a run needs an explicit user decision (approve/deny) to proceed.

`data`:

- `request_id`: UUID (stable identifier for the request)
- `reason`: `permission | choice | prompt | unknown`
- `prompt`: short prompt text for UI display
- `op_tool`: optional operation tool name for UI/risk hints (e.g. `rpc.fs.read`, `rpc.fs.write`, `bash`)
- `op_args`: optional full operation args (JSON object/value) for “view full args” UI; should be redacted/truncated as needed
- `op_args_summary`: optional short args summary for list/cards (recommended max 80 chars)
- `approve_text`: suggested text to write to PTY on approve (e.g. `"y\n"`)
- `deny_text`: suggested text to write to PTY on deny (e.g. `"n\n"`)

### `run.permission.approve` / `run.permission.deny` (web/cli → server → hostd)

Structured decisions for a previous `run.permission_requested`.

`data`:

- `request_id`: UUID
- `actor`: `web | cli | system` (optional)

## WS-RPC (M4-A2)

WS-RPC provides request/response operations over the existing WS paths:

- web/cli → server → hostd: `rpc.*` requests
- hostd → server → web/cli: `rpc.response` responses

All RPC requests MUST include a `request_id` (UUID) in `data`. Responses echo the same `request_id`.

## Tool Events (H3)

To make tool usage auditable and renderable as a message thread, `hostd` emits tool lifecycle events
in addition to (and compatible with) `rpc.response`.

### `tool.call` (hostd → server → web)

Emitted when `hostd` is about to perform a tool operation (via WS-RPC or local unix API).

`data`:

- `request_id`: UUID (correlates call/result)
- `tool`: tool identifier (e.g. `rpc.fs.read`, `fs.search`, `git.diff`)
- `actor`: `local | web | cli | system` (best-effort; informational)
- `args`: arbitrary JSON arguments (tool-specific)

### `tool.result` (hostd → server → web)

Emitted after a `tool.call` completes.

`data`:

- `request_id`: UUID (correlates call/result)
- `tool`: same tool identifier as the call
- `actor`: `local | web | cli | system` (best-effort; informational)
- `ok`: boolean
- `duration_ms`: integer
- `result`: arbitrary JSON (present when `ok=true`)
- `error`: string (present when `ok=false`)

### `rpc.fs.read` (web/cli → server → hostd)

Read a UTF-8 file relative to the run's `cwd` (hostd enforces scope; absolute paths are rejected).

`data`:

- `request_id`: UUID
- `path`: relative file path (e.g. `"README.md"`)

### `rpc.fs.search` (web/cli → server → hostd)

Search within the run's `cwd` using ripgrep (`rg` must exist on the host).

`data`:

- `request_id`: UUID
- `q`: query string

### `rpc.fs.list` (web/cli → server → hostd)

List a directory under the run's `cwd` (hostd enforces scope; absolute paths are rejected).

`data`:

- `request_id`: UUID
- `path`: optional relative dir path (default `"."`)

Response `result`:

- `path`: same as request (string)
- `entries`: array of `{ name, is_dir, size_bytes }` (size is only populated for regular files)

### `rpc.fs.write` (web/cli → server → hostd)

Write a UTF-8 file relative to the run's `cwd`.

Notes:

- This operation is **permission-gated**: hostd emits `run.permission_requested` and waits for `run.permission.approve` / `run.permission.deny`.
- For safety/UI, `run.permission_requested.data.op_args` may contain only a redacted/truncated preview rather than full content.

`data`:

- `request_id`: UUID
- `path`: relative file path (e.g. `"README.md"`)
- `content`: UTF-8 content to write (hostd may truncate to a max size)

### `rpc.bash` (web/cli → server → hostd)

Execute a shell command (`bash -lc`) under the run's `cwd`.

Notes:

- This operation is **permission-gated**: hostd emits `run.permission_requested` and waits for `run.permission.approve` / `run.permission.deny`.
- Outputs are truncated for UI/storage.

`data`:

- `request_id`: UUID
- `cmd`: command string to execute
- `truncated`: boolean (when entry count hits max)

### `rpc.git.status` / `rpc.git.diff` (web/cli → server → hostd)

Run `git status` / `git diff` in the run's `cwd`.

`rpc.git.diff` `data`:

- `request_id`: UUID
- `path`: optional relative file path filter

### `rpc.response` (hostd → server → web/cli)

`data`:

- `request_id`: UUID
- `ok`: boolean
- `rpc_type`: the request type string (e.g. `"rpc.fs.read"`)
- `result`: arbitrary JSON (when `ok=true`)
- `error`: string (when `ok=false`)

### `rpc.run.start` (web/cli → server → hostd)

Start a new run on a specific host. This RPC does **not** require `run_id` in the envelope.

`data`:

- `request_id`: UUID
- `host_id`: target host machine id
- `tool`: `codex | claude | iflow | gemini | ...`
- `cmd`: command to run (string, executed via `bash -lc`)
- `cwd`: optional working directory on the host (string or null)

Response:

- `rpc.response` with `run_id` set in the envelope and `data.result.run_id`

### `rpc.host.info` / `rpc.host.doctor` (web/cli → server → hostd)

Fetch host/machine information and basic health checks from a specific host. These RPCs do **not**
require `run_id` in the envelope.

`data`:

- `request_id`: UUID
- `host_id`: target host machine id

Response:

- `rpc.response` with `data.result` containing host fields and tool/dependency statuses

### `rpc.host.capabilities` (web/cli → server → hostd)

Return a structured capability manifest for the target host (supported RPC types, tools, deps, local socket path, etc).

`data`:

- `request_id`: UUID
- `host_id`: target host machine id

### `rpc.host.logs.tail` (web/cli → server → hostd)

Read the tail of the hostd log file for remote debugging. The host must set `HOSTD_LOG_PATH`.

`data`:

- `request_id`: UUID
- `host_id`: target host machine id
- `lines`: optional integer (default 200, max 2000)
- `max_bytes`: optional integer (default 200000, max 2000000)

Response `result`:

- `path`: log file path
- `text`: last N lines of text
- `truncated`: boolean (when file exceeds `max_bytes`)

### `rpc.runs.list` (web/cli → server → hostd)

List currently running runs on the host that owns the selected `run_id`.

`data`:

- `request_id`: UUID

Response:

- `rpc.response` with `data.result.runs` (array)

### `rpc.run.stop` (web/cli → server → hostd)

Stop a run with an explicit ack response (request/response style).

`data`:

- `request_id`: UUID
- `signal`: `term | kill` (default `term`)
- `actor`: `web | cli | system` (optional)
