---
name: ws-plan
description: 规划（生成可落盘 plan/ 工件；供 ws-dev 执行）
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 对齐真值文件（`AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`）
- 为当前任务生成一份可追踪的执行计划文件：`plan/<timestamp>-<slug>.md`
- 计划必须包含可复现验证命令（优先引用 `AI_WORKSPACE.md`）

约束：
- 不写入任何 secrets（token、账号、内网端点等不得进入 git）
- 本 skill 只负责“想清楚怎么做 + 落盘计划”，不要直接大规模改动代码
- 未运行不声称已运行；验证命令要写清“预期结果”

执行步骤（建议）：
1) 先运行 `$ws-preflight`（读取真值文件并输出约束摘要）。
2) 若用户任务描述不清：先问 1-3 个关键澄清问题（不要猜）。
3) 判断复杂度：`simple / medium / complex`（给出一句理由），并估算步骤数。
4) 生成计划文件：
   - 文件名：`plan/YYYY-MM-DD_HH-MM-SS-<slug>.md`（`<slug>` 用 kebab-case；同一任务调整计划时尽量复用同一文件）
   - 若 `plan/` 不存在先创建
   - 必须实际写入到磁盘（不要只在对话里输出）；如因权限/策略无法写盘，必须明确说明原因并输出可复制的完整内容
5) 计划内容至少包含（不要留空）：
   - `Goal`：要达成什么
   - `Non-goals`：明确不做什么（避免 scope creep）
   - `Scope`：将改动的文件/目录清单（不确定就写 `TBD` 并说明如何确定）
   - `Plan`：分步执行（每步尽量落到具体文件/命令；必要时拆 Phase）
   - `Verify`：可复现命令 + 期望结果（优先引用 `AI_WORKSPACE.md` 的入口；必要时补充 e2e）
   - `Risks & Rollback`：风险点 + 回滚方案（例如 git 回滚、`aiws rollback`、恢复备份等）
   - `Evidence`：计划文件路径；若创建了变更工件则附 `changes/<change-id>/...`
6) 若计划涉及“需求/验收”变更：先用 `$ws-req-review` 评审 → 用户确认后再 `$ws-req-change` 落盘（避免需求漂移）。
7) 多步任务（≥2 步）：后续进入实现时，使用 `update_plan` 工具跟踪 `pending → in_progress → completed`。

输出要求：
- `Plan file:` <实际写入的路径>
- `Next:` 推荐下一步（通常是 `$ws-dev` 或 `aiws change start <change-id> --hooks`；superproject + submodule 可用 `--worktree`）
