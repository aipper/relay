---
name: ws-plan
description: 使用时机：需要生成执行计划、建立 change 绑定时。触发词：计划、规划、方案、设计、change。注意：需求未冻结先用 ws-intake。
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 对齐真值文件（`AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`）
- 若尚未进入本次 change 的工作上下文：先建立 `change/<change-id>` 分支 / worktree，再生成计划
- 为当前任务生成一份可追踪的执行计划文件：`plan/<timestamp>-<slug>.md`
- 计划必须包含可复现验证命令（优先引用 `AI_WORKSPACE.md`）
- 计划必须包含“主索引绑定”：`Change_ID` / (`Req_ID` or `Problem_ID`) / `Contract_Row` / `Plan_File` / `Evidence_Path`

OpenCode + oMo 优先策略：
- 有 oMo 时按 `packages/spec/docs/opencode-omo-adapter.md` 优先委托 `planner-sisyphus` / `@explore` / `@librarian`；主 agent 回收结果并落盘 `plan/...`（调用 agent ≠ 完成计划）。

约束：
- 不写入任何 secrets（token、账号、内网端点等不得进入 git）
- 本 skill 只负责“想清楚怎么做 + 落盘计划”，不要直接大规模改动代码
- 未运行不声称已运行；验证命令要写清“预期结果”
- 若存在 `.aiws/changes/<change-id>/proposal.md`，计划与 proposal 的绑定字段必须保持一致（不一致时先修正再继续）

阶段定位：
- planning 阶段；负责把用户目标收敛为 change 绑定、计划文件和验证入口。

必需输入：
- 当前任务描述
- 真值文件：`AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`
- 若已存在：`.aiws/changes/<change-id>/proposal.md`
- 若已有计划：当前 `plan/...` 文件

必需输出：
- `Plan file:` / `Change context:` / `Bindings:` / `Verify:` / `Next:`（先 `$ws-plan-verify`，通过后再 `$ws-dev`）

阻断条件：
- 任务目标或归因绑定不清晰
- 当前工作区 dirty 且尚未进入可复用的 change 上下文
- 无法把计划实际写盘

完成判定：
- 计划已落盘、绑定已同步、验证入口明确，后续实现可以直接按计划推进。

执行步骤（建议）：
1) 先运行 `$ws-preflight`（读取真值文件并输出约束摘要）。
   - 若检测到 oMo：优先让 `planner-sisyphus` 生成 planning draft；若需要补结构探索，再委托 `@explore` / `@librarian`。
2) 若用户任务描述不清：先问 1-3 个关键澄清问题（不要猜）。
3) 判断复杂度：`simple / medium / complex`（给出一句理由），并估算步骤数。同时判定 `Change_Type`（pure-ui / frontend-logic / backend-api / full-stack / config-docs），后续 `Verify` 命令必须与类型匹配。Granularity gate：每步必须 ≤3 个原子操作（read/edit/write/run）。若某步超过此限，拆细后再写入计划。
4) 识别或建立主索引 / change 上下文：
   - 若存在 `.aiws/changes/<change-id>/proposal.md`：读取其中 `Change_ID` / `Req_ID` / `Problem_ID` / `Contract_Row` / `Evidence_Path`
   - 若缺失关键绑定：先补齐 proposal（至少 `Change_ID`、`Req_ID|Problem_ID`、`Contract_Row`）再继续生成计划
   - 若当前不在 `change/<change-id>` 分支 / worktree，且本次任务需要新建 change：
     ```bash
     bash .opencode/scripts/ws-plan-setup-worktree.sh <change-id>
     ```
     脚本自动处理：dirty 检查、commits/submodules 检测、worktree 创建。
   - 若脚本创建了 worktree：后续所有读取/写入都必须切到 `aiws change start` 输出的 `worktree:` 路径中进行；不要把 `plan/...` 写回原工作区
5) 生成计划文件：
   - 文件名：`plan/YYYY-MM-DD_HH-MM-SS-<slug>.md`（`<slug>` 用 kebab-case；同一任务调整计划时尽量复用同一文件）
   - 若 `plan/` 不存在先创建
   - 必须实际写入到磁盘（不要只在对话里输出）；如因权限/策略无法写盘，必须明确说明原因并输出可复制的完整内容
   - 计划必须写在当前 active change 上下文内：若当前已进入 `change/<change-id>` worktree，则 `plan/...`、`proposal.md`、`tasks.md` 都应写在该 worktree 中
6) 计划内容至少包含（不要留空）：
- `Bindings`：`Change_ID` / `Req_ID` / `Problem_ID` / `Contract_Row` / `Plan_File` / `Evidence_Path` / `Change_Type`
- `Goal`：要达成什么
   - `Non-goals`：明确不做什么（避免 scope creep）
   - `Scope`：将改动的文件/目录清单（不确定就写 `TBD` 并说明如何确定）
   - `Plan`：分步执行（每步尽量落到具体文件/命令；必要时拆 Phase）。每步必须 ≤3 个原子操作；若某步超限，拆成多步。
   - `Submodules`（当存在 `.gitmodules` 且声明了 submodule 条目时，强制）：声明“本次 change 的 submodule 目标分支真值”（用于同一 superproject 分支内的多渠道交付；也避免仅靠 `.gitmodules` 默认分支导致交付推送到错误分支）
   - `Verify`：可复现命令 + 期望结果（优先引用 `AI_WORKSPACE.md` 的入口；必要时补充 e2e）
   - `Risks & Rollback`：风险点 + 回滚方案（例如 git 回滚、`aiws rollback`、恢复备份等）
   - 若 intake 草案包含 `Error States` 或 `Rollback Plan`：必须显式引用并纳入本计划的 `Risks & Rollback` 中，不能丢弃 intake 已识别的问题
   - `Evidence`：计划文件路径；若创建了变更工件则附 `.aiws/changes/<change-id>/...`
7) 若存在 change proposal：回填并对齐 `proposal.md` 的 `Plan_File`（必要时同步 `Contract_Row` / `Evidence_Path`），保证 plan/proposal 一致。
8) 运行 `$ws-plan-verify` 作为执行前质量门（计划不过长、不跑偏、验证可复现）。
   - 通过后：标记 `[workflow-state:plan:DONE]` 或 `[workflow-state:gate:plan_passed]`，表示 plan 阶段已收敛。
8a) **Optional Spec Phase**：若用户需要正式 spec 文档（PRD），在 plan 文件生成后追加一份 `docs/spec/<feature>.md`，模板如下（来自 to-spec skill）：
    ```markdown
    ## Problem Statement
    用户视角的问题描述。
    ## User Stories
    As an <actor>, I want a <feature>, so that <benefit>（编号列表，全面覆盖）
    ## Implementation Decisions
    模块/接口/架构/API 合同（不含具体文件路径，避免过期）
    ## Testing Decisions
    好测试的定义（仅测试外部行为，非实现细节）+ 需要测试的模块 + 代码库中同类测试的先例
    ## Out of Scope
    明确不做的范围
    ```
    可选流程：sketch seams（代码库中已有的 seam 优先于新 seam）→ 与用户确认 seam 匹配 → 写 spec → 落盘。
8b) **Task Granularity Gate（T3）**：在 plan→dev 交接前，逐条验证计划中每步 ≤3 个原子操作（read/edit/write/run）：
    - 若 any 步超限：拆细后再重新通过 ws-plan-verify，**不** 允许带着粗粒度任务进入 ws-dev
    - 仅当全部通过后才可标记 plan 完成并进入 ws-dev
    - 此门禁同样适用于 goal pipeline 的 Granularity Gate（c-i）—— ws-plan 的产出必须能通过下游检查
9) 若计划涉及“需求/验收”变更：先用 `$ws-req-review` 评审 → 用户确认后再 `$ws-req-change` 落盘（避免需求漂移）。
10) 多步任务（≥2 步）：后续进入实现时，使用 `update_plan` 工具跟踪 `pending → in_progress → completed`。
11) 运行 `aiws memory write decision://<change-id>/plan` 写入计划决策（scope、approach、风险摘要）。

oMo 回退：
- 若当前没有 oMo、没有 `planner-sisyphus`，或你无法稳定调用相关 agent：直接回退为普通 OpenCode `plan` / 当前 agent 本地规划。
- 回退不改变 AIWS 的要求：`plan/...` 仍必须落盘，bindings / verify / risks / evidence 仍必须完整。

补充：submodule 目标分支真值
- 规范见 `packages/spec/docs/ws-goal-contract.md` §delivery-targets；落盘 `.aiws/changes/<change-id>/submodules.targets`（可用 `bash .opencode/scripts/ws-plan-gen-submodule-targets.sh <change-id>`）。生成的 branch 是**裸分支名**（如 `main`），**不是** `aiws/pin/main`——`aiws/pin/` 前缀由 `change finish` 自动拼接。有 `.gitmodules` 时门禁强制要求覆盖所有 submodule path；计划须写清 targets，后续 `$ws-dev` 前挂到 `aiws/pin/<target_branch>`。
- ⚠️ **不要手动在 `submodules.targets` 的 branch 字段加 `aiws/pin/` 前缀**——`change finish` 已自动处理，加前缀会导致双拼（如 `aiws/pin/aiws/pin/main`）。

输出要求：
- `Plan file:` <实际写入的路径>
- `Change context:` <当前 change 分支或 worktree 路径；若新建了 worktree 需明确写出>
- `Next:` 推荐下一步（先 `$ws-plan-verify`，通过后再 `$ws-dev`；或 `aiws change start <change-id> --hooks`，superproject + submodule 可用 `--switch`）

> 运行时行为约束：`packages/spec/docs/run-behavior-guidelines.md`
