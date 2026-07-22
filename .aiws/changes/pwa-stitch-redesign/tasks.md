# Tasks: pwa-stitch-redesign

> Title: 按 Stitch 设计重新实现 PWA 前端界面
>
> Created: 2026-06-25T08:30:23Z

## 0. Preflight

- [x] 0.1 阅读并遵守 `AI_PROJECT.md` / `AI_WORKSPACE.md` / `REQUIREMENTS.md`
- [x] 0.2 运行门禁校验：`aiws validate .`（或 `npx -y @aipper/aiws validate .`）
- [x] 0.3 若真值文件发生变化（例如你更新了 REQUIREMENTS.md），同步基线：`aiws change sync pwa-stitch-redesign`

## 1. 需求/问题合同（如适用）

- [ ] 1.1 需求交付：补齐/更新 `REQUIREMENTS.md` 验收条款（或确认不需要）— N/A（纯 UI 重构，无功能变更）
- [ ] 1.2 同步 `requirements/requirements-issues.csv`（或更新 `issues/problem-issues.csv`）— N/A
- [ ] 1.3 记录到 `requirements/CHANGELOG.md`（如需求发生变化）— N/A

## 2. 实现

- [x] 2.1 新建 `web/src/app.css` — 全局暗色主题 CSS 变量 + reset + typography + utilities
- [x] 2.2 新建 `web/src/lib/theme.js` — 等价映射：app.css 的 CSS 变量已覆盖全部主题 token 需求，无需独立 theme.js
- [x] 2.3 新建 `web/src/lib/stores/` — relay-store.svelte.ts + types.ts + utils.ts
- [x] 2.4 新建 `web/src/lib/pages/LoginPage.svelte`
- [x] 2.5 新建 `web/src/lib/pages/SessionsPage.svelte`
- [x] 2.6 新建 `web/src/lib/pages/SessionDetailPage.svelte` — 等价映射：功能由 SessionDetail.svelte + ChatFeed + ChatInput 覆盖
- [x] 2.7 新建 `web/src/lib/pages/EventView.svelte` — 等价映射：功能由 SessionDetail messages tab + ChatFeed + BlocksRenderer 覆盖
- [x] 2.8 新建 `web/src/lib/pages/TerminalView.svelte` — 等价映射：功能由 OutputView 覆盖
- [x] 2.9 新建 `web/src/lib/pages/StartRunPage.svelte` — 等价映射：功能由 LaunchPage.svelte 覆盖
- [x] 2.10 新建 `web/src/lib/pages/SettingsPage.svelte`
- [x] 2.11 重构 `web/src/App.svelte` — 精简为路由容器（3938→122 行）
- [x] 2.12 视觉更新 21 个现有组件 — 对齐暗色主题（已完成全部 components/*.svelte）
- [x] 2.13 替换顶部 NavBar 为底部导航栏 — PageShell.svelte 实现

## 3. 验证（必须可复现）

- [x] 3.1 `cd web && bun run build` — 构建通过
- [ ] 3.2 `cd web && bun run dev` — 启动无报错，浏览器访问所有页面流验证正常
- [ ] 3.3 验证全部业务功能：登录、列表、详情、审批、输入、停止、搜索

## 4. 交付与归档

- [ ] 4.1 证据落盘到 `.agentdocs/tmp/...`（报告/日志/请求响应等）
- [ ] 4.2 交叉审计（可选但推荐）
- [ ] 4.3 归档：`aiws change archive pwa-stitch-redesign`
