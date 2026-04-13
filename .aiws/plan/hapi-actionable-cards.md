# Plan: hapi-actionable-cards

## Bindings

- Change_ID: hapi-actionable-cards
- Req_ID: WEB-062
- Contract_Row(s): Req_ID=WEB-062, Req_ID=WEB-063, Req_ID=WEB-064, Req_ID=SRV-020, Req_ID=HST-020
- Plan_File: plan/hapi-actionable-cards.md
- Evidence_Path(s): .agentdocs/tmp/hapi-actionable-cards/verify.md, .agentdocs/tmp/hapi-actionable-cards-ui/verify.md

## Goal

- 对齐 hapi 的可操作卡片交互：ToolCard、PermissionCard footer、AwaitingInputCard footer；并确保权限决定可回放。

## Non-goals

- 不更改鉴权模型（继续 `/auth/login + JWT`）。
- 不引入新前端依赖。

## Scope

- `web/src/App.svelte`
- `hostd/src/run_manager.rs`
- `server/src/main.rs`, `server/src/db.rs`
- `protocol/src/lib.rs`

## Plan

- [x] 协议：扩展 permission payload 字段（`decision/allow_tools/answers/questions`）并保持向后兼容
- [x] hostd：实现 run 生命周期 allowlist 自动审批（tmux/PTY 不在本计划范围）
- [x] server：持久化审批决定并在 messages 回放
- [x] web：在事件卡片流内实现 approve_for_session/answers/就地输入与复制

## Verify

- 命令：
  - `cargo test`
  - `cd web && bun run build`
  - `aiws validate .`
- 期望：
  - 测试与构建通过；`aiws validate .` 无 error。

## Risks & Rollback

- 风险：字段扩展若处理不当会导致旧版本 client/hostd 混跑不兼容。
- 回滚：回退新增字段解析与语义，只保留 approve/deny 基本闭环。

## Evidence

- `.agentdocs/tmp/hapi-actionable-cards/verify.md`
- `.agentdocs/tmp/hapi-actionable-cards-ui/verify.md`
