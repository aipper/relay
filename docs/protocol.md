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
