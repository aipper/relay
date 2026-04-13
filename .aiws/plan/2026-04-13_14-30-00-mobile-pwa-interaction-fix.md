# Plan: mobile-pwa-fix

## Bindings
- Change_ID: mobile-pwa-fix
- Plan_File: plan/2026-04-13_14-30-00-mobile-pwa-interaction-fix.md
- Evidence_Path: changes/mobile-pwa-fix/
- Contract_Row: N/A
- Req_ID: N/A
- Problem_ID: N/A

## Goal
修复移动端交互问题 - 移除 isMobile 限制

## Non-goals
- 不改变视觉风格
- 不添加新功能

## Scope
- web/src/App.svelte

## Plan (3步)
1. File: web/src/App.svelte line 3072 - 移除 `if (!isMobile) approvalModalOpen = true;` 改为 `approvalModalOpen = true;`
2. File: web/src/App.svelte line 3238 - 移除 isMobile 检查
3. File: web/src/App.svelte line 3250 - 移除 isMobile 检查

## Verify
- 命令: cd web && bun run build
- 期望: BUILD SUCCESS

## Risks & Rollback
- 风险: 低 - 仅修改条件判断
- 回滚: git checkout web/src/App.svelte

## Evidence
- changes/mobile-pwa-fix/