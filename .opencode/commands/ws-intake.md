---
description: 计划前置澄清：逐条冻结问题并写入 intake 草案，再交给 ws-plan
---
<!-- AIWS_MANAGED_BEGIN:opencode:ws-intake -->
# ws intake

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 在正式 `/ws-plan` 前，先把需求里的待确认问题逐条澄清并冻结。

执行建议：
1) 先读取 `AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`。
2) 若存在最新 `plan/*.intake.md`，先续写；否则创建新的 intake 草案。
3) 把问题写成 `Open Questions`，状态只允许 `open / in_discussion / frozen / deferred`。
4) 每次只推进 1 条 `Current question`，允许围绕这 1 条问题多轮沟通。
5) 当前问题未标记为 `frozen` 或 `deferred` 前，不进入下一题。
6) 每轮都要把结论落到 `plan/<timestamp>-<slug>.intake.md`，并输出 `Ready for ws-plan: yes/no`。
7) 只有关键问题已冻结时，`Next` 才能进入 `/ws-plan`；否则继续 `/ws-intake`。
<!-- AIWS_MANAGED_END:opencode:ws-intake -->

可在下方追加本项目对 OpenCode 的额外说明（托管块外内容会被保留）。
