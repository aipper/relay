# Tasks: pwa-session-ops

> Title: PWA Session Operations
>
> Created: 2026-04-13T15:33:00Z

## 0. Preflight

- [x] 0.1 阅读并遵守 `AI_PROJECT.md` / `AI_WORKSPACE.md` / `REQUIREMENTS.md`
- [x] 0.2 运行门禁校验：`aiws validate .`
- [x] 0.3 已绑定 WEB-073 需求

## 1. 需求/问题合同

- [x] 1.1 需求确认：WEB-073 (web.opencode.session.lifecycle_and_history)

## 2. 实现

- [x] 2.1 在 PWA 中添加 Session 选择器组件（调用 GET /sessions）
- [x] 2.2 实现"切换到已有 OpenCode Session"功能（rpc.run.start --session）
- [x] 2.3 实现"Fork Session"功能
- [x] 2.4 实现"新建 Session + 绑定 opencode_session_id"功能
- [x] 2.5 编写 Playwright E2E 测试覆盖上述操作

## 3. 验证

- [x] 3.1 运行 `cd web && bun playwright test`
- [x] 3.2 验证 Session 切换/Fork/新建测试用例通过

## 4. 交付与归档

- [x] 3.1 运行 `cd web && bun playwright test tests/e2e/session-*.spec.ts`
- [x] 3.2 6 个 Session 测试用例全部通过
