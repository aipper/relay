# Tasks: mobile-pwa-fix

> Title: mobile-pwa-fix
>
> Created: 2026-04-13T14:20:30Z

## 0. Preflight (已跳过 - 已完成预检)

- [x] 0.1 阅读并遵守 `AI_PROJECT.md` / `AI_WORKSPACE.md` / `REQUIREMENTS.md`
- [x] 0.2 运行门禁校验：`aiws validate .`
- [x] 0.3 真值文件未变化，无需 sync

## 1. 需求/问题合同 (不适用)

- [x] 1.1 本次为修复型改动，不涉及需求合同
- [x] 1.2 不需要更新 requirements-issues.csv
- [x] 1.3 不需要记录到 CHANGELOG.md

## 2. 实现

- [ ] 2.1 修复: 移除 App.svelte 行3072/3238/3250 的 `!isMobile` 限制
  - 文件: App.svelte
  - 改法: 将 `if (!isMobile) xxxModalOpen = true;` 改成 `xxxModalOpen = true;`
- [ ] 2.2 构建验证: `cd web && bun run build`
- [ ] 2.3 拆分 (可选，如果时间允许): 将 App.svelte 拆分为模块

## 3. 验证 (必须可复现)

- [ ] 3.1 构建测试: `cd web && bun run build`
- [ ] 3.2 期望: BUILD SUCCESS，退出码 0
- [ ] 3.3 开发服务器 (可选): `cd web && bun run dev`

## 4. 交付与归档

- [ ] 4.1 交付: `aiws change finish mobile-pwa-fix`
- [ ] 4.2 归档: `aiws change archive mobile-pwa-fix`