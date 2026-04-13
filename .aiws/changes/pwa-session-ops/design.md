# Design: pwa-session-ops

> Title: PWA Session Operations
>
> Created: 2026-04-13T15:33:00Z

## Context

- 背景：需要让 PWA 具备 Session 级别管理能力，对齐 Happy 的 Session 模型
- 现状：后端已有 Session API，前端缺少 UI 入口

## Goals

- 在 PWA 中实现"切换已有 Session"、"Fork Session"、"新建 Session + 绑定 opencode_session_id"
- 编写 Playwright E2E 测试覆盖

## Non-Goals

- 不实现跨端加密同步与离线合并
- 不修改 Server 核心 API

## Decisions

- 使用现有 Server Session API（无需新增 API）
- Session 选择器组件基于现有 runs 列表组件扩展

## Risks

- UI 改动可能影响现有布局 → 回滚可快速恢复
- 需要真实 OpenCode session 测试数据

## Migration / Rollback

- 回滚：`git checkout changes/pwa-session-ops^0 -- web/`