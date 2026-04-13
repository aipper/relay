---
name: ws-delegate
description: 原生多 agent 委托入口（Codex 优先；先定义角色/边界/工件/降级，再决定是否委托）
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：在 AIWS 治理约束下，只有在任务已绑定且 scope 可控时，才使用当前工具的原生多 agent / sub-agent 能力；否则明确降级为单 agent + 协同工件模式。

阶段定位：
- implementation / review 的辅助入口；它不是独立 workflow 阶段，也不能绕过 `ws-plan`、`ws-dev`、`ws-review`、`ws-finish`。

必需输入：
- 真值文件：`AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`
- delegation contract：`packages/spec/docs/workflow-delegation-contracts.md`
- 当前任务已绑定 `Req_ID` / change / Verify 入口
- 拟委托子任务的角色划分、读写边界、artifact 目标与 fallback

必需输出：
- `Delegation Plan:` 逐项写清 role / task / readScope / writeScope / artifactTargets / fallback
- `Execution Mode:` 明确是 `native multi-agent` 还是 `fallback single-agent`
- `Evidence:` 委托产物路径（`analysis/` / `patches/` / `review/` / `evidence/`）
- `Next:` 收敛后应回到 `ws-dev`、`ws-review`、`ws-commit` 或 `ws-finish`

阻断条件：
- 无法绑定到 `Req_ID` / change / Verify
- 没有明确的 `writeScope` / `artifactTargets`
- 当前工具缺少稳定 native delegation 能力，且你又不能接受降级执行
- 任务规模太小，引入委托只会增加复杂度

完成判定：
- 已经明确本次是否适合委托
- 若适合：delegation plan 已声明并执行，产物回收到 AIWS 约定路径
- 若不适合：已明确降级原因，并回到单 agent 流程继续执行

步骤（建议）：
1) 先读取真值文件与 `packages/spec/docs/workflow-delegation-contracts.md`，确认这不是“为了多 agent 而多 agent”。
2) 判断当前任务是否真的需要委托：
   - 需要并行的只读探索
   - 需要把实现与审查拆开
   - 有明确、互不重叠的写入边界
   - 委托收益大于协调成本
3) 先写 `Delegation Plan:`，至少包含：
   - `role`
   - `task`
   - `readScope`
   - `writeScope`
   - `artifactTargets`
   - `fallback`
4) 若当前环境支持原生多 agent：
   - 优先把探索型工作交给 `explorer`
   - 只把显式授权的文件范围交给 `worker`
   - 至少保留一个独立 `reviewer`
   - 由主 agent / `integrator` 统一收敛结果
5) 若当前环境不支持，或无法稳定约束 scope：
   - 明确输出 `Execution Mode: fallback single-agent`
   - 仍按 AIWS 协同工件约定执行：
     - `changes/<id>/analysis/`
     - `changes/<id>/patches/`
     - `changes/<id>/review/`
     - `changes/<id>/evidence/`
6) 不论是否启用原生多 agent，都不要跳过：
   - 需求归因
   - 验证命令
   - review convergence
   - finish / handoff gate

Codex 优先说明：
- 若当前环境支持原生多 agent / sub-agent，就按 delegation contract 显式拆 role 与 scope。
- 若不支持，或你无法保证写边界，直接降级，不临时发明新的 runtime/controller。

安全：
- 不把 `aiws` 变成统一 orchestrator。
- 不让 delegated agent 越权写未授权文件。
- 不把 `patches/` 视为“已合并代码”。
