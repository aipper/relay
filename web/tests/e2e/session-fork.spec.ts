import { test, expect } from "./fixtures/app";

test.describe("会话 Fork", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("domcontentloaded");
    await page.waitForTimeout(1000);
    await page.getByRole("textbox", { name: "用户名" }).fill("admin");
    await page.getByRole("textbox", { name: "密码" }).fill("password");
    await page.getByRole("button", { name: "登录" }).click();
    await expect(page.getByRole("button", { name: "会话" })).toBeVisible({ timeout: 30000 });
  });

  test("Session ID 输入框在选择 opencode 后显示", async ({ page }) => {
    await page.getByRole("button", { name: "启动" }).first().click();
    await page.waitForSelector("text=Session ID（可选，续接已有会话）", { timeout: 30000 });
    await expect(page.getByRole("textbox", { name: "Session ID（可选，续接已有会话）" })).toBeVisible();
  });

  test("填写 Session ID 后 rpc.run.start 会带 opencode_session_id", async ({ page }) => {
    await page.getByRole("button", { name: "启动" }).first().click();
    await page.waitForSelector("text=Session ID（可选，续接已有会话）", { timeout: 30000 });
    await page.getByRole("textbox", { name: "Session ID（可选，续接已有会话）" }).fill("ses-test-fork");
    const sessionIdInput = await page.locator('input[placeholder="ses_xxx 或留空新建"]').inputValue();
    expect(sessionIdInput).toBe("ses-test-fork");
  });
});