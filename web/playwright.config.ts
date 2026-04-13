import { defineConfig, devices } from "@playwright/test";

const CI = !!process.env.CI;

export default defineConfig({
  testDir: "./tests/e2e",
  timeout: 30_000,
  expect: { timeout: 10_000 },
  fullyParallel: false,
  workers: 1,
  forbidOnly: CI,
  retries: CI ? 2 : 0,
  reporter: CI
    ? [
        ["list"],
        ["junit", { outputFile: "test-results/playwright/junit.xml" }],
        ["html", { open: "never" }],
      ]
    : [["list"], ["html", { open: "never" }]],
  use: {
    baseURL: "http://127.0.0.1:4173",
    trace: "retain-on-failure",
    video: "retain-on-failure",
    screenshot: "only-on-failure",
    locale: "zh-CN",
    timezoneId: "Asia/Shanghai",
  },
  webServer: [
    {
      command: "RELAY_DISABLE_PWA=1 bunx vite --host 127.0.0.1 --port 4173",
      url: "http://127.0.0.1:4173",
      timeout: 60_000,
      reuseExistingServer: !CI,
    },
  ],
  projects: [
    {
      name: "desktop",
      use: {
        ...devices["Desktop Chrome"],
        viewport: { width: 1440, height: 960 },
        launchOptions: {
          args: [
            "--no-proxy-server",
          ],
        },
      },
    },
    {
      name: "tablet",
      use: {
        ...devices["iPad Pro 11"],
        browserName: "chromium",
        launchOptions: {
          args: ["--no-proxy-server"],
        },
      },
    },
    {
      name: "mobile",
      use: {
        ...devices["iPhone 13"],
        browserName: "chromium",
        launchOptions: {
          args: ["--no-proxy-server"],
        },
      },
    },
  ],
});
