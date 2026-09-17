# Phase C evidence — approval structured `actions[]` one-round approve

## What changed (C1–C3 summary, read-only for C4)

C4 made **zero source changes** to `protocol/`, `hostd/`, `server/`, `web/`.
Only the assertion spec + this evidence file were touched (allowed: spec/evidence).

- **C1 protocol (additive only):** `protocol/src/lib.rs` (+75) — `PermissionAction{id,label,behavior?}`,
  `PermissionActionBehavior{Approve,Deny,Abort,Custom}`, `PermissionSuggestion{id?,label,text?}`,
  `PermissionRequestedData.actions/suggestions` (both `Option`, serde default+skip),
  `PermissionApproveData.selected_action_id` (Option); 2 compat tests
  (old-shape requested/approve parse without new fields, new fields omitted on serialize).
  `docs/protocol.md` (+5) — `actions[]`/`suggestions[]`/`selected_action_id` documented as additive.
- **C2 backend passthrough:** `hostd/src/run_manager.rs` (+66) — `default_permission_actions()`
  (approve/deny), `actions` on pending permission, emission at all 3 sites
  (run_manager PTY prompt ×2, `local_api.rs` fs_write/bash_run, `main.rs` elicitation path),
  `decide_permission_with_action()` mapping `selected_action_id`→behavior→PTY text
  (Approve→approve_text, Deny/Abort→deny_text, Custom/None→error; unknown id→error).
  `hostd/src/main.rs` (+25) — extracts `selected_action_id` (typed, fallback raw JSON)
  and calls `decide_permission_with_action`. Server passthrough by design:
  `run.permission_requested` is fanned out unmodified; `run.permission.approve/deny`
  persists `run.permission_decided` carrying `selected_action_id` (verified in DB below).
- **C3 PWA multi-button:** `web/src/lib/stores/types.ts` (+16) — `PermissionAction/PermissionSuggestion` types;
  `relay-store.svelte.ts` (+48) — `actions/suggestions` parsed on WS `run.permission_requested`
  and hydrated from messages, `sendDecision(decision, selectedActionId?)`,
  `sendActionDecision(actionId)` (behavior deny/abort→deny else approve);
  `ApprovalCard.svelte`/`ApprovalModal.svelte` (+13 each) — render one button per
  `actions[]` via `onSendAction`, fallback to legacy 拒绝/同意 when absent;
  `SessionDetail.svelte` (+2), `SessionsPage.svelte` (+1), `App.svelte` (+1) — `onSendAction` wiring.
- **Spec fix during C4 (evidence-only):** `evidence/phase-c-assert.spec.ts` (112→114 lines):
  (1) run-card selector `.session-item:hasText("待审批").first()` → `.session-item:hasText(MARK).first()`
  — stale `awaiting_approval` runs (permission_requested buried beyond messages limit=200)
  render no approval card and list order varies, so `.first()` was ambiguous;
  (2) card selector `.sessions-main .pinned-actions .approval-card` →
  `.sessions-main div[class*='approval-card']` for existence + modal
  `getByRole("dialog").getByRole("button",{name:"Approve"})` for the click —
  Svelte scopes classes (`approval-card s-xxx`), so a multi-class descendant chain
  never matches, and the approval modal auto-opens on select covering the inline card.
  Both changes are selector-only; asserted behavior is unchanged.

Full uncommitted diff stat at verify time (C1–C3 + Phase A/B carried):
`docs/protocol.md` +5, `hostd/src/local_api.rs` +12, `hostd/src/main.rs` +25,
`hostd/src/run_manager.rs` +66, `protocol/src/lib.rs` +75, `server/src/main.rs` +42
(Phase B coalescer), `web/src/App.svelte` +1, `ApprovalCard.svelte` +13,
`ApprovalModal.svelte` +13, `ChatInput.svelte` +22 (Phase A), `SessionDetail.svelte` +2,
`SessionsPage.svelte` +1, `relay-store.svelte.ts` +48, `types.ts` +16. No commit.

## Stack

- Isolated stack reused (already alive, not restarted by C4):
  server `127.0.0.1:8790` (pid 300866, `DATABASE_URL=.../dev.EhYXyaN2/server.db`) +
  hostd (pid 310446, `SERVER_BASE_URL=ws://127.0.0.1:8790`, `RELAY_OPENCODE_MODE=tui`) +
  web dev `5174` (vite pid 3654490, `RELAY_DEV_PROXY_TARGET=http://127.0.0.1:8790`).
- `curl --noproxy "*" http://127.0.0.1:8790/health` → `{"name":"relay-server","version":"0.1.0"}` exit 0.
- `curl -s -o /dev/null -w "%{http_code}" http://127.0.0.1:5174/` → `200`.
- Browser: `~/.cache/ms-playwright/chromium-1208/chrome-linux64/chrome`
  via `executablePath` + `--no-sandbox`. Same-origin proxy URL
  `http://127.0.0.1:5174` for login (avoids CORS). All waits marker-delimited, no sleep-sync.
- Run: `run-d427fb73-3669-497d-80d6-d11b8acd9179` (opencode tool,
  `bash /home/ab/code/relay/scripts/mock-codex.sh`, unique cwd `/tmp/phasec-cwd-phasec-1789624787`).
  NOTE: `--cmd` must be the absolute script path — bare `bash scripts/mock-codex.sh`
  resolves relative to hostd CWD and the run exits immediately without `permission_requested`.
- Spec: `evidence/phase-c-assert.spec.ts` (synced copy at `web/.aiws-tmp-pw/phase-c-assert.spec.ts`;
  `web/.aiws-tmp-pw/` is the pre-existing temp Playwright dir, config `timeout: 120000`).
- DB: `/home/ab/code/relay/.relay-tmp/dev.EhYXyaN2/server.db`.

## Test commands + exit codes

- `curl --noproxy "*" -s -m 5 http://127.0.0.1:8790/health` → 0, `{"name":"relay-server","version":"0.1.0"}`.
- `curl -s -m 5 -o /dev/null -w "%{http_code}" http://127.0.0.1:5174/` → `200`.
- `bun run cli/src/index.ts ws-start-run --server http://127.0.0.1:8790 --token $TOKEN --host-id host-dev --tool opencode --cmd "bash /home/ab/code/relay/scripts/mock-codex.sh" --cwd /tmp/phasec-cwd-$MARK` → 0, `{"run_id":"run-d427fb73-…"}`.
  (One earlier attempt with relative `--cmd "bash scripts/mock-codex.sh"` exited at once with no
  `permission_requested` — CLI itself errored `run cwd does not exist` on the first try before that;
  both were setup mistakes, not product failures.)
- `PHASE_C_RUN_ID=run-d427fb73… PHASE_C_DB=… PHASE_C_MARK=phasec-1789624787 ./node_modules/.bin/playwright test -c .aiws-tmp-pw/playwright.config.ts phase-c-assert.spec.ts` (in `web/`) → 0, `1 passed (4.3s)`.
- Two non-passing runs before the fix (both diagnosed, neither a product bug):
  (1) stale-card run → `1 failed`, `.pinned-actions .approval-card` not found (clicked wrong card);
  (2) after MARK-selector fix → `1 failed`, `Test timeout of 120000ms exceeded` at the card assert
  (Svelte scoped-class selector never matches; modal covers inline card).
- Cleanup: stale `run-fc3efba0…` denied via
  `bun run cli/src/index.ts ws-deny --server … --run run-fc3efba0… --request-id 6dae4a3b…` → 0 (`sent`);
  its `permission_requested` had scrolled beyond messages limit=200 so the PWA could no longer
  render its card — expected behavior, not a bug. No commits. `tasks.md`/`proposal.md` untouched.

## Playwright assertions (passing run, real isolated stack)

Flow: login page `连接到服务器` → custom URL `http://127.0.0.1:5174` → `已连接` badge →
session card matching MARK → `.sessions-main .detail-head` visible →
DB poll (15s deadline) finds `run.permission_requested` with `actions[]` →
modal `Approve` button visible+enabled → click →
DB poll (15s) finds `run.permission_decided` → DB poll (15s) finds mock approval echo.

- `actions=[{"behavior":"approve","id":"approve","label":"Approve"},{"behavior":"deny","id":"deny","label":"Deny"}] request_id=f60f6403-ba69-41ce-b3dc-be9a8531ab8b` —
  run reaches `awaiting_approval` with `actions[]` in DB: **pass**.
- `decided={"actor":"web","decision":"approve","request_id":"f60f6403-…","selected_action_id":"approve"}` —
  `run.permission_decided` lands in DB carrying the selected action id: **pass**
  (asserts `toContain("approve")` + `toContain("selected_action_id")`).
- `run.output` contains `[mock-codex] approved`, run status `running` —
  the run proceeds after the action-button decision: **pass**.
- Old-shape fallback (`{request_id:"x",prompt:"y"}` without `actions` renders legacy
  approve/deny — in-page unit check): **pass**.

## Cleanup notes

- Temp probe specs (probe/probe2/probe3/probe4/probe5/probe6) removed after use; only
  `phase-c-assert.spec.ts` + `playwright.config.ts` remain in `web/.aiws-tmp-pw/`.
- Isolated stack (dev-up 8790 + web 5174) left running for follow-up phases.
  No secrets stored in files. No commit. `tasks.md` not marked (per instructions).
