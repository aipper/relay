---
name: ws-intake
description: 计划前置澄清（逐条冻结问题并产出 intake 草案，供 ws-plan 消费）
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 在进入 `/ws-plan` 前，把新需求或中大型变更里的待确认问题逐条澄清并冻结。
- 采用“一题一线程”模式推进：每次只处理 1 个问题，允许该问题多轮往返，直到形成明确结论。
- 产出一份可被 `/ws-plan` 消费的轻量草案：`plan/<timestamp>-<slug>.intake.md`。

执行要求：
1) 先读 `AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`，必要时先 `/ws-preflight`。
2) 若存在最新 `plan/*.intake.md`，先续写它；否则新建一份 intake 草案。
3) 把当前任务拆成 `Open Questions`，状态只允许 `open / in_discussion / frozen / deferred`。
4) 每次只推进 1 个当前问题，并显式输出：
   - `Current question:`
   - `Why it matters:`
   - `Current options / current understanding:`
   - `Exit condition:`
5) 当前问题在没有被标记成 `frozen` 或 `deferred` 前，不进入下一题。
6) 每轮都要把 intake 草案写盘，至少包含：
   - `Context`
   - `Open Questions`
   - `Resolved Questions`
   - `Frozen Decisions`
   - `Draft Scope`
   - `Draft Verify`
   - `Ready for ws-plan: yes/no`
7) 若关键问题已冻结：`Next` 指向 `/ws-plan`；否则继续 `/ws-intake`。
