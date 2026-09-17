# Change Proposal: web-unit-blocks

> Title: web-unit-blocks
>
> Created: 2026-09-17T10:33:55Z

## 目标与非目标

**目标：**
- reduceToBlocks/reconcileBlocks 被单测锁定：bun run test:unit 全绿

**非目标：**
- 不改产品逻辑；relay-store 直测仍跳过

## 变更归因（强制二选一）

- 需求交付：`Req_ID` = WEB-UNIT-002
- 问题修复：`Problem_ID` = PWA-UNIT-002

> 备注：若“问题阻塞需求”，两边都要在各自 CSV 的 `Notes` 字段互相引用对方 ID。

## 现状与问题

- blocks/ 纯函数无单测覆盖；vitest 基建已就绪（WEB-UNIT-001）

## 方案概述（What changes）

- 新增 reduce.test.ts + reconcile.test.ts；零产品代码改动，无 BREAKING

## 影响范围（Scope）

- 影响的服务/模块/目录：
  - web/src/lib/blocks/
- 可能影响的外部接口/使用方：
  - 无（测试-only）

## 风险与回滚

- 风险：
  - 用例与实现语义分歧（以实现为准修正用例）
- 回滚方案（必须可执行）：
  - 删测试文件即回滚

## 验证计划（必须可复现）

> 从 `AI_WORKSPACE.md` 选择最贴近本变更的验证入口，写成可直接复制执行的命令。

- 命令：
  - bash -c "cd web && bun run test:unit"；bash -c "cd web && bun run build"
- 期望结果：
  - test:unit 全绿；build exit 0

## 真值文件/合同更新清单

- `REQUIREMENTS.md`：不需要
- `requirements/CHANGELOG.md`：不需要
- `requirements/requirements-issues.csv`：已追加 WEB-UNIT-002
- `issues/problem-issues.csv`：需追加 PWA-UNIT-002（归档前）
- 证据落盘（`.agentdocs/tmp/...`）：evidence/blocks-unit.md

## Bindings
Change_ID: web-unit-blocks
Req_ID: WEB-UNIT-002
Problem_ID: PWA-UNIT-002
Contract_Row: Req_ID=WEB-UNIT-002, Problem_ID=PWA-UNIT-002
Plan_File: .aiws/plan/2026-09-17_19-10-00-web-unit-blocks.md
Evidence_Path: .aiws/changes/web-unit-blocks/evidence/
