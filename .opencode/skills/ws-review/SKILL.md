---
name: ws-review
description: 使用时机：需要审计当前改动、查找风险时。触发词：审计、评审、review、风险检查、回归检查。注意：高风险变更应补 ws-spec-review + ws-quality-review。
---

## 双审查边界

- `ws-review` 是通用评审入口
- **高风险或准备 finish 的变更**：必须拆为 `ws-spec-review`（流程/归因/真值完整性）+ `ws-quality-review`（行为回归/边界条件/测试覆盖），两份独立证据
- 单份 review 文件同时覆盖 spec 和 quality 不计为双审查

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：在提交/交付前审计当前改动，对照真值文件检查是否越界，并把审计证据优先落盘到 `.aiws/changes/<change-id>/review/`（若无法确定 `change-id` 再回退 `.aiws/tmp/review/`）。
若当前语境已经明确是“准备交付/finish”，则本入口不应只停留在通用 review：应继续同时补齐 `$ws-spec-review` 与 `$ws-quality-review`，把 dual review gate 一次性收敛完。

OpenCode + oMo 优先策略：
- 若检测到 `.opencode/oh-my-opencode.json`，或当前会话明确可用 `oracle` / `explore` / `librarian`，优先借用这些 agent 做 review。
- `@oracle` 优先负责独立审查与 findings；`@explore` 用于补 diff 影响面；`@librarian` 用于补 requirements / docs / 依赖上下文。
- 主 agent 必须负责把 findings 收敛并落盘，不要把子 agent 输出直接当最终 review 结论。

阶段定位：
- review 阶段；负责对当前改动做规范、风险和验证完整性的审计。
- 双审查边界：ws-spec-review 查流程/真值归因，ws-quality-review 查行为/回归/测试。高风险或准备 finish 的任务必须同时完成两者。

必需输入：
- 当前 `git status` / `git diff`
- 已执行的验证结果
- 真值文件：`AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`
- 当前 `change/<change-id>` 上下文（若能识别）
- 若存在：`.aiws/changes/<change-id>/analysis/`、`patches/`、已有 `review/` 文件

必需输出：
- 审计文件：`.aiws/changes/<change-id>/review/codex-review.md` 或回退 `.aiws/tmp/review/codex-review.md`
- `主要风险（Top risks）:` 3-8 条
- `下一步（Next）:` 最小修复清单 + 最小验证命令

阻断条件：
- 没有可审计的改动或验证上下文
- 审计证据无法写盘

完成判定：
- 审计证据已落盘，主要风险和下一步已明确，可作为 commit/deliver 前置输入。

步骤（建议）：
1) 先做 preflight：定位项目根目录，读取 `AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`，输出约束摘要。
   - 若检测到 oMo：优先让 `@oracle` 做独立审查；必要时再让 `@explore` / `@librarian` 补上下文。
2) **Change Scope Assessment**：在深入审查前，先获取变更上下文。
   - 执行 `git diff --stat HEAD` 查看变更文件及行数
   - 执行 `git log --oneline -3` 查看最近提交
   - 使用此上下文将审查聚焦在变更区域，各 reviewer agent 无需独立发现变更范围
3) **Triage**：preflight 后立即判断本 change 是否需要双审查：
   - 需要双审查的条件：改动涉及 REQUIREMENTS 真值、跨 3+ 文件、涉及安全/数据一致性、或准备 finish
   - 若需要双审查：在步骤 4 开始时即标注 "dual-review: required"，并规划 spec-review + quality-review 两条并行路径
     - 若不需要双审查：继续通用 review 流程，标注 "dual-review: not-required"
     - **不得等到 step 7 才决定是否需要双审查**——风险判定必须前置
    - **Triage 输出格式**：
      ```
      Triage: dual-review: required | not-required
      Rationale: <one reason>
      Spec review scope: <what to check> (if required)
      Quality review scope: <what to check> (if required)
      ```
    - Findings 格式要求：每个 finding 必须有 [Critical/Warning/Info] 级别标签 + 归因到 SPEC/QUALITY/REGRESSION 类别
3a) **Two-Axis Review Methodology**（可选深化）。对于需要深入审查的变更，拆为两个正交维度并行审查（参考自 `code-review` skill）：
    - **Standards** — 代码是否符合代码库的编码标准？携带下面的 smell baseline 作为默认基准。
    - **Spec** — 代码是否忠实地实现了源需求/spec？
    - 两轴并行运行，避免一个轴掩盖另一个。"符合所有标准但实现了错误需求" → Standards pass, Spec fail；"需求实现了但破坏了项目规范" → Spec pass, Standards fail。

3b) **Code Smell Baseline**。Standards 轴在项目自有标准之外，始终携带以下 Fowler 代码坏味作为默认基准（项目标准高于基线）。每个坏味是启发式标签，不是硬性违规——跳过已由工具强制的内容。

    | Smell | 识别信号 | 修复方向 |
    |---|---|---|
    | Mysterious Name | 名称不揭示作用 | 重命名；若找不到诚实名称，设计有问题 |
    | Duplicated Code | 相同逻辑形状出现在多处 | 提取共享形状 |
    | Feature Envy | 方法存取其他对象数据多于自身 | 把方法移到它嫉妒的数据上 |
    | Data Clumps | 相同字段/参数反复出现 | 打包为一个类型 |
    | Primitive Obsession | 原始类型代替领域概念 | 给概念自己的类型 |
    | Repeated Switches | 对同类型的 switch/if 级联重复 | 多态或共享 map |
    | Shotgun Surgery | 一个逻辑变更导致多处分散修改 | 聚合到一个模块 |
    | Divergent Change | 一个文件因多个不相关原因被改 | 拆分，每个模块只为一个原因变 |
    | Speculative Generality | 加了 spec 不需要的抽象 | 删除，直到真有需求 |
    | Message Chains | 长链式导航 | 隐藏在第一个对象的方法后 |
    | Middle Man | 类/函数主要只是委托转发 | 直接调用真正的目标 |
    | Refused Bequest | 子类忽略大部分继承内容 | 组合替代继承 |

4) 基于 `git status` / `git diff`（以及你实际运行过的测试结果），对照 `AI_PROJECT.md` 与 `REQUIREMENTS.md` 检查：
   - 是否存在越界目录改动/危险操作
   - 是否有可复现验证命令与证据
   - 是否维护了 `.aiws/changes/<change-id>/` 或相关 `issues/*.csv`
   - 若存在 `analysis/` / `patches/`：审查这些委托工件是否已被主 agent 理解、是否需要采用/拒绝，并把结论写入 review 文件
5) Workflow State Suffix 审计（检查 4 种后缀使用是否一致）：
   - `session` 后缀：只由 ws-dev-lite / ws-intake 写入，标记会话级进度（如 `[workflow-state:session:in_progress]`）
   - `gate` 后缀：由 ws-dev / ws-plan-verify 写入，标记计划/实现门禁结果（如 `[workflow-state:gate:plan_passed]`）
   - `plan` 后缀：由 ws-plan 写入，标记计划阶段状态（如 `[workflow-state:plan:in_progress]`）
   - `gateway` 后缀：由 ws-finish / ws-deliver 写入，标记交付门禁结果（如 `[workflow-state:gateway:finish_gate_ok]`）
   - 检查当前 change 中使用的后缀类型是否正确对应所在阶段；若出现混用（如 session 与 gate 在同一文件），在审计报告中标记异常并说明应该修正的方向。
6) 将审计落盘到（目录不存在则创建）：
   - 默认：`.aiws/changes/<change-id>/review/codex-review.md`
   - 回退：`.aiws/tmp/review/codex-review.md`（仅在无法确定 `change-id` 时使用）
   - 若已有其它 reviewer 文件：不要覆盖它们；当前 reviewer 应写自己的文件或更新自己的汇总文件
7) 若 triage 标记为 `dual_review_required`，继续补齐 dual review gate：
   - 运行/收敛 `$ws-spec-review`，落盘 `.aiws/changes/<change-id>/review/spec-review.md`（或回退 `.aiws/tmp/review/spec-review.md`）
   - 运行/收敛 `$ws-quality-review`，落盘 `.aiws/changes/<change-id>/review/quality-review.md`（或回退 `.aiws/tmp/review/quality-review.md`）
    - 不要把单个 `codex-review.md` 误当成 finish gate 已完成
8) 运行 `aiws memory write decision://<change-id>/review` 写入审查发现的约束/决策。
9) 回复中输出：
   - `证据（Evidence）:` 证据文件路径
   - `主要风险（Top risks）:` 3–8 条（高→低）
   - `下一步（Next）:` 最小修复清单 + 最小验证命令

安全：
- 不打印 secrets。
- 不执行破坏性命令。
- 若 oMo agent 不可用，回退为当前 agent 本地 review，不阻断流程。

> 运行时行为约束：`packages/spec/docs/run-behavior-guidelines.md`
