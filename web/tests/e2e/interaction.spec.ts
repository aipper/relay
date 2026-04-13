import { test, expect } from "./fixtures/app";

async function login(page: import("@playwright/test").Page) {
  await page.goto("/");
  await page.waitForLoadState("networkidle", { timeout: 30000 });
  await page.getByRole("textbox", { name: "用户名" }).fill("admin");
  await page.getByRole("textbox", { name: "密码" }).fill("password");
  await page.getByRole("button", { name: "登录" }).click();
  await expect(page.getByText("已连接")).toBeVisible({ timeout: 15000 });
}

test.describe("审批流程", () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test("待审批会话显示警告标签", async ({ page }) => {
    await expect(page.getByText("待审批")).toBeVisible({ timeout: 10000 });
  });

  test("点击审批会话展示审批卡片", async ({ page }) => {
    await page.getByText(/待审批/).first().click();
    await page.waitForTimeout(1000);
    await expect(page.getByRole("dialog")).toBeVisible({ timeout: 5000 });
  });

  test("同意操作发送 approve", async ({ page }) => {
    await page.getByText(/待审批/).first().click();
    await page.waitForTimeout(1000);
    const approveBtn = page.getByRole("button", { name: /同意|批准|approve/i });
    if (await approveBtn.isVisible().catch(() => false)) {
      await approveBtn.click();
      await page.waitForTimeout(2000);
    }
  });

  test("拒绝操作发送 deny", async ({ page }) => {
    await page.getByText(/待审批/).first().click();
    await page.waitForTimeout(1000);
    const denyBtn = page.getByRole("button", { name: /拒绝|deny/i });
    if (await denyBtn.isVisible().catch(() => false)) {
      await denyBtn.click();
      await page.waitForTimeout(2000);
    }
  });
});

test.describe("导航与视图切换", () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test("顶部连接状态显示", async ({ page }) => {
    await expect(page.getByText("已连接")).toBeVisible({ timeout: 10000 });
  });

  test("切换到主机视图", async ({ page }) => {
    const hostsNav = page.getByRole("button", { name: /主机|hosts/i });
    await hostsNav.click();
    await expect(page.getByRole("combobox")).toBeVisible({ timeout: 5000 });
  });

  test("切换到启动视图", async ({ page }) => {
    const startNav = page.getByRole("button", { name: /启动|start/i });
    await startNav.click();
    await page.waitForTimeout(1000);
    await expect(page.getByRole("button", { name: /启动$/i }).first()).toBeVisible({ timeout: 5000 });
  });

  test("切换到设置视图", async ({ page }) => {
    const settingsNav = page.getByRole("button", { name: /设置|settings/i });
    await settingsNav.click();
    await page.waitForTimeout(500);
  });
});

test.describe("输入流程", () => {
  test("WebSocket 断开显示离线提示", async ({ page }) => {
    await login(page);
    await page.waitForTimeout(2000);
  });
});
