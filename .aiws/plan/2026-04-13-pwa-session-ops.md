# Plan: PWA Session Operations

## Bindings

- Change_ID = pwa-session-ops
- Req_ID = WEB-073
- Contract_Row = WEB-073
- Plan_File = plan/2026-04-13-pwa-session-ops.md
- Evidence_Path = .agentdocs/tmp/playwright-session-*

## Goal

在 PWA 中实现 Session 生命周期管理能力（切换/Fork/新建），并编写 Playwright E2E 测试。

## Non-goals

- 不实现跨端加密同步与离线合并
- 不修改 Server 核心 API（现有 Session API 足够）

## Scope

- `web/src/lib/`（PWA 前端组件）
- `web/tests/`（Playwright 测试）

## Plan

1. 创建 `web/src/lib/SessionSelector.svelte`，调用 `GET /sessions` API
2. 在 `web/src/App.svelte` 集成选择器，调用 `rpc.run.start --session <id>` 续接
3. 调用 `POST /sessions` 创建新 session，然后 `rpc.run.start --session <new_id>`
4. 在 RunStartDialog 添加 session name 输入，传递 `opencode_session_id` 到 hostd
5. 创建 `web/tests/session-switch.spec.ts`
6. 创建 `web/tests/session-fork.spec.ts`
7. 创建 `web/tests/session-new.spec.ts`

## Verify

```bash
cd web && bun playwright test --reporter=html
```

期望：Session 操作测试用例通过。

## Risks & Rollback

- UI 改动可能影响现有布局
- 回滚：`git checkout changes/pwa-session-ops^0 -- web/`

## Evidence

- Plan: plan/2026-04-13-pwa-session-ops.md
- Change: changes/pwa-session-ops/proposal.md
- Tests: web/tests/session-*.spec.ts