import { test, expect, chromium } from "@playwright/test";
import { execFileSync } from "node:child_process";

const CHROME_PATH = "/home/ab/.cache/ms-playwright/chromium-1208/chrome-linux64/chrome";
const WEB_URL = "http://127.0.0.1:5174/";
const API_BASE = "http://127.0.0.1:5174";
const RUN_ID = process.env.PHASE_B_RUN_ID!;
const DB = process.env.PHASE_B_DB!;
const MARK = process.env.PHASE_B_MARK!;

function dbRows(): { seq: number | null; text: string }[] {
  const out = execFileSync("sqlite3", ["-separator", "\t", DB, `select ifnull(seq,-1), data_json from events where run_id='${RUN_ID}' and type='run.output' order by seq;`], { encoding: "utf8" });
  return out.trim().split("\n").filter(Boolean).map((line) => {
    const tab = line.indexOf("\t");
    const seq = Number(line.slice(0, tab));
    let text = "";
    try {
      const j = JSON.parse(line.slice(tab + 1));
      text = (j.text ?? j.text_redacted ?? "") as string;
    } catch { /* ignore */ }
    return { seq, text };
  });
}

function stripAnsi(s: string): string {
  return s.replace(/\x1b\[[0-9;?]*[a-zA-Z]/g, "\n").replace(/\x1b[()][0-9A-B]/g, "").replace(/\x1b[=>]/g, "").replace(/\x1b\].*?(\x07|\x1b\\)/g, "").replace(/\x0f/g, "").replace(/\x1bM/g, "\n");
}

function echoNums(texts: string[]): number[] {
  return stripAnsi(texts.join("\n")).split("\n").map((l) => l.trim()).filter((l) => /(\[mock-codex\] echo: )?ECHOLINE_\d+$/.test(l)).map((l) => Number((l.match(/ECHOLINE_(\d+)$/) ?? [])[1] ?? NaN)).filter((n) => Number.isFinite(n));
}

test("phase-b: first byte fast + burst coalesced, text exact", async () => {
  const browser = await chromium.launch({ executablePath: CHROME_PATH, args: ["--no-sandbox"] });
  const ctx = await browser.newContext();
  const page = await ctx.newPage();

  await page.addInitScript((runId: string) => {
    const st: any = ((window as any).__phaseB = { count: 0, firstAt: 0, texts: [] as string[] });
    const OrigWS = window.WebSocket;
    function wrap(sock: any) {
      sock.addEventListener("message", (ev: MessageEvent) => {
        try {
          const env = JSON.parse(String(ev.data));
          if (env.type === "run.output" && env.run_id === runId) {
            st.count++;
            if (!st.firstAt) st.firstAt = Date.now();
            const t = env.data?.text ?? "";
            if (t) st.texts.push(t);
          }
        } catch { /* ignore */ }
      });
    }
    (window as any).WebSocket = new Proxy(OrigWS, {
      construct(t: any, a: any[]) {
        const sock = new t(...a);
        try { wrap(sock); } catch { /* ignore */ }
        return sock;
      },
    });
  }, RUN_ID);

  try {
    await page.goto(WEB_URL);
    await expect(page.getByText("连接到服务器")).toBeVisible({ timeout: 15000 });
    await page.getByRole("button", { name: "自定义" }).click();
    await page.getByPlaceholder("http(s)://host:8787").fill(API_BASE);
    await page.locator("#username-input").fill("admin");
    await page.locator("#password-input").fill("123456");
    await page.getByRole("button", { name: "登录" }).click();
    await expect(page.getByText("已连接")).toBeVisible({ timeout: 20000 });

    // Our run uses a unique cwd marker, so its card is the only one whose
    // summary contains MARK (marker: exactly one matching card).
    const runCard = page.locator(".session-item", { hasText: MARK }).first();
    await expect(runCard).toBeVisible({ timeout: 20000 });
    await runCard.click();
    await expect(page.locator(".sessions-main", { hasText: RUN_ID })).toBeVisible({ timeout: 10000 });
    const ta = page.locator(".chat-textarea");
    await expect(ta).toBeEnabled({ timeout: 15000 });

    // Approve the mock-codex prompt: bare "y" routes to sendDecision("approve")
    // (marker: mock echoes approval into run.output).
    await ta.fill("y");
    await ta.press("Enter");
    await expect(ta).toHaveValue("", { timeout: 5000 });
    const d0 = Date.now() + 15000;
    let ready = false;
    while (Date.now() < d0) {
      const joined = stripAnsi(dbRows().map((r) => r.text).join("\n"));
      if (joined.includes("[mock-codex] approved") || joined.includes("[mock-codex] echo:")) { ready = true; break; }
      await page.waitForTimeout(300);
    }
    expect(ready, "mock run must approve via chat input").toBe(true);

    // 1) echo hello: leading edge — first pushed byte <50ms after DB persist.
    await page.evaluate(() => { const s: any = (window as any).__phaseB; s.count = 0; s.firstAt = 0; s.texts = []; });
    const baseRows = dbRows().length;
    await ta.fill("echo __RELAY_PB_ECHO__");
    await ta.press("Enter");
    await expect(ta).toHaveValue("", { timeout: 5000 });

    let dbAt = 0;
    const d1 = Date.now() + 15000;
    while (Date.now() < d1) {
      const rows = dbRows();
      if (rows.length > baseRows && stripAnsi(rows.map((r) => r.text).join("\n")).includes("__RELAY_PB_ECHO__")) { dbAt = Date.now(); break; }
      await page.waitForTimeout(100);
    }
    expect(dbAt, "echo output must persist in DB").toBeGreaterThan(0);

    let pushAt = 0;
    const d1b = Date.now() + 10000;
    while (Date.now() < d1b) {
      pushAt = await page.evaluate(() => (window as any).__phaseB.firstAt as number);
      if (pushAt) break;
      await page.waitForTimeout(25);
    }
    expect(pushAt, "echo output must be pushed to page socket").toBeGreaterThan(0);
    const lag = pushAt - dbAt;
    const echoPushed = await page.evaluate(() => (window as any).__phaseB.count as number);
    console.log(`[phase-b] echo lag=${lag}ms pushed=${echoPushed}`);
    expect(lag, "leading edge: first byte <50ms after persist").toBeLessThan(50);

    // 2) burst: 10 numbered lines as ONE multiline paste. The mock echoes each
    // line; tmux types them rapidly and PTY output + server coalescer merge
    // them. Raw DB text and pushed text must both reassemble to exactly
    // ECHOLINE_1..10 in order, with pushed chunks far fewer.
    // NOTE: the mock reads stdin line-by-line and the PWA chat input sends
    // bare "y" as run.permission.approve (no run.input row), so the mock may
    // still be waiting if a previous approval raced. Re-approve if needed.
    await page.evaluate(() => { const s: any = (window as any).__phaseB; s.count = 0; s.firstAt = 0; s.texts = []; });
    const burstBase = dbRows().length;
    const burst = Array.from({ length: 10 }, (_, i) => `ECHOLINE_${i + 1}`).join("\n");
    await ta.fill(burst);
    await ta.press("Enter");
    await expect(ta).toHaveValue("", { timeout: 5000 });
    // marker-delimited: our burst run.input row must land (proves the send
    // reached OUR run, not a stale card selection).
    {
      const dB = Date.now() + 10000;
      let landed = false;
      while (Date.now() < dB) {
        const n = execFileSync("sqlite3", [DB, `select count(*) from events where run_id='${RUN_ID}' and type='run.input' and instr(data_json,'ECHOLINE_1')>0;`], { encoding: "utf8" });
        if (Number(n.trim()) >= 1) { landed = true; break; }
        await page.waitForTimeout(300);
      }
      expect(landed, "burst run.input must land in OUR run").toBe(true);
    }

    const d2 = Date.now() + 30000;
    let burstRows: { seq: number | null; text: string }[] = [];
    for (;;) {
      burstRows = dbRows().slice(burstBase);
      if (stripAnsi(burstRows.map((r) => r.text).join("\n")).includes("ECHOLINE_10")) break;
      if (Date.now() > d2) break;
      await page.waitForTimeout(300);
    }
    // quiescence: let trailing PTY flushes land (count-stable polling).
    let q2 = 0, l2 = -1;
    const dq = Date.now() + 15000;
    while (Date.now() < dq) {
      const n = dbRows().length;
      if (n === l2) { q2++; if (q2 >= 5) break; } else { q2 = 0; l2 = n; }
      await page.waitForTimeout(500);
    }
    burstRows = dbRows().slice(burstBase);
    const rawNums = echoNums(burstRows.map((r) => r.text));
    const rawSorted = [...new Set(rawNums)].sort((a, b) => a - b);
    expect(rawSorted, "raw DB text contains each of 1..10 at least once").toEqual(Array.from({ length: 10 }, (_, i) => i + 1));
    const dbBurstCount = burstRows.length;
    console.log(`[phase-b] burst rawRows=${dbBurstCount}`);

    let quiet = 0, last = -1;
    const d3 = Date.now() + 10000;
    while (Date.now() < d3) {
      const c: number = await page.evaluate(() => (window as any).__phaseB.count as number);
      if (c === last) { quiet++; if (quiet >= 4) break; } else { quiet = 0; last = c; }
      await page.waitForTimeout(250);
    }
    const pushed: number = await page.evaluate(() => (window as any).__phaseB.count as number);
    const ptexts: string[] = await page.evaluate(() => (window as any).__phaseB.texts as string[]);
    const pNums = echoNums(ptexts);
    console.log(`[phase-b] burst pushedChunks=${pushed}`);
    expect(pushed, `burst pushed chunks (${pushed}) far fewer than 10 lines`).toBeLessThan(10);
    const pSorted = [...new Set(pNums)].sort((a, b) => a - b);
    expect(pSorted, "pushed text contains each of 1..10 at least once").toEqual(Array.from({ length: 10 }, (_, i) => i + 1));
  } finally {
    await browser.close();
  }
});
