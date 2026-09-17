# Phase A evidence — PWA local slash interception

## What changed
- `web/src/lib/components/ChatInput.svelte` only (+22 lines):
  - Added `LOCAL_COMMANDS = {"/quit","/clear"}` exact-match set.
  - `isLocalCommand()` trims then exact-matches (no-args by construction; no attachments prop exists so vacuously true).
  - `handleSend()` intercepts before `onSendChatInput()`; `handleLocalCommand()` clears draft locally and never calls the send path, so nothing reaches `run.send_input`.
  - Normal text path untouched: still `onSendChatInput(localText)` → `relay.sendChatInput` → `run.send_input`.
- No changes to `server/`, `hostd/`, `protocol/`, `docs/protocol.md`, proposal.md, tasks.md. No commit. No new dependencies.

## Build
- `cd web && bun run build` → exit 0 (`✓ built in 2.68s`, PWA precache 5 entries). Only pre-existing Svelte a11y/non-reactive warnings.

## Playwright assertion (real isolated stack, not mock suite)
- Stack: `bash scripts/dev-up.sh --port 8790 --no-build --keep-tmp` (server.db=`dev.GryqYPL1/server.db`) + web dev `RELAY_DEV_PROXY_TARGET=http://127.0.0.1:8790 bun run dev --port 5174`.
- Browser: `~/.cache/ms-playwright/chromium-1208/chrome-linux64/chrome` via `executablePath` + `--no-sandbox`.
- Run: `run-73e271a9-a99d-4ce2-860c-9983c5d2431d` (opencode tool, `bash scripts/mock-codex.sh`).
- Spec: `.aiws/tmp/phase-a-assert.spec.ts` (copied to `web/.aiws-tmp-pw/` for the run; temp dir removed afterwards).
- Flow (marker-delimited waits, no sleep): login page `连接到服务器` → custom URL `http://127.0.0.1:5174` (same-origin proxy avoids CORS) → `已连接` badge → session card → enabled `.chat-textarea` → fill `/quit` + 发送 → textarea cleared → DB poll 3s deadline shows `run.input` count unchanged → fill `hello-phase-a` + 发送 → textarea cleared → DB poll 10s deadline shows count +1.
- Result: `1 passed (33.0s)`, exit 0.
- DB post-check: `select count(*) ... type='run.input'` = 1; latest row `text_redacted='hello-phase-a'` — proves `/quit` created zero rows while normal text sent.

## Test commands + exit codes
- `cd web && bun run build` → 0
- `./node_modules/.bin/playwright test -c .aiws-tmp-pw/playwright.config.ts` (in `web/`) → 0, `1 passed`

## Cleanup notes
- Temp Playwright dir `web/.aiws-tmp-pw/` removed after run; spec preserved at `.aiws/tmp/phase-a-assert.spec.ts`.
- Isolated stack (dev-up 8790 + web 5174) left running for follow-up phases; kill via jobs/pkill when done. No secrets stored in files.
