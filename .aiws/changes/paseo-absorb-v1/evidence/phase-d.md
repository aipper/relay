# Phase D evidence — WS-kill-mid-burst reconnect: no seq gap/duplicates

## Scope

Phase D made **zero source changes** to `protocol/`, `hostd/`, `server/`, `web/`.
Only the assertion spec + this evidence file were touched (allowed: spec/evidence).
All `server/` + `web/` diffs in the working tree are carried D1/D2 work from
prior phases, not Phase D edits.

- D1 (carried): `server/src/db.rs` `after_seq` filter in `list_message_events`
  + `server/src/main.rs` `MessagesQuery.after_seq` plumbing + `docs/protocol.md`.
- D2 (carried): `web/src/lib/stores/relay-store.svelte.ts` `maxSeqByRun` +
  `backfillAfterReconnect` + seq dedupe.

## Stack setup

- Server `127.0.0.1:8790`, fresh binary rebuilt from working tree (contains D1
  `after_seq` filter), restarted pid 893066:
  `DATABASE_URL=sqlite:.relay-tmp/dev.EhYXyaN2/server.db BIND_ADDR=127.0.0.1:8790
  JWT_SECRET=dev-jwt-secret-phasec-1234567890abcdef` + dev admin hash.
  `curl --noproxy '*' http://127.0.0.1:8790/health` →
  `{"name":"relay-server","version":"0.1.0"}` exit 0.
- hostd pid 863285, `SERVER_BASE_URL=ws://127.0.0.1:8790`,
  `RELAY_OPENCODE_MODE=tui`, same unix socket + spool DB as the 8790 stack.
  `curl --unix-socket .relay-tmp/relay-hostd-dev-8790.sock http://localhost/runs` → 200.
- Web dev `5174` reused (already alive, vite pid 3654490,
  `RELAY_DEV_PROXY_TARGET=http://127.0.0.1:8790`).
  `curl -s -o /dev/null -w "%{http_code}" http://127.0.0.1:5174/` → `200`.
- Browser: `~/.cache/ms-playwright/chromium-1208/chrome-linux64/chrome`
  via `executablePath` + `--no-sandbox`. Same-origin proxy URL
  `http://127.0.0.1:5174` for login. All waits marker-delimited, no sleep-sync.
- Run: `run-40262e21-98fe-4ac2-87e1-906578f0fec1` (opencode tool,
  `bash /home/ab/code/relay/scripts/mock-codex.sh`,
  unique cwd `/tmp/phased-cwd-phased-1789631191`).
  Created via hostd unix socket (not server WS-RPC — the CLI `ws-start-run`
  path hangs when the host WS is backlogged; socket POST is deterministic).
- DB: `/home/ab/code/relay/.relay-tmp/dev.EhYXyaN2/server.db`.
- Spec: `.aiws/changes/paseo-absorb-v1/evidence/phase-d-assert.spec.ts`
  (synced copy at `web/.aiws-tmp-pw/phase-d-assert.spec.ts`; `diff -q` → IN_SYNC).

## WS-kill method (deterministic, documented)

In-page `WebSocket` proxy installed via `addInitScript` **before** any page
script runs: records every envelope `seq` for our `run_id` seen on the page
socket (`__phaseD.wsSeqs`), tracks all sockets. Kill switch
`__phaseDKillWs()` closes sockets whose URL contains `/ws/app` with code 1000
— i.e. **page-side close of the PWA app socket only**. Server and hostd keep
running; the burst continues server-side while the page is offline.
Reconnect uses the real UI path: offline banner (`离线`) → `重连` button →
`relay.resumeFromStoredToken` → `#openAppWebSocket` → `onopen` resubscribes +
`backfillAfterReconnect(runId)` with `after_seq` = pre-kill `maxSeqByRun`.

Burst injection goes through hostd's unix socket
(`POST /runs/:id/input` × 20, unique `D3BURST_<ts>_LINE_<i>` lines,
fire-and-forget, never blocks) — NOT the server HTTP input endpoint, which
blocks on host delivery while the spec needs line injection during the kill
window. The mock echoes each line; hostd assigns monotonic host seq; server
persists per-chunk (`INSERT OR IGNORE` on `(run_id, seq)` unique index) and
the `after_seq` messages API serves the backfill tail.

## Test commands + exit codes

- `curl -s --noproxy '*' http://127.0.0.1:8790/health` → 0.
- `curl -s -o /dev/null -w "%{http_code}" http://127.0.0.1:5174/` → `200`.
- `curl --unix-socket .relay-tmp/relay-hostd-dev-8790.sock -X POST
  http://localhost/runs -d '{"tool":"opencode","cmd":"bash .../mock-codex.sh",
  "cwd":"/tmp/phased-cwd-phased-1789631191"}'` → 0,
  `{"run_id":"run-40262e21-…"}`.
- `curl --unix-socket ... -X POST .../runs/<id>/input -d '{"input_id":"approve-fresh2",
  "text":"y\\n"}'` → `HTTP:204`; DB poll finds `[mock-codex] approved`.
- `PHASE_D_RUN_ID=run-40262e21… PHASE_D_DB=… PHASE_D_MARK=phased-1789631191
  PHASE_D_TOKEN=… PHASE_D_SOCK=… ./node_modules/.bin/playwright test
  -c .aiws-tmp-pw/playwright.config.ts phase-d-assert.spec.ts` (in `web/`)
  → **0, `1 passed (5.4s)`**.
- `cargo test -p relay-server after_seq` (D1 unit test sanity) → 0, 1 passed.
- No commits. `tasks.md`/`proposal.md` untouched. `scripts/e2e.sh` untouched
  (pre-existing failure, out of scope).

## Playwright assertions (passing run, real isolated stack)

Flow: login `连接到服务器` → custom URL → `已连接` → session card matching
MARK → `.detail-head` visible → DB poll finds `[mock-codex] approved` →
20-line burst via hostd socket → DB poll (≥2 burst rows = in flight) →
`__phaseDKillWs()` kills 1 app socket → `离线` banner visible → all 20 lines
land in DB while page WS dead → `重连` click → `已连接` returns → burst tail
in feed → seq/backfill/text/phantom assertions.

Console markers from the passing run:

- `pre-burst page WS seqs seen: 1`
- `baseMax seq=19 burstTag=D3BURST_1789631236767`
- `killed 1 app socket(s) mid-burst`
- `post-burst max seq=40 (base was 19)`
- `burst tail in feed after reconnect: true`
- `db seqs n=40 range=[1,40] windowMissing=[]`
- `backfill n=21 bSeqs=[20..40] absent=[]`
- `page WS seqs observed=22 phantom=0`

Results:

- DB ground truth: full per-run seq series `1..40`, 0 dupes, 0 gaps in the
  kill window `(19,40]` — **pass**.
- `after_seq` backfill (`/sessions/:id/messages?limit=500&after_seq=19`)
  returns exactly seqs `20..40`, no dupes, nothing `<= after_seq`, every
  window seq present — **pass**.
- Reassembled burst text: all 20 `D3BURST_…_LINE_<i>` lines in DB
  `run.output` — **pass**.
- Page-socket seqs are a subset of DB seqs (0 phantom) — **pass**.
- Backfill renders the burst tail in the feed after reconnect — **pass**.

## What failed before the pass (diagnosed, no source edits)

1. **Stale server binary ignored `after_seq`.** The long-lived 8790 server
   (pid 300866, `/proc/exe` → deleted binary) predated the D1 working-tree
   change, so `?limit=500&after_seq=70` returned 100+ rows starting at seq 1
   (filter silently absent). The spec's `backfill: all seqs > after_seq`
   assertion failed correctly — 1 failed, exit 1. Fix was operational, not a
   source edit: restarted `relay-server` from the current tree (pid 893066);
   the same query then returned `n=87 min=71 max=157`. The D1 unit test
   (`list_message_events_after_seq_filters_by_seq`) passed throughout,
   confirming the code was right and only the running binary was stale.
2. **Wedge: hostd spool backlog blocked server POST input.** An earlier hostd
   (pid 823759) accumulated ~2 MB of un-ACKed spool rows in its WS send
   queue; server `POST /runs/:id/input` blocked on host delivery (curl
   timeout, HTTP:000) while unix-socket direct input stayed fast (HTTP:204).
   Fix was operational: killed the wedged hostd, trimmed already-persisted
   spool rows (`delete from spool_events where seq > 86`), restarted hostd
   (pid 863285, queues ESTAB 0/0). No source edits. Spool-backup at
   `/tmp/spool-backup.db` (1713 rows) if needed.
3. One earlier phase-D attempt on a stale run (`run-97f00655…`) hit failure
   (1) above; the fresh run (`run-40262e21…`) on the restarted server passed
   first try. Preferring this failure evidence over speculative source edits
   per instructions — no `protocol/`/`hostd/`/`server/`/`web/` files touched
   by Phase D.

## Cleanup notes

- Temp Playwright copy `web/.aiws-tmp-pw/phase-d-assert.spec.ts` kept in sync
  with the evidence spec (working copy for `-c .aiws-tmp-pw/playwright.config.ts`).
- Isolated stack (server 8790 pid 893066 + hostd pid 863285 + web 5174) left
  running for follow-up phases. No secrets stored in files. No commit.
