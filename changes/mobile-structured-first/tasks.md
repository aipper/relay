# Tasks: mobile-structured-first

> Title: 移动端默认会话卡片流优先（structured-first）
>
> Created: 2026-02-08

## 0. Preflight

- [x] 0.1 阅读并遵守 `AI_PROJECT.md` / `AI_WORKSPACE.md` / `REQUIREMENTS.md`
- [x] 0.2 运行门禁校验：`aiws validate .`（或 `npx -y @aipper/aiws validate .`）
- [x] 0.3 若真值文件发生变化（例如你更新了 REQUIREMENTS.md），同步基线：`aiws change sync mobile-structured-first`

## 1. 需求/问题合同（如适用）

- [x] 1.1 需求交付：补齐/更新 `REQUIREMENTS.md` 验收条款（或确认不需要）
- [x] 1.2 同步 `requirements/requirements-issues.csv`（或更新 `issues/problem-issues.csv`）
- [x] 1.3 记录到 `requirements/CHANGELOG.md`（如需求发生变化）

## 2. 实现

- [x] 2.1 调整移动端会话打开/恢复/切换默认进入 `messages` 卡片流
- [x] 2.2 保持终端为次级入口，仅显式操作进入且不跨会话继承

## 3. 验证（必须可复现）

- [x] 3.1 `cd web && bun run build`
- [x] 3.2 `aiws validate .`

## 4. 交付与归档

- [x] 4.1 证据落盘到 `.agentdocs/tmp/...`（报告/日志/请求响应等）
- [ ] 4.2 交叉审计（可选但推荐）：在 AI 工具内运行 `/ws-review`（或按 `AI_PROJECT.md` 手工审计）
- [ ] 4.3 归档：`aiws change archive mobile-structured-first`
