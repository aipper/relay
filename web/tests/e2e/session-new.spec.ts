import { test, expect } from "./fixtures/app";

test.describe("会话 新建", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("domcontentloaded");
    await page.waitForTimeout(1000);
    await page.getByRole("textbox", { name: "用户名" }).fill("admin");
    await page.getByRole("textbox", { name: "密码" }).fill("password");
    await page.getByRole("button", { name: "登录" }).click();
    await expect(page.getByRole("button", { name: "会话" })).toBeVisible({ timeout: 30000 });
  });

  test("新建会话 - 填写 Session ID 启动", async ({ page }) => {
    await page.getByRole("button", { name: "启动" }).first().click();
    await page.waitForSelector("input[placeholder='ses_xxx 或留空新建']", { timeout: 30000 });
    const sessionId = `ses_${Date.now()}`;
    await page.locator("input[placeholder='ses_xxx 或留空新建']").fill(sessionId);
  });

  test("新建会话 - 留空 Session ID 则新建", async ({ page }) => {
    await page.getByRole("button", { name: "启动" }).first().click();
    await page.waitForSelector("input[placeholder='ses_xxx 或留空新建']", { timeout: 30000 });
    await page.getByRole("textbox", { name: "CWD（可选，主机路径）" }).fill("/tmp");
    await expect(page.locator("input[placeholder='ses_xxx 或留空新建']")).toHaveValue("");
  });
});