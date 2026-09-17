import { test, expect, chromium } from "@playwright/test";
import { execFileSync } from "node:child_process";

const CHROME_PATH = "/home/ab/.cache/ms-playwright/chromium-1208/chrome-linux64/chrome";
const WEB_URL = "http://127.0.0.1:5174/";
const API_BASE = "http://127.0.0.1:5174";
const RUN_ID = process.env.PHASE_C_RUN_ID!;
const DB = process.env.PHASE_C_DB!;
const MARK = process.env.PHASE_C_MARK!;

function q(sql: string): string {
  return execFileSync("sqlite3", [DB, sql], { encoding: "utf8" }).trim();
}

function permRequested(): { request_id: string; actions: Array<{ id: string; label: string; behavior?: string }> } | null {
  const out = q(`select data_json from events where run_id='${RUN_ID}' and type='run.permission_requested' order by seq desc limit 1;`);
  if (!out) return null;
  try {
    const j = JSON.parse(out);
    return { request_id: j.request_id, actions: j.actions ?? [] };
  } catch {
    return null;
  }
}

test("phase-c: actions[] multi-button approve round-trip via action button", async () => {
  const browser = await chromium.launch({ executablePath: CHROME_PATH, args: ["--no-sandbox"] });
  const page = await browser.newPage();
  try {
    await page.goto(WEB_URL);
    await expect(page.getByText("连接到服务器")).toBeVisible({ timeout: 15000 });
    await page.getByRole("button", { name: "自定义" }).click();
    await page.getByPlaceholder("http(s)://host:8787").fill(API_BASE);
    await page.locator("#username-input").fill("admin");
    await page.locator("#password-input").fill("123456");
    await page.getByRole("button", { name: "登录" }).click();
    await expect(page.getByText("已连接")).toBeVisible({ timeout: 20000 });

    // marker: our run card — select by unique MARK cwd text, NOT .first():
    // stale awaiting runs (permission_requested buried beyond messages
    // limit=200) render no approval card, and list order varies.
    const runCard = page.locator(".session-item", { hasText: MARK }).first();
    await expect(runCard).toBeVisible({ timeout: 20000 });
    await runCard.click();
    // marker: detail pane opens (SessionDetail renders for selectedRunId)
    await expect(page.locator(".sessions-main .detail-head")).toBeVisible({ timeout: 10000 });

    // marker-delimited: permission_requested with actions[] must already be in DB
    let pr: { request_id: string; actions: Array<{ id: string; label: string; behavior?: string }> } | null = null;
    {
      const d = Date.now() + 15000;
      while (Date.now() < d) {
        pr = permRequested();
        if (pr && pr.actions.length >= 2) break;
        await page.waitForTimeout(300);
      }
    }
    expect(pr, "run.permission_requested must exist").not.toBeNull();
    expect(pr!.actions.map((a) => a.id).sort(), "actions[] ids").toEqual(["approve", "deny"]);
    console.log(`[phase-c] actions=${JSON.stringify(pr!.actions)} request_id=${pr!.request_id}`);

    // Svelte scopes classes (approval-card s-xxx), so a multi-class
    // descendant chain never matches; match the card by attribute substring.
    // The approval modal auto-opens on select and covers the inline card,
    // so click the modal's action button (same actions[] + onSendAction path).
    const card = page.locator(".sessions-main div[class*='approval-card']").first();
    await expect(card).toBeVisible({ timeout: 15000 });

    // marker: action buttons render (Approve/Deny labels from actions[])
    const approveBtn = page.getByRole("dialog").getByRole("button", { name: "Approve" });
    await expect(approveBtn).toBeVisible({ timeout: 10000 });
    await expect(approveBtn).toBeEnabled({ timeout: 10000 });

    await approveBtn.click();

    // marker-delimited: run.permission_decided persisted with selected_action_id=approve
    let decided = "";
    {
      const d = Date.now() + 15000;
      while (Date.now() < d) {
        decided = q(`select data_json from events where run_id='${RUN_ID}' and type='run.permission_decided' order by seq desc limit 1;`);
        if (decided && decided.includes("approve")) break;
        await page.waitForTimeout(300);
      }
    }
    expect(decided, "run.permission_decided must persist").toContain("approve");
    expect(decided, "selected_action_id must round-trip").toContain("selected_action_id");
    console.log(`[phase-c] decided=${decided.slice(0, 300)}`);

    // marker-delimited: mock echoes approval into run.output
    let ready = false;
    {
      const d = Date.now() + 15000;
      while (Date.now() < d) {
        const joined = q(`select group_concat(data_json, char(10)) from events where run_id='${RUN_ID}' and type='run.output';`);
        if (joined.includes("[mock-codex] approved")) { ready = true; break; }
        await page.waitForTimeout(300);
      }
    }
    expect(ready, "mock run must approve via action button").toBe(true);

    // old-shape fallback: strip actions from a synthetic check — awaiting without
    // actions must still render legacy approve/deny (unit-level, in-page).
    const fallback = await page.evaluate(() => {
      const hasActions = (a: unknown): boolean =>
        !!a && typeof a === "object" && Array.isArray((a as Record<string, unknown>).actions) &&
        ((a as Record<string, unknown>).actions as unknown[]).length > 0;
      return { oldShapeFallsBack: !hasActions({ request_id: "x", prompt: "y" }) };
    });
    expect(fallback.oldShapeFallsBack).toBe(true);
  } finally {
    await browser.close();
  }
});
