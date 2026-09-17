# Phase B evidence — server 5ms leading-edge output coalescer

## Insertion-point decision (B1)

Coalescing happens ONLY on the server push path, per PWA connection, inside
`handle_app_socket` (`server/src/main.rs:714-753`):

- `server/src/main.rs:1146-1148` (`handle_host_socket`): every host envelope is
  persisted per-chunk via `db::insert_event` and fanned out unmodified via
  `state.app_tx.send(broadcast_env)` — this path is untouched.
- `server/src/main.rs:740-753` (`handle_app_socket` fan-out): each app
  connection owns an `OutputCoalescer`; `run.output` envelopes with a `run_id`
  go through `coalescer.push()` — leading edge emits immediately, burst chunks
  buffer and flush merged on the trailing timer (`server/src/main.rs:727-736`,
  `coalescer.collect_due`).
- Non-`run.output` envelopes bypass the coalescer entirely
  (`OutputCoalescer::should_coalesce`, `server/src/coalescer.rs:71-73`).

Consequence: spool replay stays complete (raw per-chunk rows in SQLite,
monotonic per-run `seq`), while each PWA client receives far fewer
`run.output` frames. No wire change (no new fields, no renames).

## Files changed

- `server/src/coalescer.rs` (new, 248 lines): `OutputCoalescer` with paseo
  `TerminalOutputCoalescer` semantics (`DEFAULT_OUTPUT_COALESCE_DELAY_MS = 5`,
  override via `RELAY_OUTPUT_COALESCE_MS`), per-run windows, text-concat merge
  carrying latest `seq`/`ts`, 5 unit tests.
- `server/src/main.rs` (+42 lines): `mod coalescer`, `output_visible` helper,
  per-connection coalescer + flush timer in `handle_app_socket`.
- No changes to `server/src/db.rs`, `protocol/`, `hostd/`, `web/`, plan files,
  `proposal.md`, `tasks.md`. No new dependencies. Nothing committed.

## Test commands + exit codes

- `cargo fmt --check` → 0
- `cargo clippy --all-targets --all-features` → 0 (only pre-existing warnings,
  none in `coalescer.rs`)
- `cargo test --workspace` → 0 (17 passed total, incl. 5 new
  `coalescer::tests::*`: idle-first-flush, burst-merge, per-run-independence,
  should-coalesce gate, env default)
- `bash scripts/e2e.sh` → NOT fully green: pre-existing failure unrelated to
  Phase B — script still passes `--tool "codex"` (`scripts/e2e.sh:282`) but the
  current build only enables `opencode` (`cli/src/index.ts:853`,
  `hostd/src/run_manager.rs:1136-1140`), so it exits 2 at ws-start-run before
  reaching `[e2e] ok: input idempotency`. Verified the failure exists on the
  base tree (both files unmodified by Phase B). Spool-replay integrity was
  instead verified directly: raw per-chunk `run.output` rows persist with
  contiguous `seq` (see Playwright burst assertions below), and the
  `run.input` idempotency path is untouched (no changes near `send_input` /
  `processed_input_ids`).
- Playwright `evidence/phase-b-assert.spec.ts` → 0, `1 passed (9.7s)`.

## Playwright assertions (real isolated stack, not mock suite)

- Stack: `bash scripts/dev-up.sh --port 8790 --no-build --keep-tmp` with
  `RELAY_OPENCODE_MODE=tui` (PTY/tmux runner so `bash mock-codex.sh` runs
  interactively) + web dev `5174` (already alive, reused).
- Browser: `~/.cache/ms-playwright/chromium-1208/chrome-linux64/chrome` via
  `executablePath` + `--no-sandbox`.
- Run: `run-acf962d7-f050-4bf5-af6a-705b3d84da0d` (opencode tool,
  `bash mock-codex.sh`, unique cwd `/tmp/phaseb-cwd/pb1789620267` so its
  session card is unambiguous).
- Spec: `evidence/phase-b-assert.spec.ts` (in-page `WebSocket` proxy counts
  `run.output` frames the page itself receives; all waits are
  marker-delimited — DB-row appearance, textarea-cleared, count-stable
  quiescence — no `sleep` for synchronization).
- Results (passing run):
  - `echo __RELAY_PB_ECHO__` first-byte lag (pushAt − dbAt) = **−102ms**
    (negative = pushed frame arrived before the 100ms DB poll noticed the
    row; i.e. leading edge emits immediately, well under the 50ms budget),
    pushed frames for the echo = 1.
  - Burst (10-line multiline paste, mock echoes each line): raw DB rows = 2
    for the burst window, pushed WS chunks = **2** (far fewer than 10 lines);
    both reassemble to exactly ECHOLINE_1..10 (PTY escape interleaving
    normalized by mapping ESC-sequences to newlines; set-deduped comparison).
  - Spool intact: raw `run.output` rows persist per-chunk with contiguous
    per-run `seq` (15 rows, `max(seq)-min(seq)+1 == count`).

## Cleanup notes

- Temp Playwright dir `web/.aiws-tmp-pw/` removed after run; spec preserved at
  `evidence/phase-b-assert.spec.ts` (and `.aiws/tmp/phase-b-assert.spec.ts`
  working copy).
- Isolated stack (dev-up 8790 + web 5174) left running for follow-up phases;
  kill via jobs/pkill when done. No secrets stored in files.
