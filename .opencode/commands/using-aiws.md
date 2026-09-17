---
description: 默认 workflow router：先判定阶段，再进入具体 ws-* 入口
---
<!-- AIWS_MANAGED_BEGIN:opencode:using-aiws -->
# using aiws

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 把当前任务先路由到正确的 AIWS workflow，而不是直接跳进实现。

执行建议：
1) 先读取 `AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`。
   - 若检测到 `.opencode/oh-my-opencode.json`：输出 `OpenCode mode: oMo-enabled`，并注明后续会优先借用 `planner-sisyphus` / `explore` / `librarian` / `oracle`
   - 若未检测到：输出 `OpenCode mode: standard-opencode`
2) 若缺失任一真值文件：先 `/ws-preflight`，必要时运行 `aiws init .`，不要继续实现。
3) 若任务意图、归因或验证入口不明确：先提 1-3 个关键澄清问题并停止。
  4) 两层架构：默认走 mattpocock/skills 工程流水线，需要 aiws 治理时叠加 ws-*。
     - 工程入口（mattpocock）：
       - 需求澄清/设计探讨：`/grill-with-docs`（→ `/grilling` + `/domain-modeling`）
       - 输出 spec：`/to-spec`
       - 拆 tickets：`/to-tickets`
       - 实现（含 TDD）：`/implement`
       - 代码审查：`/code-review`
       - 交接：`/handoff`
     - 治理叠加（aiws ws-*）：
       - 设定可审计目标：`/ws-goal`
       - 绑定 change + contract：`/ws-plan`
       - 小步实现（simple/local 单点修复）：`/ws-dev-lite`
       - 完整开发（AIWS 约束下）：`/ws-dev`
       - 前端设计规则：`/ws-frontend-design`
       - aiws 规范/质量双审查：`/ws-spec-review` + `/ws-quality-review`
       - 完成前验证：`/ws-verify-before-complete`
       - 合并/推送/清理：`/ws-finish`
       - 需求/验收/合同变更：`/ws-req-review`
5) 输出 `OpenCode mode:` / `Task intent:` / `Binding:` / `Route:` / `Why:` / `Next:`。
6) 若 `Route: ws-dev` 且任务属于 simple/local 单点修复：`Next` 可显式建议 `/ws-dev-lite`。
7) 除非用户只要路由判断，否则给出 route 后继续遵循对应入口的契约。
<!-- AIWS_MANAGED_END:opencode:using-aiws -->

可在下方追加本项目对 OpenCode 的额外说明（托管块外内容会被保留）。
