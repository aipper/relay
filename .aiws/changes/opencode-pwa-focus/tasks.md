# Tasks: opencode-pwa-focus

> Title: OpenCode-first 运行时收敛与 PWA 对齐
>
> Created: 2026-03-19T03:02:10Z

## 0. Preflight

- [x] 0.1 阅读并遵守 `AI_PROJECT.md` / `AI_WORKSPACE.md` / `REQUIREMENTS.md`
- [x] 0.2 运行门禁校验：`aiws validate .`（或 `npx -y @aipper/aiws validate .`）
- [x] 0.3 若真值文件发生变化（例如你更新了 REQUIREMENTS.md），同步基线：`aiws change sync opencode-pwa-focus`

## 1. 需求/问题合同（如适用）

- [ ] 1.1 运行 `/ws-req-review`，确认“OpenCode-first”方向的需求边界
- [x] 1.2 更新 `REQUIREMENTS.md`
- [ ] 1.3 记录到 `requirements/CHANGELOG.md`
- [ ] 1.4 更新 `requirements/requirements-issues.csv`

## 2. 实现

- [x] 2.1 把 OpenCode `sessionID` 提升为 relay 的一等模型并向 server/web 暴露
- [ ] 2.2 为 PWA/CLI 增加 OpenCode-first 的 session/todo/history 能力
- [x] 2.3 将当前运行时范围收敛为仅支持 `opencode`
- [x] 2.4 为 structured OpenCode 增加 silent watchdog、模型护栏与错误可见性

## 3. 验证（必须可复现）

- [x] 3.1 `python3 tools/requirements_contract.py validate`
- [x] 3.2 `aiws validate .`

## 4. 交付与归档

- [x] 4.1 证据落盘到 `.agentdocs/tmp/opencode-pwa-focus-feasibility-20260319.md`
- [x] 4.1b 补充运行时与交付证据到 `changes/opencode-pwa-focus/evidence/`
- [ ] 4.2 交叉审计（可选但推荐）：在 AI 工具内运行 `/ws-review`
- [ ] 4.3 归档：`aiws change archive opencode-pwa-focus`
