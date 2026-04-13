# Tasks: hapi-actionable-cards

> Title: 对齐 hapi 的可操作卡片（Phase 1 + Phase 2）
>
> Created: 2026-02-08

## 0. Preflight

- [x] 0.1 运行门禁校验：`aiws validate .`

## 1. 协议与合同归因

- [x] 1.1 需求归因：`WEB-062` / `WEB-063` / `WEB-064` / `SRV-020` / `HST-020`

## 2. 实现

- [x] 2.1 协议/类型：新增 permission approve 字段（decision/allow_tools/answers）与 requested.questions
- [x] 2.2 hostd：实现 run 生命周期 allowlist 自动审批（审计事件保留）
- [x] 2.3 server：落库并回放 decision/allow_tools/answers
- [x] 2.4 web：卡片 footer 交互（approve_for_session + allow_tools 默认 op_tool；questions->answers 表单）

## 3. 验证（必须可复现）

- [x] 3.1 `cargo test`
- [x] 3.2 `cd web && bun run build`
- [x] 3.3 `aiws validate .`

## 4. 交付

- [x] 4.1 证据落盘到 `.agentdocs/tmp/hapi-actionable-cards/...`
