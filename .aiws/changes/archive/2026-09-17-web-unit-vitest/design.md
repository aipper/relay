# Design: web-unit-vitest

> Title: web-unit-vitest
>
> Created: 2026-09-17T10:19:08Z

## Context

- web/ 仅有 Playwright e2e（99 用例，需浏览器 ~5min），无单测基建；纯函数（stores/utils.ts）零覆盖，回归反馈慢。约束：Svelte5 + vite6 + bun；单测须 node 下 <30s 跑完。

## Goals / Non-Goals

**Goals:**
- vitest 基建落地（test:unit 脚本 + 最小 node 配置），首批覆盖 utils.ts 纯函数。

**Non-Goals:**
- 不测 .svelte 组件渲染；不直测 relay-store 类实例（runes + 浏览器依赖，本轮跳过，见 evidence/unit-baseline.md）。

## Decisions

- vitest 5 node 环境、不引 svelte 插件管线：relay-store 需 runes 编译，首轮只测纯函数，配置保持最小；后续如需组件/类测试再引入 jsdom。
- 首批目标选 utils.ts：纯函数、无浏览器依赖、覆盖 seq 语义外围（truncateHead 截断上限）。

## Risks / Trade-offs

- relay-store seq 去重/回填逻辑仍无单测 → 缓解：e2e 全绿兜底 + evidence 如实记录缺口，后续补 jsdom 收敛。

## Migration / Rollback

- 无数据/接口迁移。回滚：`git revert` 基建提交即可（新增文件 + package.json/bun.lock 增量，无产品代码耦合）。
