---
name: ws-delegate
description: 使用时机：需要拆分子任务、委托给子 agent 时。触发词：委托、子 agent、拆分、并行、sub-agent。注意：简单任务不需委托。
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：优先 oMo agent 拆分任务；不可用则回退 OpenCode delegation / 单 agent。

## 核心约束

- **Subagent-First**：主 session 只编排收敛，不写代码；产出由 subagent 完成并可追溯。
- **Handoff 证据**：worker 必须产出 `.aiws/changes/<id>/handoff-evidence.md`（完成/未完成/残余风险）。缺失=委托未完成。

## 必需输入 / 输出

**输入：** 真值 + delegation contract 上下文（`workflow-delegation-contracts.md`、`opencode-omo-adapter.md`、`opencode-subagent-first.md`）；任务已绑定 `Req_ID` / change / Verify。

**输出：** `Delegation Plan:` role / preferred agent / readScope / writeScope / artifactTargets / fallback；`Context Curation:` / `Execution Mode:` / `Evidence:` / `Next:`。

**执行：** 主 session 不改代码；产物由 subagent 可追溯产出；handoff 含 delegate round number、产出路径、未关闭项。

**阻断：** 任务未绑定 / 边界不清 / 未策展上下文 / 无法判断 oMo 可用性 / handoff 缺失。

## 角色映射

| aiws | oMo | 标准角色 | 职责 | 读 | 写 |
|------|-----|----------|------|----|----|
| planner | planner-sisyphus | implementer | 代码+测试 | 真值+change | 代码+测试+evidence/ |
| explorer | @explore / @librarian | reviewer | 独立审查 | 真值+diff+evidence | review/*.md |
| reviewer | @oracle | researcher | 分析探索 | 真值+外部文档 | analysis/*.md |
| integrator | 当前主 agent | | | | |

## 连续执行循环

1. 主 session 策展 JSONL → dispatch `aiws-worker`
2. 检查 Status：DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED
3. DONE → dispatch `aiws-reviewer`；pass→收敛 evidence；fail→worker 修复（≤3）
4. DONE_WITH_CONCERNS → 先 `ws-quality-review`
5. NEEDS_CONTEXT → 补上下文重试（≤2）；仍失败→回退单 agent
6. BLOCKED → 输出 blocker，不继续

## 上下文策展

1. 读合同 `contextFiles` → 2. 展开 glob（`<id>`）→ 3. 委托者调整（增删/priority/sections）→ 4. 预算 high+medium ≤5 文件、总行 ≤4000 → 5. 写 `.aiws/changes/<id>/analysis/<role>-context.jsonl`

插件 `aiws-inject-context` 自动注入；`task()` 指定 `role: <role>` 即可。

## 子 agent 返回协议

```
**Status:** DONE | DONE_WITH_CONCERNS | NEEDS_CONTEXT | BLOCKED
**Completed:** <实现内容>
**Files Changed:** <路径>
**Verification:** <命令+结果>
**Artifacts:** <analysis|patches|review|evidence 路径>
**Concerns:** <疑虑或未完成项>
```

## Delegation Plan 格式

```
**Delegation Plan:**
- role: worker | preferred agent: aiws-worker
- readScope: <...> | writeScope: <...>
- artifactTargets: .aiws/changes/<id>/patches/, .aiws/changes/<id>/evidence/
- fallback: single-agent
Context Curation: .aiws/changes/<id>/analysis/worker-context.jsonl
```

## 委托者检查清单

**派遣前：** `[ ] prompt 含上下文引用` `[ ] JSONL 已写` `[ ] 预算通过` `[ ] readScope/writeScope/artifactTargets 已声明`

**返回后：** `[ ] 解析 Status` `[ ] 非 DONE→按规则处理` `[ ] 非 DONE→记入 delegation-decisions.md` `[ ] handoff 存在且非空`

**安全：** 不把 `ws-delegate` 做成第二套 orchestrator；delegated agent 不越权写未授权文件；有 `.gitmodules` 时不跳过 submodule drift check。

> 运行时行为约束：`packages/spec/docs/run-behavior-guidelines.md`
