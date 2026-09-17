---
description: 目标协议：设定可审计的 goal 目标；依赖链预检；完成审计
---
<!-- AIWS_MANAGED_BEGIN:opencode:ws-goal -->
# ws goal

用中文输出（命令/路径/代码标识符保持原样不翻译）。

## 职责边界

ws-goal 只做三件事，不做更多：

| 做 | 不做 |
|---|---|---|
| 录入目标（写 `.aiws/goals/<id>.md`） | 直接在 main session 创建/驱动 change（通过 pipeline subagent 委托执行） |
| 依赖链预检（上游死 change → 阻断） | 评估复杂度、路由执行路径 |
| 完成审计（claim done 时验证 outcome） | git add/commit/push、finish |
| 记录 target_base_branch 约束 | auto-chain 到下一个 goal（但在同一 goal 内支持多组顺序调度 §6） |

超出以上范围的需求，路由到对应 ws-* 技能，ws-goal 不碰。

## 前置条件

1) 先运行 `/ws-preflight`（对齐 `AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`）。

## 执行流程

0) 检查 `.aiws/goals/` 目录：
   a) 若用户仅查询状态（无明确目标），列出所有 goal 文件及其 status 字段，然后结束。
   b) 若存在 status=active 或 status=paused 的 goal 文件，优先读取并恢复执行，不再新建 goal。

1) 读取真值文件（`AI_PROJECT.md`、`REQUIREMENTS.md`、`AI_WORKSPACE.md`），确认项目规则与边界。

2) 接受用户输入的 goal objective，明确目标范围与验收标准。

3) 按 ws-goal-contract.md 的目标模板生成目标文件，写入 `.aiws/goals/<goal-id>.md`。
   a) 生成时填入 `Target Base Branch` 字段：
      - `target_base_branch`：从当前分支追踪或用户声明确定。默认 `main`
      - `base_branch_mismatch_action`：默认 `block`
   b) 生成时填入初始 `Dependency Chain` 字段：
      - `base_branch`：当前检出分支或用户指定的基分支
      - `chain`：追溯完整的 change 链到 main
      - `chain_verified_at`：当前时间
      - `user_confirmed_unhealthy`：初始为 null

4) **Dependency Chain Validation**：上游依赖链健康检查（不创建任何 change/plan/commit）。
   a) 确定 base_branch：
      - 若当前已存在 change 分支，读取 `.ws-change.json` 或 proposal.md 的 `base_branch`
      - 若用户显式指定 base_branch，以用户指定为准
      - 否则默认 main
   a1) **target_base_branch 一致性检查**（Rule C）：
      - 读取 goal 文件的 `Target Base Branch.target_base_branch`
      - 若 `base_branch != target_base_branch`：
        - 输出 "goal 声明 target_base_branch = X，但当前 base_branch = Y"
        - 若 `base_branch_mismatch_action` 为 `block`（默认）：**强制 blocker**，必须用户修正 base_branch 或显式确认 mismatch 才能继续
        - 若为 `warn`：输出警告并允许继续
        - 将 mismatch 记录到 goal 文件的 Audit Trail
      - 若一致：继续 step b
   b) 追溯 chain：从 base_branch 逐级向上追溯到 main
      - 每级检查 `.ws-change.json` 的 `base_branch` 或 git 分支关系
      - 若无法追溯（孤儿分支），标记 UNKNOWN 并输出警告
   c) 对 chain 中每个 link 做健康检查（按 ws-goal-contract.md 2.4.2 标准）：
      - artifacts 完整性：proposal/tasks/design 是否存在
      - task 完成率：WS:TODO 占比
      - review 状态：是否有 HIGH blocker
      - 活跃度：最后更新时间
      - truth drift：`aiws validate .` 是否通过
   d) 结果：
      - ALL HEALTHY → 输出 "依赖链健康" 并继续
      - 存在 STALE → 输出链拓扑 + 警告，允许继续
      - 存在 UNHEALTHY → 输出完整 chain 拓扑 + 每级健康报告，**强制 blocker**，必须用户显式确认才能继续
      - UNKNOWN（无法追溯）→ 输出警告"无法验证上游依赖链"，不阻断但要求用户确认
   e) 将验证结果写入 goal 文件的 `Dependency Chain` 字段：
       - 更新 `chain` 中每个 link 的健康状态与判定理由
       - 若用户放行 UNHEALTHY：设置 `user_confirmed_unhealthy: true` 并记录到 `Audit Trail`
       - 若用户未放行：设置 goal status=paused 并结束

4.5) **Workspace State Analysis**：依赖链预检通过后、delegation 前，分析工作区状态并输出报告。
     必须用户确认后才能进入 step 5。
     a) 检查 dirty 状态：
        - staged changes（`git diff --cached --stat`）
        - unstaged changes（`git diff --stat`）
        - untracked files（`git status --porcelain` 中 `??` 开头项）
     b) 检查 submodule 状态：
        - 每个 submodule 的 dirty 状态（`git submodule status`）
        - detached HEAD（`git -C <path> symbolic-ref HEAD` 失败）
        - unpushed 提交（`git -C <path> log @{u}..HEAD --oneline`）
     c) 检查 change artifacts：
        - 存在哪些 change 分支（`git branch --list 'change/*'`）
        - 是否有未完成的 change（proposal/tasks 仍含 WS:TODO）
        - 是否与当前 goal 冲突（同名、同域）
     d) 检查 git 状态：
        - 是否有 unpushed 提交（`git log @{u}..HEAD --oneline`）
        - 是否有 stash（`git stash list`）
     e) 评估影响：逐项判断与当前 goal 的关联度：
        - HIGH：阻碍 goal 执行，必须处理
        - MED：可能干扰或产生误报
        - LOW：无影响，仅提示
        - NONE：完全无关，忽略
     f) 输出结构化分析报告：
        用格式化的文本块输出，每行标注影响等级：
        ```
        ═══ 工作区状态报告 ═══
        [HIGH] 子模块 web/ dirty（7 文件）— 与 goal 同一目录，可能干扰
        [MED]  旧 change contract-ai-rag 残留 — 可能干扰依赖链判断
        [LOW]  .aiws/journal/ 日志文件 — 无影响
        ════════════════════════
        ```
     g) 展示报告后要求用户选择：
        - **继续** → 进入 step 5
        - **暂停** → goal status=paused，报告写入 Audit Trail，结束
        - **先清理** → goal status=paused，输出清理建议步骤，结束
     用户未确认前，不得进入 step 5。

5) **Phase-Level Pipeline Delegation**：依赖链预检 + workspace 分析通过后，将 goal 拆分为 PLAN→DEV→REVIEW→FINISH 四个 phase 顺序执行，每个 phase 委托给独立轻量子 agent，主 session 验证每个 phase 产出后决定继续/重试/暂停。
    前置条件：step 4 必须通过（ALL HEALTHY 或 UNHEALTHY 已显式放行）。若 step 4 阻断，不允许 delegation。
    前置条件 2：不存在 status=active 的 change 分支。若有，让用户选择「使用已有 change 继续」或「暂停」。

5a) **Check for Groups**：读取 goal 文件，检查是否定义了 `Groups` 区域。
    - 若 goal **不含** groups → 走单组 phase-level pipeline（step 5b-5g）
    - 若 goal **包含** groups → 走 Sequential Group Phase-Level Dispatch（step 5.1）

--- 以下为 phase-level pipeline（无 groups 的 goal） ---

5b) **Phase-Level Sequential Execution**：主 session 按 PLAN→DEV→REVIEW→FINISH 顺序执行 4 个 phase，每个 phase 前检查当前进度（支持断点续跑）。共用一个 change 分支，change id = goal-id。

5c) **PHASE 1 - PLAN**（主 session 执行 change start + 委托子 agent 做 plan）：
    1. 主 session 执行 `aiws change start <goal-id> --allow-dirty`（若 change 已存在则跳过）
    2. 委托子 agent 执行 PLAN phase：
        ```
        task(
          category="unspecified-high",
          description="PLAN phase for goal <goal-id>",
          prompt="TASK: Write proposal.md + plan file + run plan-verify for goal <goal-id>.
                  INPUT: goal file at .aiws/goals/<goal-id>.md, truth files at AI_PROJECT.md/REQUIREMENTS.md/AI_WORKSPACE.md.
                  CONSTRAINTS: Do NOT implement code. Do NOT do review. Do NOT commit.
                  COMPLETION: proposal.md exists, plan file exists, plan-verify passes."
        )
        ```
    3. 主 session 验证产出：
        - proposal.md 文件存在
        - plan 文件存在
        - plan-verify 通过
    4. 验证通过 → 进入 PHASE 2
    5. 验证失败 → 可重试最多 2 次 → 仍失败则 goal state=paused，记录 blocker

5d) **PHASE 2 - DEV**（委托子 agent 做 dev）：
    1. 委托子 agent 执行 DEV phase：
        ```
        task(
          category="deep",
          description="DEV phase for goal <goal-id>",
          prompt="TASK: Implement all code changes per plan for goal <goal-id>.
                  INPUT: proposal.md at .aiws/changes/<goal-id>/proposal.md, plan file.
                  CONSTRAINTS: Do NOT modify proposal or plan files. Do NOT commit.
                  COMPLETION: All changes implemented, lint/typecheck clean."
        )
        ```
    2. 主 session 验证产出：
        - diagnostics 干净（`lsp_diagnostics` 检查改动文件）
        - 改动范围与 plan 一致
    3. 验证通过 → 进入 PHASE 3
    4. 验证失败 → goal state=paused，记录 blocker

5e) **PHASE 3 - REVIEW**（委托子 agent 做 review）：
    1. 委托子 agent 执行 REVIEW phase：
        ```
        task(
          category="unspecified-high",
          load_skills=["review-work"],
          description="REVIEW phase for goal <goal-id>",
          prompt="TASK: Audit code changes for goal <goal-id>, produce review evidence.
                  INPUT: proposal.md, plan file, changed files.
                  CONSTRAINTS: Do NOT modify code. Do NOT commit.
                  COMPLETION: Review evidence files exist, no HIGH blockers."
        )
        ```
    2. 主 session 验证产出：
        - review 证据文件存在
        - 无未解决的 HIGH blocker
    3. 验证通过 → 进入 PHASE 4
    4. 验证失败 → goal state=paused，记录 blocker

5f) **PHASE 4 - FINISH**（委托子 agent 做 commit + finish）：
    1. 委托子 agent 执行 FINISH phase：
        ```
        task(
          category="quick",
          load_skills=["git-master"],
          description="FINISH phase for goal <goal-id>",
          prompt="TASK: Commit and finish (merge + push) for goal <goal-id> change branch.
                  CONSTRAINTS: Do NOT modify code. Git operations only.
                  COMPLETION: Change branch merged to target_base_branch and pushed."
        )
        ```
    2. 主 session 验证产出：
        - 确认 change 分支已合并到 target_base_branch
        - 确认已推送
    3. 验证通过 → goal state=complete，输出 "Goal <goal-id> complete"
    4. 验证失败 → goal state=paused，记录 blocker

5g) **Simple Goal Escape Hatch**：若 goal 为简单改动（≤3 文件，配置/doc/规范变更，无架构风险）：
    - 可跳过 PHASE 3（REVIEW），在 PHASE 2 验证后直接进入 PHASE 4
    - 必须在 goal 文件的 Progress Notes 标注 "skipped review phase (simple goal)"

--- 以下为 Sequential Group Phase-Level Dispatch（含 groups 的 goal） ---

5.1) **Sequential Group Phase-Level Dispatch**：
     当 goal 文件包含 Groups 定义时，ws-goal 按依赖顺序逐个调度每个 group，每个 group 内按 5c-5f 的 4-phase 流程执行。主 session 负责编排每个 group 的 4 个 phase 并验证每个 phase 的产出。

5.1a) **解析并验证 groups**：
     - 提取 goal 文件中所有 group 定义（id, scope, verification, depends_on, status）
     - 验证 DAG：无循环依赖，depends_on 引用正确的已知 group
     - 将所有 group status 初始化为 `pending`
     - 若解析失败（格式错误、循环依赖）→ 输出错误，不允许 delegation

5.1b) **计算拓扑顺序**：
     - 按 depends_on 确定执行顺序。默认：声明顺序
     - 输出 group 执行计划列表：
       ```
       ═══ Group 执行计划 ═══
       [1] group-1: Foundation Pages（depends_on: none）
       [2] group-2: Purchase Conversion（depends_on: group-1）
       [3] group-3: Account & Orders（depends_on: group-1）
       ═══════════════════════════
       ```
     - 展示计划后要求用户确认是否继续。用户确认后才开始调度

5.1c) **按顺序执行每个 group（phase-level）**：
     FOR each group in 拓扑顺序：
       1. 更新 group status → `in_progress`，写入 goal 文件 Progress Notes
       2. **PHASE 1 - PLAN**（同 5c，但 change id = `<goal-id>-<group-id>`）：
          - 主 session 执行 `aiws change start <goal-id>-<group-id> --allow-dirty`
          - 委托子 agent 写 proposal.md + plan 文件 + plan-verify
          - 主 session 验证：proposal/plan 存在，plan-verify 通过
       3. **PHASE 2 - DEV**（同 5d，scope 限定到当前 group）：
          - 委托子 agent 按 plan 实现当前 group 的代码改动
          - 主 session 验证：diagnostics 干净，改动匹配 group scope
       4. **PHASE 3 - REVIEW**（同 5e，scope 限定到当前 group）：
          - 委托子 agent 审计当前 group 改动
          - 主 session 验证：review 证据存在，无 HIGH blocker
       5. **PHASE 4 - FINISH**（同 5f）：
          - 委托子 agent 做 commit + finish
          - 主 session 验证：分支已合并推送
       6. 任一 phase 失败 → group status = paused/failed，记录 blocker，**STOP**（后续 group 不再调度）
       7. 全部 phase 通过 → 更新 group status → `complete`，更新 Progress Notes
          - 若 goal 配置了多组并行：可在 PLANNING 通过且 DAG 无冲突前提下的顺序执行

5.1d) **所有 group 完成后**：
     - 运行整体完成审计：
       - goal 级别的 outcome 是否满足
       - 所有 group 均为 complete（无 paused/failed）
       - 全局 lint/type 检查无新增错误
       - 输出每 group 结果 + 整体结论
     - 全部通过 → 更新 goal state=complete，输出 "Goal <id> complete（<N> groups executed）"
     - 有未通过 → 更新 goal state=paused

5.1e) **恢复机制**（下一 session 进入 step 0 时触发）：
     - 检测到 paused goal 且含 groups → 输出恢复选项：
       - （a）重试失败的 group（从失败的 phase 重新开始）
       - （b）跳过该 group（标记 complete，继续下游）
       - （c）暂停并 handoff
     - 用户选择后执行对应操作
<!-- AIWS_MANAGED_END:opencode:ws-goal -->

可在下方追加本项目对 OpenCode 的额外说明（托管块外内容会被保留）。
