# Change Proposal: pwa-session-ops

> Title: PWA Session Operations
>
> Created: 2026-04-13T15:33:00Z

## 目标与非目标

**目标：**
- 在 PWA 中实现 Session 生命周期管理能力（切换已有 Session / Fork Session / 新建 Session 绑定 opencode_session_id），对齐 REQUIREMENTS.md 中的 H1 目标。
- 编写 Playwright E2E 测试覆盖上述操作的端到端验证。

**非目标：**
- 不实现跨端加密同步与离线合并策略（对齐 Happy 的完整实现）。

## 变更归因

- 需求交付：Req_ID = WEB-073

## Bindings

- Change_ID = pwa-session-ops
- Contract_Row = WEB-073
- Plan_File = plan/2026-04-13-pwa-session-ops.md
- Evidence_Path = .agentdocs/tmp/playwright-session-*

## 现状与问题

- 当前 PWA 只能启动新 run，无法显式切换到已有的 OpenCode Session
- 当前 PWA 没有 Fork Session 功能
- 当前 PWA 新建 Session 时无法指定 opencode_session_id
- 后端（Server + hostd）已具备基础能力（Session API + `--session` 续接），但缺少前端 UI 入口

## 方案概述

### Phase 1: PWA Session UI 实现
1. 在 PWA 中添加 Session 选择器组件
2. 实现 "切换到已有 OpenCode Session" 功能
3. 实现 "Fork Session" 功能（克隆当前 session 状态）
4. 实现 "新建 Session + 绑定 opencode_session_id" 功能

### Phase 2: Playwright E2E 测试
1. 覆盖 Session 切换流程
2. 覆盖 Fork Session 流程
3. 覆盖新建 Session 流程

## 影响范围

- 影响的服务/模块/目录：
  - `web/`（PWA Svelte 前端）
  - `server/`（如有新增 API 需求）
- 可能影响的外部接口/使用方：
  - OpenCode 会话管理

## 风险与回滚

- 风险：
  - 前端改动可能影响现有 UI 布局
- 回滚方案：
  - `git checkout changes/pwa-session-ops^0 -- web/`

## 验证计划

- 命令：`cd web && bun playwright test`
- 期望结果： Session 操作相关测试用例通过

## 真值文件/合同更新清单

- `REQUIREMENTS.md`：已包含 H1 验收条款，本次为增量实现
- `requirements/requirements-issues.csv`：如有必要，添加实现任务行
- 证据落盘：测试报告 `.agentdocs/tmp/playwright-*.html`