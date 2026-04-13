---
name: using-aiws
description: 默认 workflow bootstrap/router：先读真值，再路由到正确的 ws-* 入口
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 作为默认入口，先读取 `AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`
- 先判定当前任务属于哪个 workflow，再进入具体 `ws-*` skill
- 若任务意图、归因或验证入口不明确：先澄清，不直接进入实现
- 若任务方向大致明确但仍有多条待确认问题：先进入 `$ws-intake` 冻结需求，再进入计划

阶段定位：
- bootstrap/router 阶段；负责 workflow 分流，而不是直接完成实现。

必需输入：
- 当前任务描述
- `AI_PROJECT.md`
- `REQUIREMENTS.md`
- `AI_WORKSPACE.md`
- 若已存在：当前 `change/<change-id>` 上下文、`plan/...`、`changes/<change-id>/...`

必需输出：
- `Root:` 当前项目根
- `Found:` 实际读取到的真值文件
- `Intent status:` 当前需求是否已经冻结
- `Task intent:` 当前任务意图分类
- `Binding:` `Req_ID` / `Problem_ID` / change 上下文是否清晰
- `Route:` 选中的下一步 skill
- `Why:` 选择该 route 的原因
- `Next:` 进入对应 skill，或先提澄清问题

阻断条件：
- 无法确定项目根目录
- 缺失任一真值文件
- 无法明确当前任务意图
- 无法明确归因或验证入口，且不能安全推断

完成判定：
- 已经明确给出单一路由结果，并进入对应 `ws-*` skill；或已提出关键澄清问题并停止。

执行步骤（强制）：
1) 先遵守 `$ws-preflight` 的真值读取要求：
   - 定位项目根目录
   - 读取 `AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`
   - 输出 `Root:` / `Found:` / `Missing:`
2) 若缺失任一真值文件：
   - 不进入实现
   - 输出下一步：先 `aiws init .`（或 `npx @aipper/aiws init .`），然后重新执行 `$using-aiws`
   - 此时 route 视为 `$ws-preflight`
3) 根据 `packages/spec/docs/workflow-router-rules.json` 判定任务属于哪一类：
   - 需求/验收/合同变更：`$ws-req-review`
   - 评审/审计/找风险：`$ws-review`
   - finish / merge / push / cleanup：`$ws-finish`
   - handoff / archive summary：`$ws-handoff`
   - 新需求或中大型变更，且存在多条待确认问题 / 用户要求逐条多轮沟通：`$ws-intake`
   - 中大型实现、需要方案、需要 change/worktree，且需求已经冻结：`$ws-plan`
   - 小步明确实现/修复/配置调整：`$ws-dev`（若是 simple/local 单点修复，且用户明确希望走轻量入口，可显式进入 `$ws-dev-lite`）
4) 若任务意图或归因不明确：
   - 只问 1-3 个关键澄清问题
   - 明确写出缺的是什么：意图、`Req_ID` / `Problem_ID`、verify、change 上下文
   - 然后停止；不要直接写代码
5) 若任务方向大致明确，但同一个任务包含多条待确认问题、范围尚未冻结，或用户明确说“想一条一条聊清楚再计划”：
   - 输出 `Intent status: not_frozen`
   - `Route: $ws-intake`
   - `Why:` 说明当前不适合直接进入 `$ws-plan`
   - `Next:` 进入 `$ws-intake`
   - 然后在同一轮中继续遵循 `$ws-intake` 的契约
6) 若已确定 route：
   - 先输出：
     - `Intent status:`
     - `Task intent:`
     - `Binding:`
     - `Route:`
     - `Why:`
   - 若 `Route: $ws-dev` 且任务属于 simple/local 单点修复：
     - `Next:` 可优先进入 `$ws-dev-lite`
   - 否则：
     - `Next:` 进入对应主 skill
   - 然后在同一轮中继续遵循对应 skill 的契约
7) 路由约束：
   - router 自己不是实现阶段；不要在给出 route 之前直接改代码
   - 除非用户只想听判断结果，否则给出 route 后应继续按对应 skill 的规则推进
   - 一次只选择一个主 route；不要把 `ws-plan` / `ws-dev` / `ws-review` / `ws-finish` 混成并行终态
8) 若执行过程中发现复杂度升高：
   - 从 `$ws-dev` 回退到 `$ws-plan`
   - 从 `$ws-plan` 回退到 `$ws-intake`（当发现需求并未冻结）
   - 从 `$ws-finish` 回退到前置门禁而不是硬推完成

输出模板：
- `Root:` <path>
- `Found:` <files>
- `Intent status:` <frozen / not_frozen>
- `Task intent:` <分类>
- `Binding:` <清晰 / 缺失项>
- `Route:` <$ws-... 或 clarify>
- `Why:` <一句到三句>
- `Next:` <继续执行对应 skill，或提出澄清问题>
