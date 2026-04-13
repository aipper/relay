import { test, expect } from "./fixtures/app";

test.describe("认证流程", () => {
  test("登录页面渲染", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle", { timeout: 30000 });

    await expect(page.getByRole("heading", { name: "Relay" })).toBeVisible();
    await expect(page.getByRole("textbox", { name: "用户名" })).toBeVisible();
    await expect(page.getByRole("textbox", { name: "密码" })).toBeVisible();
    await expect(page.getByRole("button", { name: "登录" })).toBeVisible();
    await expect(page.getByText("刷新后保持登录")).toBeVisible();
  });

  test("登录表单交互", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle", { timeout: 30000 });

    await page.getByRole("textbox", { name: "用户名" }).fill("admin");
    await page.getByRole("textbox", { name: "密码" }).fill("password");

    await expect(page.getByRole("textbox", { name: "用户名" })).toHaveValue("admin");
    await expect(page.getByRole("textbox", { name: "密码" })).toHaveValue("password");
  });

  test("登录按钮启用状态", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle", { timeout: 30000 });

    const loginButton = page.getByRole("button", { name: "登录" });
    const usernameInput = page.getByRole("textbox", { name: "用户名" });
    const passwordInput = page.getByRole("textbox", { name: "密码" });

    await expect(loginButton).toBeDisabled();

    await usernameInput.fill("admin");
    await expect(loginButton).toBeDisabled();

    await passwordInput.fill("123");
    await expect(loginButton).toBeEnabled();
  });

  test("登录成功进入已认证态", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle", { timeout: 30000 });

    await page.getByRole("textbox", { name: "用户名" }).fill("admin");
    await page.getByRole("textbox", { name: "密码" }).fill("password");
    await page.getByRole("button", { name: "登录" }).click();

    await expect(page.getByText("已连接")).toBeVisible({ timeout: 15000 });
    await expect(page.getByRole("navigation")).toBeVisible();
  });

  test("登录失败显示错误", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle", { timeout: 30000 });

    await page.getByRole("textbox", { name: "用户名" }).fill("admin");
    await page.getByRole("textbox", { name: "密码" }).fill("wrongpassword");
    await page.getByRole("button", { name: "登录" }).click();

    await expect(page.getByText(/login failed|invalid credentials|失败|错误/i).first()).toBeVisible({ timeout: 15000 });
  });

  test("记住登录状态", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle", { timeout: 30000 });
    await page.getByRole("textbox", { name: "用户名" }).fill("admin");
    await page.getByRole("textbox", { name: "密码" }).fill("password");
    await page.getByText("刷新后保持登录").click();
    await page.getByRole("button", { name: "登录" }).click();

    await expect(page.getByText("已连接")).toBeVisible({ timeout: 15000 });
    await page.waitForLoadState("domcontentloaded");
    const hasAuth = await page.evaluate(() => {
      const raw = localStorage.getItem("relay.auth.v1");
      if (!raw) return false;
      try { return Boolean(JSON.parse(raw)); } catch { return false; }
    });
    await expect(hasAuth).toBe(true);
  });
});
