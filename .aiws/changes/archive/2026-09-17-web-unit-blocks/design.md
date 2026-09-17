# Design: web-unit-blocks

> Title: web-unit-blocks
>
> Created: 2026-09-17T10:33:55Z

## Context

- 背景：web 单测基建（vitest + test:unit）已在 web-unit-vitest 落地并全绿（utils 26 用例）；`web/src/lib/blocks/` 的纯函数（reduce/reconcile，~260 行）是消息渲染管线的核心逻辑，尚无单测覆盖。
- 现状：`reduceToBlocks` 负责把消息事件流折叠成渲染块，`reconcileBlocks` 负责增量合并；两者均为纯函数，无浏览器依赖，可直接在 node 下测试。
- 约束：TypeScript + Svelte 5 项目；测试只新增 `.test.ts`，零产品代码改动；沿用 `web/vitest.config.ts` 最小 node 环境。

## Goals / Non-Goals

**Goals:**
- 为 `reduceToBlocks` 补单测：空输入、单消息、截断边界、输出块形状。
- 为 `reconcileBlocks` 补单测：增量追加、逐行追加 vs 整块追加分支、幂等性。
- 全量 `test:unit` 保持全绿（42/42），`bun run build` 通过。

**Non-Goals:**
- 不测 `relay-store` 单例（Svelte5 runes + 浏览器依赖，已知缺口，前序 change 已记录）。
- 不改产品代码、不调 UI/E2E。

## Decisions

- 断言行为而非实现细节：只覆盖纯函数输入输出契约，避免与内部重构耦合。
- 用例与源码同目录共置（`reduce.test.ts` / `reconcile.test.ts`），与 `utils.test.ts` 模式一致。
- 先行为级覆盖核心分支，不追求行覆盖率数字。

## Risks / Trade-offs

- 风险：`reconcileBlocks` 深层块语义变化时单测滞后 → 缓解：用例锁定输入输出契约，产品逻辑变更时同步更新用例（回归义务）。
- 风险：worker 生成用例可能断言错误行为 → 缓解：主 session 已抽查用例语义 + 全量 test:unit 回放验证。

## Migration / Rollback

- 无数据/接口迁移。回滚路径：`git revert` 测试提交即可，产品代码零改动，回滚无副作用。
