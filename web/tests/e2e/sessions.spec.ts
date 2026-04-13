import { test, expect } from "./fixtures/app";

test.describe("会话列表", () => {
  test("登录后显示导航栏", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle", { timeout: 30000 });
    await page.getByRole("textbox", { name: "用户名" }).fill("admin");
    await page.getByRole("textbox", { name: "密码" }).fill("password");
    await page.getByRole("button", { name: "登录" }).click();

    await expect(page.getByRole("button", { name: "会话" })).toBeVisible({ timeout: 15000 });
    await expect(page.getByRole("button", { name: "主机" })).toBeVisible();
    await expect(page.getByRole("button", { name: "启动" })).toBeVisible();
  });
});
