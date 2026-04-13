import { test, expect } from "./fixtures/app";

test.describe("响应式布局", () => {
  test("桌面端 - 登录表单全宽", async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 960 });
    await page.goto("/");
    await page.waitForLoadState("networkidle", { timeout: 30000 });

    await expect(page.getByRole("heading", { name: "Relay" })).toBeVisible();
    await expect(page.getByRole("textbox", { name: "用户名" })).toBeVisible();
  });

  test("手机端 - 登录表单适配", async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto("/");
    await page.waitForLoadState("networkidle", { timeout: 30000 });

    await expect(page.getByRole("heading", { name: "Relay" })).toBeVisible();
    await expect(page.getByRole("textbox", { name: "用户名" })).toBeVisible();
    await expect(page.getByRole("textbox", { name: "密码" })).toBeVisible();
  });
});
