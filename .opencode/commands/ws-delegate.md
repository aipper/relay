---
description: 委托：按 AIWS 合同拆分子任务，并优先借用 oMo agent
---
<!-- AIWS_MANAGED_BEGIN:opencode:ws-delegate -->
# ws delegate

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：先写清 delegation plan，再决定是否优先借用 `oh-my-opencode` 的 `planner-sisyphus` / `explore` / `librarian` / `oracle`；若不可用则回退，不阻断流程。

执行建议：
1) 先运行 `/ws-preflight`，再读取 `packages/spec/docs/workflow-delegation-contracts.md` 与 `packages/spec/docs/opencode-omo-adapter.md`。
2) 先写 `Delegation Plan:`，至少包含：
   - `role`
   - `preferred agent`
   - `task`
   - `readScope`
   - `writeScope`
   - `artifactTargets`
   - `fallback`
3) 若检测到 `.opencode/oh-my-opencode.json` 或当前会话明确可用相关 agent：
   - planning 优先 `planner-sisyphus`
   - 探索优先 `@explore` / `@librarian`
   - 独立审查优先 `@oracle`
4) 主 agent 统一收敛结果，并把产物回收到：
   - `changes/<id>/analysis/`
   - `changes/<id>/patches/`
   - `changes/<id>/review/`
   - `changes/<id>/evidence/`
5) 若 oMo agent 不可用：明确回退为普通 OpenCode delegation / 单 agent 执行。
<!-- AIWS_MANAGED_END:opencode:ws-delegate -->

可在下方追加本项目对 OpenCode 的额外说明（托管块外内容会被保留）。
