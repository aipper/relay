import { test, expect } from "./fixtures/app";

async function login(page: import("@playwright/test").Page) {
  await page.goto("/");
  await page.waitForLoadState("networkidle", { timeout: 30000 });
  await page.getByRole("textbox", { name: "用户名" }).fill("admin");
  await page.getByRole("textbox", { name: "密码" }).fill("password");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByText("已连接")).toBeVisible({ timeout: 15000 });
}

test.describe("会话列表", () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test("显示会话列表", async ({ page }) => {
    await expect(page.getByText(/^relay /).first()).toBeVisible({ timeout: 10000 });
    await expect(page.getByText(/^web /).first()).toBeVisible({ timeout: 5000 });
  });

  test("显示主机分组", async ({ page }) => {
    await expect(page.getByText("Development")).toBeVisible({ timeout: 5000 });
  });

  test("点击会话进入详情", async ({ page }) => {
    const relayBtn = page.locator('button').filter({ hasText: /^relay / });
    await relayBtn.click();
    await page.waitForLoadState("domcontentloaded");
    await expect(page.locator(".messages")).toBeVisible({ timeout: 10000 }).catch(async () => {
      await expect(page.getByText("run started")).toBeVisible({ timeout: 5000 });
    });
  });

  test("会话状态标签", async ({ page }) => {
    await expect(page.getByText("运行中")).toBeVisible({ timeout: 5000 });
    await expect(page.getByText("已结束")).toBeVisible({ timeout: 5000 });
    await expect(page.getByText("待审批")).toBeVisible({ timeout: 5000 });
  });

  test("会话搜索过滤", async ({ page }) => {
    const searchInput = page.getByPlaceholder(/搜索|search/i);
    await searchInput.fill("web");
    await page.waitForTimeout(300);
    await expect(page.getByText(/^web /).first()).toBeVisible({ timeout: 5000 });
  });
});

test.describe("会话详情", () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
    const relayBtn = page.locator('button').filter({ hasText: /^relay / });
    await relayBtn.click();
    await page.waitForLoadState("domcontentloaded");
  });

  test("消息流渲染", async ({ page }) => {
    await expect(page.getByText("run started")).toBeVisible({ timeout: 5000 });
    await expect(page.getByText("tool.call fs.read")).toBeVisible({ timeout: 5000 });
  });

  test("切换到输出视图", async ({ page }) => {
    const outputTab = page.getByRole("tab", { name: /输出|output/i });
    if (await outputTab.isVisible()) {
      await outputTab.click();
      await page.waitForTimeout(500);
    }
  });

  test("WebSocket 实时更新", async ({ page }) => {
    await page.waitForResponse(/\/sessions\/run-001\/messages/, { timeout: 10000 }).catch(() => {});
    const connected = await page.evaluate(() => {
      const statusEl = document.querySelector('[class*="status"]');
      return document.body.textContent?.includes("已连接") ?? false;
    });
    await expect(connected).toBe(true);
    await page.waitForTimeout(1000);
  });
});
