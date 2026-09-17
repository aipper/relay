---
name: ws-bugfix-batch
description: 使用时机：批量修复禅道激活 bug 时。触发词：批量、batch、自动修复、循环修复。注意：单个 bug 修复请走 ws-bugfix。
---

目标：自动拉取禅道所有激活 bug，逐条修复、验证、resolve、finish，直至无激活 bug。
非目标：不并行修复（顺序执行）；不修改 batch 状态机字段以外的 JSON（bug 状态由 CLI 管理）。

前置：
- `$ws-preflight`
- 确认 Zentao MCP 可用（`get_my_bugs` / `get_bug_detail` / `resolve_bug`）
- 确认 `opencode` CLI 可用（用于 spawn 子 session）
- 确认 `tmux` 可用（用于子 session 窗口管理）

## Batch 状态模型（SSOT）

状态文件：`.aiws/batches/<batch-id>/batch-state.json`
状态迁移 CLI（sole writer 规则同 ws-goal）：

```bash
aiws bugfix batch start <project-id>    # 创建 batch，写入初始状态
aiws bugfix batch advance <batch-id>    # 当前 bug done，推进到下一个
aiws bugfix batch status [batch-id]     # 读状态（列表/详情）
```

`batch-state.json` 的 bug 列表（`bugs[]`）由 skill 直接写入（skill 有文件写权限），但 `current_bug_index`、`status`（active/completed）和 `bugs[].status`（done）的变更**必须**通过 `aiws bugfix batch advance` 完成，不得手改 JSON。

## 执行流程

### PHASE 0 — INIT

1. `aiws bugfix batch start <project-id>` → 获取 `batchId`
2. 通过 `get_my_bugs`（或项目级查询）拉取当前项目所有激活 bug
3. 对每个 bug，调用 `get_bug_detail` 获取完整 JSON
4. 将 bug 列表写入 `batch-state.json` 的 `bugs[]`（status=`pending`），更新 stats

### PHASE 1 — LOOP（逐 bug 执行）

对 `state.bugs[]` 中 status=`pending` 的 bug，按 index 顺序执行：

#### Phase A — VET（可读性筛查）

1. 从 `batch-state.json` 读取当前 bug
2. 通过 `get_bug_detail`（或已缓存的 JSON）检查：
   - steps 描述是否≥2 步
   - expect 与 actual 是否不同
   - 是否有 environment 信息
3. 若清晰 → 更新 `bugs[i].status = "vetting"`（直接改 JSON），继续
4. 若不清晰 → 更新 `bugs[i].status = "vet_failed"`，设置 `error`，跳过
   - **注意**：此处跳过但不 advance（advance 只在 RESOLVE 成功后调用）

#### Phase B — SPAWN（子 session 修复）

1. 在当前 tmux 窗口创建新窗格或新窗口
2. 启动子 session：
```
tmux send-keys -t <target> "opencode run --auto --no-implicit-connect '在仓库根目录下执行以下操作：

1. aiws bugfix start <bug-id>
2. 诊断并修复 bug #<bug-id>（使用 diagnosing-bugs + ws-dev）
3. 运行测试，将测试输出保存到 evidence：
   - npm run build > .aiws/changes/bugfix-<id>/evidence/build.log 2>&1
   - npm test > .aiws/changes/bugfix-<id>/evidence/test.log 2>&1
4. aiws commit
5. aiws change finish <change-id> --push

注意：
- 使用实际的测试框架运行测试（非 --dry-run）
- evidence/build.log 和 evidence/test.log 必须包含真实输出
- 子 session 自动继承 Zentao MCP 访问权限
'" Enter
```
3. 等待子 session 完成（轮询 `tmux capture-pane` 或检查 evidence 文件）
4. 更新 `bugs[i].change_id = "bugfix-<bug-id>"`，`bugs[i].status = "building"`

#### Phase C — PAROLE（证据审查）

检查 `.aiws/changes/bugfix-<id>/evidence/` 下的证据文件：

1. `build.log` — 须包含真实的 build 输出，exit code 0
2. `test.log` — 须包含真实的 test runner 输出（如 `PASS` / `FAIL` / `ok`），不可是空文件或仅含"build succeeded"
3. 若证据不达标 → `bugs[i].status = "build_failed"`，记录 `error`，跳过

#### Phase D — RESOLVE（禅道回填+推送）

1. 从 `bug/zentao-bug-<id>.json` 和证据中提取 solutionModules：
   - rootCause：诊断得出的根因
   - fixApproach：最小改动方案
   - logicChange：改了什么文件/逻辑
   - impact：影响范围
2. 调用 `resolve_bug` MCP（带完整 `solutionModules` JSON）
3. `git push`（将已完成 commit 的 change branch 推送）
4. 调用 `aiws bugfix batch advance <batch-id>`（更改状态机推进到下一 bug）
5. 若已无剩余 bug → batch status 自动变为 `completed`

### PHASE 2 — REPORT

输出 batch 修复报告：
- total / done / skipped / failed
- 每个 bug 的最终状态
- 失败的 bug 及 error 原因

## Batch FSM 状态图

```
INIT → [bug loop] → VET → SPAWN → PAROLE → RESOLVE → advance → [next bug]
                        ↓          ↓          ↓
                   vet_failed  build_failed  (retry)   → advance marks done
```

- `vet_failed` / `build_failed`：skill 直接写入 bug status
- `done`：仅由 `aiws bugfix batch advance` 写入

## 子 session 契约

子 session 在独立的 tmux 窗口/窗格中运行，负责：
1. `aiws bugfix start <bug-id>`（创建 change 分支）
2. 诊断+修复
3. 构建证据（`build.log` + `test.log`）
4. `aiws commit` + `aiws change finish --push`

主 session 通过 evidence 文件验证子 session 的真实执行，不依赖子 session 的 stdin/stdout 输出。

## 错误处理

- 单个 bug 失败（vet_failed / build_failed）：记录 error，跳过，继续下一个
- subprocess 超时（如 build 卡住）：记录 error，标记 build_failed，继续
- Zentao MCP 不可用：暂停 batch（`status = "paused"`），输出提示
- 中断恢复：`aiws bugfix batch status <batch-id>` 查看当前 index → 从当前 bug 重跑
