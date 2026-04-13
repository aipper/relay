---
name: ws-delegate
description: 原生多 agent 委托入口（OpenCode + oMo 优先；先写委托合同，再调用对应 agent）
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：在 OpenCode 中，优先借用 `oh-my-opencode` 的现有 agent 做任务拆分；若 oMo 不可用，再回退为普通 OpenCode delegation / 单 agent 执行。

必需输入：
- 真值文件：`AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`
- delegation contract：`packages/spec/docs/workflow-delegation-contracts.md`
- OpenCode + oMo 适配说明：`packages/spec/docs/opencode-omo-adapter.md`
- 当前任务已绑定 `Req_ID` / change / Verify

必需输出：
- `Delegation Plan:` 写清 role / preferred agent / readScope / writeScope / artifactTargets / fallback
- `Execution Mode:` `omo-native` / `opencode-native` / `single-agent`
- `Evidence:` 产物路径
- `Next:` 回到 `ws-dev` / `ws-review` / `ws-commit` / `ws-finish`

阻断条件：
- 任务未绑定
- 没有写清委托边界
- 无法判断当前是否可用 oMo，又不能接受回退

推荐映射：
- `planner` -> `planner-sisyphus`
- `explorer` -> `@explore` / `@librarian`
- `reviewer` -> `@oracle`
- `integrator` -> 当前主 agent

执行步骤（建议）：
1) 先读真值文件、delegation contract 和 `opencode-omo-adapter.md`。
2) 判断当前项目是否启用了 oMo：
   - 优先检查 `.opencode/oh-my-opencode.json`
   - 或当前会话是否明确可用 `planner-sisyphus` / `librarian` / `explore` / `oracle`
3) 先写 `Delegation Plan:`：
   - `role`
   - `preferred agent`
   - `task`
   - `readScope`
   - `writeScope`
   - `artifactTargets`
   - `fallback`
4) 若 oMo 可用：
   - planning 优先 `planner-sisyphus`
   - 文档/知识检索优先 `@librarian`
   - 代码导航与结构探索优先 `@explore`
   - 独立审查优先 `@oracle`
5) 主 agent 统一收敛结果，并把产物回收到：
   - `changes/<id>/analysis/`
   - `changes/<id>/patches/`
   - `changes/<id>/review/`
   - `changes/<id>/evidence/`
6) 若 oMo 不可用：
   - 输出 `Execution Mode: opencode-native` 或 `Execution Mode: single-agent`
   - 不阻断工作流，继续按 AIWS 委托合同执行

安全：
- 不让 `ws-delegate` 变成第二套 orchestrator。
- 不覆盖 oMo 的 hooks / session / agent harness。
- 不让 delegated agent 越权写未授权文件。
