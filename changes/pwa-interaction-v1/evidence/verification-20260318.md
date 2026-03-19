# Verification Evidence — 2026-03-18

## Scope covered

- `server/src/main.rs` / `server/src/db.rs`: event-view `include_output` gating for history + live WS fan-out
- `web/src/App.svelte`: host-aware start form, `opencode` model selection, cwd guardrails, event/output view split
- `hostd/src/run_manager.rs`: `opencode` structured config isolation + model override; restore default `codex` mode to `tui`
- `hostd/src/main.rs` / `hostd/src/local_api.rs` / `cli/src/index.ts`: model passthrough and host info metadata

## Commands executed

```bash
cargo test -p relay-server
cargo test -p relay-hostd
cargo build -p relay-server -p relay-hostd
cd web && bun run build
aiws change sync pwa-interaction-v1
aiws validate .
aiws validate . --stamp
bash scripts/e2e.sh
```

## Results

- `cargo test -p relay-server`: passed
- `cargo test -p relay-hostd`: passed
- `cargo build -p relay-server -p relay-hostd`: passed
- `cd web && bun run build`: passed
- `aiws change sync pwa-interaction-v1`: passed
- `aiws validate .`: passed after artifact alignment
- `aiws validate . --stamp`: passed
- `bash scripts/e2e.sh`: passed after restoring default `RELAY_CODEX_MODE` behavior to `tui`

## Key runtime checks

- TUI tools no longer stream bulk `run.output` into event view unless explicitly requested.
- PWA assets (`manifest.webmanifest`, `sw.js`, `registerSW.js`) are generated again and served with non-stale cache policy.
- `opencode` structured runs now isolate user config, disable `share`, remove `plugin`, set `stdin` to null, and accept per-run `model` override.
- `rpc.host.info` / `rpc.host.doctor`, hostd local API, web start form, and CLI start path all expose or pass the `model` override needed for `opencode`.
- `scripts/e2e.sh` end-to-end approval + idempotent input path passes again with mock codex.

## Repo-local evidence references

- AIWS validation stamp: `.agentdocs/tmp/aiws-validate/20260318-152931701Z.json`
- AIWS change sync stamp: `.agentdocs/tmp/change-sync/20260318-151930Z-pwa-interaction-v1.json`

## Notes

- The e2e regression root cause was a drift in `hostd/src/run_manager.rs`: `codex_mode_setting()` defaulted to `Structured`, while truth/docs require default `tui`. Restoring the default to `tui` brought back the PTY prompt flow used by `scripts/mock-codex.sh`.
