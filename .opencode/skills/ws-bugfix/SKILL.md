---
name: ws-bugfix
description: 使用时机：从禅道/外部系统拉取 bug 进行修复时。触发词：bug、修复、禅道。注意：非禅道小修复请用 ws-dev-lite。
---

目标：用 FSM 驱动 bug 修复全流程——禅道拉取 → 诊断 → 修复 → review → finish。
非目标：不自动 commit/push；不写入 secrets；不凭空改代码。

前置：`$ws-preflight`

## 数据真值约定（所有 Phase 必须遵守）

> **JSON 是 bug 详情唯一真值源**。所有 phase 的 agent 必须从 `bug/zentao-bug-<id>.json` 读取 bug 详情（steps/expect/actual/notes 等），**不得**依赖任何 Markdown 文件中的转述。
>
> Bugfix 目录下 `.md` 文件（如有）仅为人类可读的索引摘要，不具备数据权威性。Dev/Diagnose agent 被喂入 prompt 时，应优先 inline 注入 JSON 原文，而非 MD 的「分析」段。

## Bugfix FSM — 5 个 Phase

使用 `aiws bugfix start|status|advance` 驱动状态机。

### PHASE 0 — INTAKE（数据获取，无 AI 转译）
`aiws bugfix start <bug-id>`
- 创建 change + bugfix-state.json
- 通过 Zentao MCP 拉取 bug 详情
- **原始 JSON** 落盘到 `bug/zentao-bug-<id>.json`（完整字段，不允许裁剪）
- 下载附件图片到 `bug/images/<id>/`
- **不生成 intake Markdown**（不需要 "分析"段或 AI 重述）
- 输出摘要信息到终端即可
- `aiws bugfix advance <change-id>` → DIAGNOSE

### PHASE 1 — DIAGNOSE
- **Bug 详情源**：`bug/zentao-bug-<id>.json`（直接读取，不参考任何 MD）
- 运行 `diagnosing-bugs`（反馈循环 → 复现 → 假设 → 打点）
- 确认 root cause 后 `aiws bugfix advance` → FIX

### PHASE 2 — FIX
- **Bug 详情源**：`bug/zentao-bug-<id>.json`（直接读取）
- 进入 `$ws-dev` 做最小改动
- LSP clean + 测试通过

### PHASE 2.5 — REQ SYNC GATE（硬阻断）

在 FIX 完成后、进入 REVIEW 前，**必须**执行需求同步检查：

1. 对比本次修复涉及的 API 行为、接口字段、错误信息是否与 `REQUIREMENTS.md` 当前描述一致
2. 若存在偏差（例如：bug 暴露了需求描述不准确、修复改变了接口行为、新增了字段/状态码等）：
   - 运行 `$ws-req-change` 更新 `REQUIREMENTS.md`
   - 或记录到 `requirements/CHANGELOG.md`
3. 输出 `REQ_SYNC:` 状态：
   - `SYNCED` — 已同步（REQUIREMENTS.md 已更新）
   - `NOT_NEEDED` — 本次修复不影响需求描述
   - `BLOCKED` — 应更新但未更新，阻断进入 REVIEW
4. 只有 `REQ_SYNC` 为 `SYNCED` 或 `NOT_NEEDED` 时，才能 `aiws bugfix advance` → REVIEW

### PHASE 3 — REVIEW
- `$ws-review` + `$ws-commit`
- review 文件落盘后 `aiws bugfix advance` → FINISH

### PHASE 4 — FINISH
- `$ws-finish` 收尾合并
- 回填 `issues/fix_bus_issues.csv`
- `aiws bugfix advance` → DONE

中断后恢复：`aiws bugfix status <change-id>` 查看当前 phase，继续对应步骤。
