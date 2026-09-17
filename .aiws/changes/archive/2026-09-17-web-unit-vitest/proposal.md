# Change Proposal: web-unit-vitest

> Title: web 前端 vitest 单测基建 + 首批用例
>
> Created: 2026-09-17

## 目标与非目标

**目标：**
- web 拥有可运行的 vitest 单测基建（`bun run test:unit` 全绿，<30s）
- 首批用例锁定 relay-store seq 核心语义：去重 / 回填合并 / 截断上限

**非目标：**
- 不改任何产品逻辑（`web/src` 只读验证，不编辑）
- 不做覆盖率门禁、不拆 relay-store 重构

## 变更归因（强制二选一）

- 需求交付：`Req_ID` = WEB-UNIT-001

## 现状与问题

- `web/src` 下零单测文件，`package.json` 无单测脚本；前端仅靠 Playwright e2e（99 全绿）兜底，反馈慢（~5min）且粒度粗。
- relay-store 的 seq 去重/回填逻辑（paseo-absorb-v1 引入）无单测锁定，后续改动易回归。

## 方案概述（What changes）

- `web/package.json`：加 `vitest` devDep + `test:unit` 脚本
- 新增 `web/vitest.config.ts`（复用现有 vite svelte 插件管线）
- 新增 `web/src/lib/stores/relay-store.seq.test.ts`：seq 去重、after_seq 回填合并、消息截断语义
- 无 BREAKING：纯新增测试文件 + devDep

## 影响范围（Scope）

- 影响的服务/模块/目录：
  - `web/package.json`、`web/vitest.config.ts`（新增）、`web/src/lib/stores/*.test.ts`（新增）
- 可能影响的外部接口/使用方：
  - 无（不改产品代码）

## 风险与回滚

- 风险：
  - `.svelte.ts` runes 文件在 vitest 下编译问题（用 svelte vite 插件管线覆盖）
  - bun run vitest 与 node 行为差异（以 `bun run test:unit` 为准）
- 回滚方案（必须可执行）：
  - `git revert` 本 change 提交；删新增测试文件即回退

## 验证计划（必须可复现）

- 命令：
  - `cd web && bun run test:unit`
  - `cd web && bun run build`
- 期望结果：
  - 单测全绿（≥5 用例），build exit 0

## 真值文件/合同更新清单

- `REQUIREMENTS.md`：不需要（测试基建，不涉及验收条款）
- `requirements/CHANGELOG.md`：不需要
- `requirements/requirements-issues.csv`：已追加 WEB-UNIT-001（READY/TODO）
- `issues/problem-issues.csv`：不需要
- 证据落盘（`.agentdocs/tmp/...`）：`.aiws/changes/web-unit-vitest/evidence/unit-baseline.md`

## 机器绑定

- Change_ID: web-unit-vitest
- Req_ID: WEB-UNIT-001
- Contract_Row: WEB-UNIT-001
- Plan_File: .aiws/plan/2026-09-17_18-25-00-web-unit-vitest.md
- Evidence_Path: .aiws/changes/web-unit-vitest/evidence/
- Change_Type: config-docs
