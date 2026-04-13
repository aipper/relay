<!-- AIWS_MANAGED_BEGIN:claude:using-aiws -->
# using aiws

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 把当前任务先路由到正确的 AIWS workflow，而不是直接跳进实现。

执行建议：
1) 先读取 `AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`。
2) 若缺失任一真值文件：先 `/ws-preflight`，必要时运行 `aiws init .`，不要继续实现。
3) 若任务意图、归因或验证入口不明确：先提 1-3 个关键澄清问题并停止。
4) 路由规则：
   - 需求/验收/合同变更：`/ws-req-review`
   - 中大型实现或需要 change/worktree：`/ws-plan`
   - 小步明确实现：`/ws-dev`（若是 simple/local 单点修复，可显式进入 `/ws-dev-lite`）
   - 评审/审计：`/ws-review`
   - finish / merge / push / cleanup：`/ws-finish`
   - handoff / archive summary：`/ws-handoff`
5) 输出 `Task intent:` / `Binding:` / `Route:` / `Why:` / `Next:`。
6) 若 `Route: ws-dev` 且任务属于 simple/local 单点修复：`Next` 可显式建议 `/ws-dev-lite`。
7) 除非用户只要路由判断，否则给出 route 后继续遵循对应入口的契约。
<!-- AIWS_MANAGED_END:claude:using-aiws -->

可在下方追加本项目对 Claude Code 的额外说明（托管块外内容会被保留）。
