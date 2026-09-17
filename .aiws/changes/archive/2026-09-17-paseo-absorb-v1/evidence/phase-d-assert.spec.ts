import { test, expect, chromium } from "@playwright/test";
import { execFileSync } from "node:child_process";

// Phase D: WS-kill-mid-burst reconnect has no seq gap/duplicates.
//
// WS-kill method (deterministic, documented): an in-page WebSocket proxy
// installed via addInitScript tracks every app socket; the test closes the
// socket whose URL contains "/ws/app" via page.evaluate (server keeps
// running, hostd keeps running — only the PWA page socket dies mid-burst).
// Reconnect is the real UI path: the offline banner's 重连 button calls
// relay.resumeFromStoredToken → #openAppWebSocket → onopen resubscribes +
// backfillAfterReconnect(runId) with after_seq = pre-kill maxSeqByRun.
//
// Burst injection goes through hostd's unix socket (POST /runs/:id/input),
// NOT the server HTTP input endpoint: server POST blocks on host delivery
// while this spec needs fire-and-forget line injection during the kill
// window. The mock echoes each line; hostd assigns monotonic host seq;
// server persists per-chunk (INSERT OR IGNORE on (run_id, seq)) and the
// after_seq messages API serves the backfill tail.

const CHROME_PATH = "/home/ab/.cache/ms-playwright/chromium-1208/chrome-linux64/chrome";
const WEB_URL = "http://127.0.0.1:5174/";
const API_BASE = "http://127.0.0.1:5174";
const SERVER = "http://127.0.0.1:8790";
const RUN_ID = process.env.PHASE_D_RUN_ID!;
const DB = process.env.PHASE_D_DB!;
const MARK = process.env.PHASE_D_MARK!;
const TOKEN = process.env.PHASE_D_TOKEN!;
const SOCK = process.env.PHASE_D_SOCK!;

function q(sql: string): string {
  return execFileSync("sqlite3", [DB, sql], { encoding: "utf8" }).trim();
}

// All persisted host seqs for the run (every event type carrying a seq).
function dbSeqs(): number[] {
  const out = q(`select seq from events where run_id='${RUN_ID}' and seq is not null order by seq;`);
  return out.split("\n").filter(Boolean).map(Number);
}

// Fire-and-forget line injection via hostd unix socket (never blocks).
function sockInput(inputId: string, text: string): void {
  execFileSync("curl", [
    "--noproxy", "*", "-s", "-m", "5", "-o", "/dev/null",
    "--unix-socket", SOCK,
    "-X", "POST", `http://localhost/runs/${RUN_ID}/input`,
    "-H", "content-type: application/json",
    "-d", JSON.stringify({ input_id: inputId, actor: "cli", text }),
  ]);
}

test("phase-d: kill page WS mid-burst, reconnect, seq no-gap no-dupe after backfill", async () => {
  const browser = await chromium.launch({ executablePath: CHROME_PATH, args: ["--no-sandbox"] });
  const page = await browser.newPage();
  try {
    // Install WS spy BEFORE any page script runs: record every envelope seq
    // for our run seen on the page socket, and expose a deterministic kill
    // switch for the app socket (page-side close = mid-burst kill).
    await page.addInitScript((runId: string) => {
      const st: any = ((window as any).__phaseD = {
        wsSeqs: [] as number[],
        sockets: [] as WebSocket[],
      });
      const OrigWS = window.WebSocket;
      function wrap(sock: WebSocket) {
        (st.sockets as WebSocket[]).push(sock);
        sock.addEventListener("message", (ev: MessageEvent) => {
          try {
            const env = JSON.parse(String(ev.data));
            if (env.run_id === runId && typeof env.seq === "number") {
              (st.wsSeqs as number[]).push(env.seq as number);
            }
          } catch { /* ignore non-JSON */ }
        });
      }
      (window as any).WebSocket = new Proxy(OrigWS, {
        construct(t: any, a: any[]) {
          const sock = new t(...a);
          try { wrap(sock); } catch { /* ignore */ }
          return sock;
        },
      });
      // Deterministic kill: close the app WS (url contains /ws/app).
      (window as any).__phaseDKillWs = () => {
        let n = 0;
        for (const s of st.sockets as WebSocket[]) {
          try {
            const url = (s as any).url as string;
            if (url.includes("/ws/app") && s.readyState === WebSocket.OPEN) {
              s.close(1000, "phase-d-kill");
              n++;
            }
          } catch { /* ignore */ }
        }
        return n;
      };
    }, RUN_ID);

    await page.goto(WEB_URL);
    await expect(page.getByText("连接到服务器")).toBeVisible({ timeout: 15000 });
    await page.getByRole("button", { name: "自定义" }).click();
    await page.getByPlaceholder("http(s)://host:8787").fill(API_BASE);
    await page.locator("#username-input").fill("admin");
    await page.locator("#password-input").fill("123456");
    await page.getByRole("button", { name: "登录" }).click();
    await expect(page.getByText("已连接")).toBeVisible({ timeout: 20000 });

    // marker: our run card (unique cwd MARK).
    const runCard = page.locator(".session-item", { hasText: MARK }).first();
    await expect(runCard).toBeVisible({ timeout: 20000 });
    await runCard.click();
    // marker: detail pane opens.
    await expect(page.locator(".sessions-main .detail-head")).toBeVisible({ timeout: 10000 });

    // marker-delimited: run must be approved and accepting input (mock
    // echoes approval). Approve via hostd socket if still awaiting.
    {
      const d = Date.now() + 20000;
      let approved = false;
      while (Date.now() < d) {
        const joined = q(`select group_concat(data_json, char(10)) from events where run_id='${RUN_ID}' and type='run.output';`);
        if (joined.includes("[mock-codex] approved")) { approved = true; break; }
        await page.waitForTimeout(500);
      }
      if (!approved) {
        sockInput(`phase-d-approve-${Date.now()}`, "y\n");
        const d2 = Date.now() + 20000;
        let ok = false;
        while (Date.now() < d2) {
          const joined = q(`select group_concat(data_json, char(10)) from events where run_id='${RUN_ID}' and type='run.output';`);
          if (joined.includes("[mock-codex] approved")) { ok = true; break; }
          await page.waitForTimeout(500);
        }
        expect(ok, "mock run must approve").toBe(true);
      }
    }

    const preKillWsSeqs: number[] = await page.evaluate(() => [...((window as any).__phaseD.wsSeqs as number[])]);
    console.log(`[phase-d] pre-burst page WS seqs seen: ${preKillWsSeqs.length}`);

    // Start a burst: N numbered lines via hostd socket (deterministic,
    // avoids chat-input coalescing races). The mock echoes each line;
    // hostd emits run.output with monotonic host seq.
    const N = 20;
    const BURST_TAG = `D3BURST_${Date.now()}`;
    const baseMax = Math.max(...dbSeqs(), 0);
    console.log(`[phase-d] baseMax seq=${baseMax} burstTag=${BURST_TAG}`);
    for (let i = 1; i <= N; i++) {
      sockInput(`${BURST_TAG}-${i}`, `${BURST_TAG}_LINE_${i}\n`);
    }

    // marker-delimited: wait until at least 2 burst rows land in DB
    // (burst is in flight), then KILL the page WS mid-burst.
    {
      const d = Date.now() + 30000;
      let landed = 0;
      while (Date.now() < d) {
        const c = q(`select count(*) from events where run_id='${RUN_ID}' and data_json like '%${BURST_TAG}%';`);
        landed = Number(c.trim());
        if (landed >= 2) break;
        await page.waitForTimeout(300);
      }
      expect(landed, "burst must be in flight (>=2 burst rows in DB)").toBeGreaterThanOrEqual(2);
    }
    const killed = await page.evaluate(() => (window as any).__phaseDKillWs() as number);
    console.log(`[phase-d] killed ${killed} app socket(s) mid-burst`);
    expect(killed, "must kill >=1 app WS").toBeGreaterThanOrEqual(1);

    // marker: offline banner appears while disconnected.
    await expect(page.getByText("离线")).toBeVisible({ timeout: 15000 });

    // While the page WS is dead, the burst continues server-side. Wait for
    // ALL N burst lines to land in DB (marker-delimited, not sleep).
    {
      const d = Date.now() + 60000;
      let done = false;
      while (Date.now() < d) {
        const joined = q(`select group_concat(data_json, char(10)) from events where run_id='${RUN_ID}' and type='run.output';`);
        let all = true;
        for (let i = 1; i <= N; i++) {
          if (!joined.includes(`${BURST_TAG}_LINE_${i}`)) { all = false; break; }
        }
        if (all) { done = true; break; }
        await page.waitForTimeout(500);
      }
      expect(done, `all ${N} burst lines must land in DB while page WS dead`).toBe(true);
    }
    const postBurstMax = Math.max(...dbSeqs(), 0);
    console.log(`[phase-d] post-burst max seq=${postBurstMax} (base was ${baseMax})`);
    expect(postBurstMax, "burst must advance host seq").toBeGreaterThan(baseMax);

    // Reconnect via the real UI path: offline banner 重连 button
    // (resumeFromStoredToken → #openAppWebSocket → onopen resubscribes +
    // backfillAfterReconnect with after_seq = pre-kill maxSeqByRun).
    const reconnectBtn = page.getByRole("button", { name: "重连" });
    await expect(reconnectBtn).toBeVisible({ timeout: 15000 });
    await reconnectBtn.click();
    // marker: connected badge returns.
    await expect(page.getByText("已连接")).toBeVisible({ timeout: 20000 });

    // marker-delimited: backfill completion — the burst tail line must
    // appear in the rendered feed (ChatFeed ← messagesByRun ← backfill).
    {
      const d = Date.now() + 30000;
      let found = false;
      while (Date.now() < d) {
        const feedText = await page.locator(".sessions-main").innerText().catch(() => "");
        if (feedText.includes(`${BURST_TAG}_LINE_${N}`)) { found = true; break; }
        await page.waitForTimeout(500);
      }
      console.log(`[phase-d] burst tail in feed after reconnect: ${found}`);
      expect(found, "backfill must render burst tail in feed").toBe(true);
    }

    // Core assertion 1: server-side seq series has no gaps and no
    // duplicates across the kill window (DB ground truth). Continuity is
    // checked on the full per-run seq series (run.input consumes seqs too).
    const seqs = dbSeqs();
    const uniq = new Set(seqs);
    expect(uniq.size, "no duplicate seqs in DB").toBe(seqs.length);
    const lo = Math.min(...seqs);
    const hi = Math.max(...seqs);
    const missing: number[] = [];
    for (let s = lo; s <= hi; s++) if (!uniq.has(s)) missing.push(s);
    const windowMissing = missing.filter((s) => s > baseMax && s <= postBurstMax);
    console.log(`[phase-d] db seqs n=${seqs.length} range=[${lo},${hi}] windowMissing=${JSON.stringify(windowMissing)}`);
    expect(windowMissing, `no seq gap in burst window (${baseMax},${postBurstMax}]`).toEqual([]);

    // Core assertion 2: after_seq backfill returns exactly the window tail
    // with no dupes (same query shape the client uses).
    const backfillRaw = execFileSync("curl", [
      "--noproxy", "*", "-s", "-m", "10",
      `${SERVER}/sessions/${RUN_ID}/messages?limit=500&after_seq=${baseMax}`,
      "-H", `Authorization: Bearer ${TOKEN}`,
    ], { encoding: "utf8" });
    const backfill = JSON.parse(backfillRaw) as Array<{ seq?: number | null }>;
    const bSeqs = backfill.map((m) => m.seq).filter((s): s is number => typeof s === "number").sort((a, b) => a - b);
    const bUniq = new Set(bSeqs);
    expect(bUniq.size, "backfill: no duplicate seqs").toBe(bSeqs.length);
    const bMissing = bSeqs.filter((s) => s <= baseMax);
    expect(bMissing, "backfill: all seqs > after_seq").toEqual([]);
    // Every window seq present in DB must be returned by backfill (except
    // null-seq kinds which carry no seq by design).
    const dbWindow = seqs.filter((s) => s > baseMax && s <= postBurstMax);
    const absent = dbWindow.filter((s) => !bUniq.has(s));
    console.log(`[phase-d] backfill n=${backfill.length} bSeqs=[${bSeqs[0]}..${bSeqs[bSeqs.length - 1]}] absent=${JSON.stringify(absent)}`);
    expect(absent, "backfill must contain every window seq").toEqual([]);

    // Core assertion 3: reassembled burst text complete — all N lines
    // present in DB output text.
    const joined = q(`select group_concat(data_json, char(10)) from events where run_id='${RUN_ID}' and type='run.output';`);
    const absentLines: number[] = [];
    for (let i = 1; i <= N; i++) {
      if (!joined.includes(`${BURST_TAG}_LINE_${i}`)) absentLines.push(i);
    }
    expect(absentLines, "all burst lines reassembled in DB").toEqual([]);

    // Page WS seqs observed before kill must be a subset of DB seqs
    // (no phantom/duplicate seqs delivered to the page socket).
    const pageSeqs: number[] = await page.evaluate(() => [...((window as any).__phaseD.wsSeqs as number[])]);
    const phantom = pageSeqs.filter((s) => !uniq.has(s));
    console.log(`[phase-d] page WS seqs observed=${pageSeqs.length} phantom=${phantom.length}`);
    expect(phantom, "no phantom seqs on page socket").toEqual([]);
  } finally {
    await browser.close();
  }
});
