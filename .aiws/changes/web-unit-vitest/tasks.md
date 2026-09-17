# Tasks: web-unit-vitest

> Title: web 前端 vitest 单测基建 + 首批用例
>
> Created: 2026-09-17

## 0. Preflight

- [x] 0.1 阅读并遵守 `AI_PROJECT.md` / `AI_WORKSPACE.md` / `REQUIREMENTS.md`
- [ ] 0.2 运行门禁校验：`aiws validate .`（预置 `.agents/skills` 漂移已知阻断，见前 changes 记录）
- [ ] 0.3 若真值文件发生变化，同步基线：`aiws change sync web-unit-vitest`

## 1. 需求/问题合同（如适用）

- [x] 1.1 需求交付：确认不需要改 `REQUIREMENTS.md`（测试基建）
- [x] 1.2 已追加 `requirements-issues.csv` WEB-UNIT-001 行
- [x] 1.3 `requirements/CHANGELOG.md` 不需要（需求未变）

## 2. 实现

- [x] 2.1 安装 vitest devDep + 新增 vitest.config.ts + package.json test:unit 脚本
- [x] 2.2 新增单测（换路：utils.ts 纯函数 26 用例；relay-store 直测因浏览器依赖跳过，已记录）（去重/回填合并/截断），`bun run test:unit` 全绿

## 3. 验证（必须可复现）

- [x] 3.1 `cd web && bun run test:unit` 全绿 + `bun run build` exit 0
- [x] 3.2 证据落盘 `.aiws/changes/web-unit-vitest/evidence/unit-baseline.md`

## 4. 交付与归档

- [x] 4.1 证据落盘（见 3.2）
- [ ] 4.2 交叉审计 `$ws-review`（test-only，通用 review）
- [ ] 4.3 归档：`aiws change archive web-unit-vitest`
