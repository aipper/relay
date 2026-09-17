# Design: web-unit-more

> Title: web-unit-more
>
> Created: 2026-09-17T10:46:57Z

## Context

- 背景：vitest 基建与 blocks 单测已落地（42 用例）；`web/src/lib/stores/utils.ts` 仍有 12 个纯函数无单测。现状：`relay-store.svelte.ts` 因 Svelte5 runes + 浏览器依赖不可直测。约束：只新增 `*.test.ts`，零产品代码改动。

## Goals / Non-Goals

**Goals:**
- 为 `stores/utils.ts` 剩余 12 个无单测纯函数补单测，全量 `test:unit` 70/70 全绿。

**Non-Goals:**
- 不碰产品代码；不测 `relay-store` 单例；不改现有测试。

## Decisions

- 新增 `web/src/lib/stores/utils-extra.test.ts`（184 行），沿用 vitest 5.0.1 + node 环境既有管线，因为目标全是纯函数、无需 svelte 插件。

## Risks / Trade-offs

- 风险：纯函数后续改签名导致单测过期 → 缓解：单测与源码同目录，改动时就近可见；CI 可跑 `test:unit`（~0.3s，极快）。

## Migration / Rollback

- 无数据/接口迁移。回滚：删除 `utils-extra.test.ts` 单文件即可，不影响产品与构建。
