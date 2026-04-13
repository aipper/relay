---
name: ws-intake
description: 计划前置澄清（逐条冻结问题并产出 intake 草案，供 ws-plan 消费）
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 在进入 `$ws-plan` 前，把新需求或中大型变更里的待确认问题逐条澄清并冻结
- 采用“一题一线程”模式推进：每次只处理 1 个问题，允许该问题多轮往返，直到形成明确结论
- 产出一份可被 `$ws-plan` 消费的轻量草案：`plan/<timestamp>-<slug>.intake.md`

约束：
- 不直接创建 change，不直接落正式 `plan/...`，不直接进入实现
- 同一时刻只能有 1 条问题处于 `in_discussion`
- 当前问题没有被标记为 `frozen` 或 `deferred` 前，不进入下一题
- 新衍生问题只加入 `Open Questions` 队列，不抢占当前问题

阶段定位：
- pre-planning 阶段；负责把“多轮沟通”从正式计划阶段剥离出来，先冻结需求结论。

必需输入：
- 当前任务描述
- 真值文件：`AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`
- 若已存在：最新 `plan/*.intake.md`

必需输出：
- `Intake file:` 实际写入的 `plan/*.intake.md`
- `Current question:` 当前正在处理的问题
- `Open Questions:` 尚未冻结的问题列表
- `Frozen Decisions:` 已冻结结论
- `Ready for ws-plan:` yes/no
- `Next:` 继续 `$ws-intake` 或进入 `$ws-plan`

阻断条件：
- 无法确定项目根或真值文件缺失
- 当前问题无法被表达为具体、可冻结的决策
- 无法把澄清结果实际写盘

完成判定：
- 已有一份最新 intake 草案，且当前轮次至少把 1 条问题推进到 `frozen` 或 `deferred`
- 当 `Open Questions` 已清空，或剩余问题明确被标记为 `deferred`，输出 `Ready for ws-plan: yes`

执行步骤（建议）：
1) 先运行 `$ws-preflight`，读取真值文件并输出约束摘要。
2) 读取当前任务描述；若存在最新 `plan/*.intake.md`，先读取其中的：
   - `Open Questions`
   - `Resolved Questions`
   - `Frozen Decisions`
   - `Draft Scope`
   - `Draft Verify`
3) 初始化或续写问题队列：
   - 把当前任务拆成 `N` 条待决问题
   - 每条问题都要写成一句明确的决策句，而不是模糊话题
   - 状态只允许：`open` / `in_discussion` / `frozen` / `deferred`
4) 选择唯一的当前问题：
   - 优先取已有 `in_discussion`
   - 否则取最影响 `Goal / Scope / Verify / Binding` 的 `open`
5) 对当前问题输出并沟通：
   - `Current question:`
   - `Why it matters:`
   - `Current options / current understanding:`
   - `Exit condition:` 这条问题在什么条件下算谈完
6) 只推进当前问题：
   - 允许围绕这 1 条问题多轮问答
   - 若用户回答引出新问题：加入 `Open Questions`
   - 若当前问题已经有明确结论：标记为 `frozen`
   - 若当前问题故意留到后面：标记为 `deferred`
7) 每次落盘 `plan/<timestamp>-<slug>.intake.md`，至少包含：
   - `Context`
   - `Open Questions`
   - `Resolved Questions`
   - `Frozen Decisions`
   - `Draft Scope`
   - `Draft Verify`
   - `Ready for ws-plan: yes/no`
8) 判断是否可以移交给 `$ws-plan`：
   - 若仍有关键问题未冻结：`Ready for ws-plan: no`，`Next: 继续 $ws-intake`
   - 若关键问题已冻结，剩余仅是实现细节或已标记 `deferred`：`Ready for ws-plan: yes`，`Next: $ws-plan`

输出要求：
- `Intake file:` <实际写入的路径>
- `Current question:` <当前问题>
- `Open Questions:` <剩余问题列表>
- `Frozen Decisions:` <已冻结结论>
- `Ready for ws-plan:` <yes/no>
- `Next:` <继续 `$ws-intake` 或进入 `$ws-plan`>
