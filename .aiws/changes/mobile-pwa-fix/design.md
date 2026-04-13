# Design: mobile-pwa-fix

> Title: mobile-pwa-fix
>
> Created: 2026-04-13T14:20:30Z

## Context

- **背景**: 移动端 PWA 用户无法使用输入/审批弹窗
- **框架**: Svelte + Vite
- **约束**: 5794 行单文件 App.svelte

## Goals / Non-Goals

**Goals:**
1. 修复移动端交互 - 移除 isMobile 限制

**Non-Goals:**
- 不改变视觉风格
- 不添加功能

## Decisions

1. **直接移除 isMobile 条件判断** - 最简修复方案
2. 后续可考虑拆分 App.svelte

## Risks / Trade-offs

- **风险**: 拆分可能引入路径错误 → **缓解**: 分步验证，每次构建

## Migration / Rollback

- **回滚**: `git checkout App.svelte` 恢复