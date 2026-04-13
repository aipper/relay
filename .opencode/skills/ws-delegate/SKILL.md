---
name: ws-delegate
description: 使用时机：需要拆分子任务、委托给子 agent 时。触发词：委托、子 agent、拆分、并行、sub-agent。注意：简单任务不需委托。
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：在 OpenCode 中，优先借用 `oh-my-opencode` 的现有 agent 做任务拆分；若 oMo 不可用，再回退为普通 OpenCode delegation / 单 agent 执行。

## 核心约束

- **Subagent-First**：主 session 只做编排与收敛，不直接写实现代码。所有产出必须由 subagent 完成并可追溯到具体 worker。
- **Handoff 证据**：每轮委托完成后，worker 必须产出结构化 handoff 到 `.aiws/changes/<id>/handoff-evidence.md`（完成项、未完成项、残余风险），供主 session 收敛判断。handoff 文件缺失等同于委托未完成。

## 必需输入

- 真值文件：`AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`
- delegation contract：`packages/spec/docs/workflow-delegation-contracts.md`
- 上下文策展规范：`packages/spec/docs/workflow-delegation-context-injection.md`
- OpenCode + oMo 适配说明：`packages/spec/docs/opencode-omo-adapter.md`
- 连续执行循环：`packages/spec/docs/opencode-subagent-first.md`
- 当前任务已绑定 `Req_ID` / change / Verify

## 必需输出

- `Delegation Plan:` role / preferred agent / readScope / writeScope / artifactTargets / fallback
- `Context Curation:` 上下文策展详情
- `Execution Mode:` `omo-native` / `opencode-native` / `single-agent`
- `Evidence:` 产物路径
- `Next:` 回到 `ws-dev` / `ws-review` / `ws-commit` / `ws-finish`

## 执行要求

- 主 session 不直接改代码：所有实现与验证产物必须由 subagent 产出并向 handoff 记录可追溯
- handoff 证据：手交材料中须含 delegate round number、产出文件路径、已知未关闭项

## 阻断条件

- 任务未绑定
- 没有写清委托边界
- 上下文策展未执行（未生成 JSONL 或未在 prompt 中引用）
- 无法判断当前是否可用 oMo，又不能接受回退
- handoff 文件 `.aiws/changes/<id>/handoff-evidence.md` 缺失或为空（委托返回后必须检查）

## 角色映射

| aiws 角色 | oMo Agent |
|-----------|-----------|
| `planner` | `planner-sisyphus` |
| `explorer` | `@explore` / `@librarian` |
| `reviewer` | `@oracle` |
| `integrator` | 当前主 agent |

**推荐标准角色**（source: `workflow-delegation-contracts.json` standardRoles）：

| 角色 | 职责 | 读取范围 | 写入范围 |
|------|------|----------|----------|
| `implementer` | 代码+测试实现 | 真值文件+change上下文 | 代码文件+测试文件+evidence/ |
| `reviewer` | 独立审查 | 真值文件+diff+evidence | review/*.md |
| `researcher` | 分析探索 | 真值文件+外部文档 | analysis/*.md |

这些角色为建议非强制；委托时优先参考但允许根据任务需要调整。

## 连续执行循环（Worker → Reviewer → Fix）

默认闭环（详见 `packages/spec/docs/opencode-subagent-first.md`）：

1. 主 session 策展上下文 JSONL → dispatch `aiws-worker`
2. 检查 worker 返回状态（DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED）
3. DONE → dispatch `aiws-reviewer`
4. Reviewer pass → 收敛 evidence；fail → 返回 worker 修复（最多 3 次）
5. DONE_WITH_CONCERNS → 先 `ws-quality-review`
6. NEEDS_CONTEXT → 补充上下文重试（最多 2 次）
7. BLOCKED → 输出 blocker 详情，不继续

## 上下文策展

详细规范见 `packages/spec/docs/workflow-delegation-context-injection.md`。

策展步骤：
1. 读取合同基线（`delegation-contracts.json` 中对应角色的 `contextFiles`）
2. 展开 glob 为实际路径（替换 `<id>`）
3. 委托者调整：添加/删除/调整 priority/sections
4. 预算检查：high+medium ≤ 5 文件，总行数 ≤ 4000
5. 写入 `.aiws/changes/<id>/analysis/<role>-context.jsonl`

OpenCode 插件 `aiws-inject-context` 会自动注入 JSONL 上下文——只需在 `task()` 中指定 `role: <role>`。

## 子 agent 返回协议

```
**Status:** DONE | DONE_WITH_CONCERNS | NEEDS_CONTEXT | BLOCKED
**Completed:** 实现内容
**Files Changed:** 文件路径
**Verification:** 命令 + 结果
**Artifacts:** analysis|patches|review|evidence 下的路径
**Concerns:** 疑虑或未完成项
```

### 状态处理

- **DONE**: 进入 ws-review；若已过 review 则准备 ws-finish
- **DONE_WITH_CONCERNS**: 先 ws-quality-review，根据风险决定是否阻断
- **NEEDS_CONTEXT**: 补上下文重试（最多 2 次）；仍失败则回退单 agent
- **BLOCKED**: 停止委托；解 blocker 后重试；永不到达则升级给用户

## Delegation Plan 格式

```
**Delegation Plan:**
- role: worker
- preferred agent: aiws-worker
- task: <描述>
- readScope: <文件/目录>
- writeScope: <文件/目录>
- artifactTargets: .aiws/changes/<id>/patches/, .aiws/changes/<id>/evidence/
- fallback: single-agent
Context Curation: .aiws/changes/<id>/analysis/worker-context.jsonl
```

## 委托者检查清单

派遣前：
- [ ] 子 agent prompt 包含上下文引用
- [ ] JSONL 已写入 `.aiws/changes/<id>/analysis/<role>-context.jsonl`
- [ ] 预算检查通过
- [ ] readScope / writeScope / artifactTargets 已声明

返回后：
- [ ] 解析 Status 行
- [ ] 非 DONE → 按状态处理规则行动
- [ ] 非 DONE → 记录决策到 `delegation-decisions.md`
- [ ] handoff 文件 `.aiws/changes/<id>/handoff-evidence.md` 已存在且非空

安全：
- 不让 `ws-delegate` 变成第二套 orchestrator
- 不让 delegated agent 越权写未授权文件
- 不跳过 submodule drift check（若 `.gitmodules` 存在）
