# Change Proposal: hapi-actionable-cards

> Title: 对齐 hapi 的可操作卡片（ToolCard / PermissionFooter / AwaitingInputFooter）
>
> Created: 2026-02-08

## Bindings

- Change_ID: hapi-actionable-cards
- Req_ID: WEB-062
- Contract_Row(s): Req_ID=WEB-062, Req_ID=WEB-063, Req_ID=WEB-064, Req_ID=SRV-020, Req_ID=HST-020
- Plan_File: plan/hapi-actionable-cards.md
- Evidence_Path(s): .agentdocs/tmp/hapi-actionable-cards/verify.md, .agentdocs/tmp/hapi-actionable-cards-ui/verify.md

## 目标与非目标

**目标：**
- 在 web 的会话事件卡片流中对齐 hapi 的“可操作卡片”体验：
  - ToolCard（`tool.call` / `tool.result`）：摘要、展开、复制。
  - PermissionCard：卡片 footer 直接 approve/deny，支持 `decision=approve_for_session` + `allow_tools` + `answers`。
  - AwaitingInputCard：卡片内输入并发送（不要求切到终端复制粘贴）。
- 协议/后端支持：新增字段向后兼容；decision/allow_tools/answers 可回放（server 落库并通过 messages API 返回）。
- hostd 支持同 run 生命周期内的 allowlist 自动审批（仍生成审计事件）。

**非目标：**
- 不改变鉴权模型（继续 `/auth/login + JWT`）。
- 不引入新依赖。

## 变更归因

- 需求交付：Req_ID = WEB-062
- 需求交付：Req_ID = WEB-063
- 需求交付：Req_ID = WEB-064
- 需求交付：Req_ID = SRV-020
- 需求交付：Req_ID = HST-020

## 方案概述（What changes）

- 协议扩展：`run.permission.approve` 增加可选字段 `decision` / `allow_tools` / `answers`；`run.permission_requested` 可选携带 `questions`。
- hostd：维护 run 生命周期内 allowlist，命中时自动 approve；继续发出 `tool.call`/`tool.result` 以保留审计。
- server：持久化 decision/allow_tools/answers，并在 messages 回放里可见。
- web：在现有事件卡片流基础上补齐 footer 交互（最小可行：approve_for_session 默认 allow_tools=[op_tool]）。

## 风险与回滚

- 风险：协议字段扩展若处理不当可能导致旧客户端/旧 hostd 混跑异常。
- 回滚：
  - 回退协议字段解析与新增语义；保留旧 approve/deny 行为。
  - 回退 web 卡片 footer 交互为仅 approve/deny。

## 验证计划

- `cargo test`
- `cd web && bun run build`
- `aiws validate .`

## 参考真值文件

- `AI_PROJECT.md`
- `AI_WORKSPACE.md`
- `REQUIREMENTS.md`
