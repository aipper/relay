# Change Proposal: mobile-pwa-fix

> Title: mobile-pwa-fix
>
> Created: 2026-04-13T14:20:30Z
> Change_ID: mobile-pwa-fix
> Plan_File: plan/2026-04-13_14-30-00-mobile-pwa-interaction-fix.md
> Evidence_Path: changes/mobile-pwa-fix/
> Contract_Row: N/A

## 目标与非目标

**目标：**
1. 修复移动端交互问题 - 移除 `isMobile` 限制，允许移动端用户使用输入/审批弹窗
2. 拆分 App.svelte - 将 5794 行单文件拆分为多个模块，优化可维护性

**非目标：**
- 不改变现有 UI 视觉风格
- 不添加新功能
- 不修改后端协议/事件模型

## 变更归因

- 本改为直接修复型改动，不关联需求合同(Req_ID=N/A)或问题跟踪(Problem_ID=N/A)

## 方案概述

1. 修改 `App.svelte` - 移除 `!isMobile` 判断条件（3处）
2. 拆分 `App.svelte` - 按功能模块化拆分到 `lib/` 目录

## 验证计划

- 命令: `cd web && bun run build`
- 期望: 构建成功