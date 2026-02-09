# Tasks: {{CHANGE_ID}}

> Title: {{TITLE}}
>
> Created: {{CREATED_AT}}

## 0. Preflight

- [ ] 0.1 阅读并遵守 `AI_PROJECT.md` / `AI_WORKSPACE.md` / `REQUIREMENTS.md`
- [ ] 0.2 运行门禁校验：`aiws validate .`（或 `npx -y @aipper/aiws validate .`）
- [ ] 0.3 若真值文件发生变化（例如你更新了 REQUIREMENTS.md），同步基线：`aiws change sync {{CHANGE_ID}}`

## 1. 需求/问题合同（如适用）

- [ ] 1.1 需求交付：补齐/更新 `REQUIREMENTS.md` 验收条款（或确认不需要）
- [ ] 1.2 同步 `requirements/requirements-issues.csv`（或更新 `issues/problem-issues.csv`）
- [ ] 1.3 记录到 `requirements/CHANGELOG.md`（如需求发生变化）

## 2. 实现

- [ ] 2.1 <!-- WS:TODO -->
- [ ] 2.2 <!-- WS:TODO -->

## 3. 验证（必须可复现）

- [ ] 3.1 <!-- WS:TODO 写具体命令（来自 AI_WORKSPACE.md） -->
- [ ] 3.2 <!-- WS:TODO 写期望结果（可判断 DONE/FAIL） -->

## 4. 交付与归档

- [ ] 4.1 证据落盘到 `.agentdocs/tmp/...`（报告/日志/请求响应等）
- [ ] 4.2 交叉审计（可选但推荐）：在 AI 工具内运行 `/ws-review`（或按 `AI_PROJECT.md` 手工审计）
- [ ] 4.3 归档：`aiws change archive {{CHANGE_ID}}`
