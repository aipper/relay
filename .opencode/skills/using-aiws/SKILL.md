---
name: using-aiws
description: 使用时机：新会话开始、不确定下一步做什么时。触发词：路由、bootstrap、Router、工作流入口、下一步做什么。注意：已明确阶段可直接进入对应 ws-*。
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：默认入口，先读真值文件，判定当前任务 workflow，再进入具体 `ws-*` skill。若意图不明确：先澄清，不直接进入实现。

## 编排约束

- **不直接实现**：`using-aiws` 自身不是实现阶段；只判 workflow、读真值、路由到对应 `ws-*`。主 session 不直接写代码。
- **上下文先于判决**：路由 `direct_implementation` 前必须先收集项目上下文——具体检查：`git status --porcelain`（已改动文件数）、`git diff --stat`（改动行范围）、`AI_WORKSPACE.md` 中声明的验证命令可行性。仅当改动文件数 ≤ 3 且改动总行数 ≤ 100 且验证入口明确时，才可判 direct。
- **意图不明先澄清**：`routeTo=clarify` 时必须停止并问 1-3 个关键问题，不猜测。

阶段定位：bootstrap/router 阶段；负责分流，不直接完成实现。

## 必需输入

- 当前任务描述
- `AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`
- 若已存在：`change/<change-id>` 上下文、`plan/...`、`.aiws/changes/<change-id>/...`

## 必需输出

- `Root:` / `Found:` / `OpenCode mode:` / `Task intent:` / `Binding:` / `Route:` / `Why:` / `Next:`

## 阻断条件

- 无法确定项目根目录
- 缺失任一真值文件
- 无法明确当前任务意图
- 无法明确归因或验证入口，且不能安全推断

## 执行步骤

### 1. Preflight

读取 `AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`。

- 若检测到 `.opencode/oh-my-opencode.json`：输出 `oMo-enabled`
- 若未检测到：输出 `standard-opencode`
- 若缺失任一真值文件 → route = `$ws-preflight`，建议先 `aiws init .`

### 1.5 Per-turn Breadcrumb（必做）

每轮对话开始时强制读取当前 change 状态（`.aiws/changes/<change-id>/.ws-change.json` 或 phase 状态文件），输出 `[workflow-state:PHASE/N]` breadcrumb 标记。

目的：即使 context 被压缩，breadcrumb 也能让 AI 知道当前处于哪个阶段、上次做到哪一步。

格式：`[workflow-state:PHASE_NAME/N]`，其中 PHASE_NAME 取自 standardChain，N 为步骤序号。

### 2. 路由判定

根据 `packages/spec/docs/workflow-router-rules.json` 判定：

- 路由前先读上下文：判 direct_implementation 前，router 必须先收集项目上下文（git status、涉及文件范围），确认是真正的单文件修复，再决定 direct vs plan。

上下文收集清单（判 direct_implementation 前必须完成）：
- `git status --porcelain` — 确认未提交改动范围
- `git diff --stat` — 确认改动文件数量
- 改动涉及 ≤2 文件且均在已知路径 → 可判 direct
- 改动涉及 ≥3 文件或含未知路径 → 必须走 ws-plan

| 意图 | Route |
|------|-------|
| 需求/验收/合同变更 | `$ws-req-review` |
| 评审/审计/找风险 | `$ws-review` |
| finish/merge/push/cleanup | `$ws-finish` |
| handoff/archive | `$ws-handoff` |
| 先出设计方案 | `$ws-plan`（设计作为 plan 阶段一部分） |
| 更新规范/验收标准 | `$ws-req-change`（需求变更需先 review） |
| 中大型实现 | `$ws-plan` |
| 小步明确实现/修复 | `$ws-dev` |
| 极简修复 | `$ws-dev-lite` |
| Subagent 不可用 | 回退单 agent + 工件模式 |

注：`$ws-dev` 默认走 subagent-first 策略（详见 `packages/spec/docs/opencode-subagent-first.md`）。主 session 应优先通过 `$ws-delegate` 派发 `aiws-worker`，除非用户明确说"直接改"或"do it inline"。

**Escape Hatch**：若用户明确说"跳过流程"/"直接改"/"do it inline"，允许走 `direct_implementation` 路由，但必须：
1. 输出 `[escape-hatch: direct-implementation]` 标记
2. 仍须归因到 Req_ID / Problem_ID
3. 仍须有可复现验证入口
4. 仍须落盘 evidence 标记 escape-hatch 使用原因

### 3. 意图不明确

只问 1-3 个关键问题，明确缺什么（意图、Req_ID/Problem_ID、verify、change 上下文），然后停止。

### 4. Continuation Routing（新 session 恢复）

`.opencode/plugins/aiws-session-start.js` 自动注入 `<resume-recommendation>` 块，包含 active change ID、phase、journal 摘要及 next action。

续跑决策表（与 `aiws-context.js#getChangeState` 同步）：

```
┌──────────────────┬──────────────────────────────────────────────┐
│ Phase            │ 推荐 Next Action                             │
├──────────────────┼──────────────────────────────────────────────┤
│ none（无 change） │ ws-intake 或 ws-plan 建立 change 上下文       │
│ intake           │ ws-intake 继续澄清，或 ws-plan 转到规划        │
│ planning         │ plan 存在 → ws-plan-verify；否则 ws-plan       │
│ ready-for-dev    │ 派发 aiws-worker（subagent-first）             │
│ in-progress      │ patches 存在 → aiws-reviewer + ws-review      │
│                  │ 上次 DONE_WITH_CONCERNS → ws-quality-review   │
│                  │ 否则 worker 继续或 aiws-reviewer 审查          │
│ review           │ evidence 齐 → ws-finish/ws-commit              │
│                  │ 否则补 evidence 再提交                         │
│ finished         │ ws-finish 收尾归档                             │
│ unknown          │ ws-preflight 重新评估                          │
└──────────────────┴──────────────────────────────────────────────┘
```

Subagent 不可用时的降级路径：
- ready-for-dev 无 subagent → 当前 agent 直接执行 ws-dev（走 inline escape hatch）
- in-progress 无 reviewer → 当前 agent 自审（走 evaluate-optimize 1 轮）
- review 无 oracle → 当前 agent 本地 review（不阻断流程）

特殊状态：
- 上次 delegation 返回 `BLOCKED` → 先解决 blocker 再继续
- 上次 delegation 返回 `NEEDS_CONTEXT` → 补 context JSONL 后重新派发
- Subagent-first 默认：`ready-for-dev` / `in-progress` 默认派发 worker/reviewer
- Subagent 不可用时：降级为当前 agent 直接执行，但必须在 evidence 中记录降级原因
- Subagent 不可用时回退：单 agent 模式 + 显式维护 `.aiws/changes/<id>/` 工件（handoff-evidence.md, analysis/, patches/）；不阻断流程，但须标注 `mode: single-agent`

### 5. 输出路由

```
Root: <path>
Found: <files>
OpenCode mode: <oMo-enabled / standard-opencode>
Task intent: <分类>
Binding: <清晰 / 缺失项>
Route: <$ws-... 或 clarify>
Why: <一句到三句>
Next: <继续执行对应 skill，或提出澄清问题>
```

## 约束

- Router 自己不实现代码；不给 route 前不直接改代码
- 一次只选一个主 route
- 若复杂度升高：`$ws-dev` 回退到 `$ws-plan`；`$ws-finish` 回退到前置门禁
- 若 subagent 不可用：回退为单 agent 执行，但须维护与 subagent 模式等价的工件结构（handoff、evidence、review 文件），确保后续 session 可接力
