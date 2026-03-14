---
description: 规划：生成可落盘 plan 工件
---
<!-- AIWS_MANAGED_BEGIN:opencode:ws-plan -->
# ws plan

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 生成可落盘执行计划（供 /ws-dev 执行）。

执行建议：
1) 先运行 `/ws-preflight`（对齐 `AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`）。
2) 生成或更新计划文件：`plan/YYYY-MM-DD_HH-MM-SS-<slug>.md`。
3) 计划至少包含：`Bindings`、`Goal`、`Non-goals`、`Scope`、`Plan`、`Verify`、`Risks & Rollback`、`Evidence`。
4) 若已有 `changes/<change-id>/proposal.md`，对齐 `Plan_File` / `Contract_Row` / `Evidence_Path`。
5) 完成后先运行 `/ws-plan-verify`，通过再进入 `/ws-dev`。
<!-- AIWS_MANAGED_END:opencode:ws-plan -->

可在下方追加本项目对 OpenCode 的额外说明（托管块外内容会被保留）。
