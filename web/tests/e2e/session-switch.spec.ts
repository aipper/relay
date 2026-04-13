import { test, expect } from "./fixtures/app";

test.describe("会话切换", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("domcontentloaded");
    await page.waitForTimeout(1000);
    await page.getByRole("textbox", { name: "用户名" }).fill("admin");
    await page.getByRole("textbox", { name: "密码" }).fill("password");
    await page.getByRole("button", { name: "登录" }).click();
    await expect(page.getByRole("button", { name: "会话" })).toBeVisible({ timeout: 30000 });
  });

  test("显示切换按钮", async ({ page }) => {
    await page.getByRole("button", { name: "会话" }).click();
    await expect(page.getByRole("button", { name: "切换" })).toBeVisible({ timeout: 5000 });
  });

  test("点击切换按钮显示会话列表", async ({ page }) => {
    await page.getByRole("button", { name: "会话" }).click();
    await page.getByRole("button", { name: "切换" }).click();
    await expect(page.locator(".session-selector")).toBeVisible({ timeout: 5000 });
  });

  test("选择会话后切换", async ({ page }) => {
    await page.getByRole("button", { name: "会话" }).click();
    await page.getByRole("button", { name: "切换" }).click();
    await expect(page.locator(".session-selector")).toBeVisible({ timeout: 5000 });
    await page.locator(".session-item").first().click();
    await page.waitForTimeout(500);
  });
});