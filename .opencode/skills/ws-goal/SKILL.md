---
name: ws-goal
description: 目标协议：设定可审计的 goal 目标；依赖链预检；管道委托；完成审计
---
# ws-goal

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 将用户需求转化为可审计的 goal 目标，按 ws-goal-contract.md 的目标模板写入目标文件。
- 依赖链预检：阻断"死 change 阻塞下游"这类 chain 问题。
- 管道委托：预检通过后将 goal 拆分为 phase-level 子任务（PLAN→DEV→REVIEW→FINISH），每个 phase 委托给独立轻量子 agent，主 session 编排调度与验证。
- 完成审计：claim done 时验证 outcome 真伪。

ws-goal 不做的事：
- 不直接在 main session 操作 change 或写代码（通过 pipeline subagent 委托执行）
- 不替换 review/commit/finish 门禁
- 不 auto-chain 到下一个 goal（但在同一 goal 内支持多组顺序调度 §6）

前置条件：
1) 先运行 `/ws-preflight`（对齐 `AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`）。

执行流程：
0) 检查 `.aiws/goals/` 目录：
   a) 若用户仅查询状态（无明确目标），列出所有 goal 文件及其 status 字段，然后结束。
   b) 若存在 status=active 或 status=paused 的 goal 文件，优先读取并恢复执行，不再新建 goal。
1) 读取真值文件（`AI_PROJECT.md`、`REQUIREMENTS.md`、`AI_WORKSPACE.md`），确认项目规则与边界。
2) 接受用户输入的 goal objective，明确目标范围与验收标准。
3) 按 ws-goal-contract.md 的目标模板生成目标文件，写入 `.aiws/goals/<goal-id>.md`。
4) 输出 completion audit checklist，列出每个 goal 的完成标准与验证方式。
5) **Workspace State Analysis**：分析 dirty/submodule/change artifacts/git 状态，输出影响等级报告，用户确认后才能继续。参考 `/ws-goal` command 的 step 4.5 完整流程。
6) **Phase-Level Pipeline Delegation**：预检 + 分析通过后，将 goal 拆分为 PLAN→DEV→REVIEW→FINISH 四个 phase 顺序执行，每个 phase 委托给独立轻量子 agent，主 session 验证每个 phase 产出后决定继续/重试/暂停。参考 `/ws-goal` command 的 step 5 完整流程。
