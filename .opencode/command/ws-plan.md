---
description: 规划：生成可落盘 plan 工件
---
<!-- AIWS_MANAGED_BEGIN:opencode:ws-plan -->
# ws plan

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 若尚未进入本次 change 的工作上下文：先建立 `change/<change-id>` 分支 / worktree，再生成可落盘执行计划（供 /ws-dev 执行）。

执行建议：
1) 先运行 `/ws-preflight`（对齐 `AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`）。
2) 若当前不在 `change/<change-id>` 分支 / worktree，先调用 `aiws change start <change-id>` 建立上下文：
   - 仓库已有提交：优先 `aiws change start <change-id> --hooks --worktree`；若声明了 submodules，加 `--submodules`
   - 仓库尚无提交 / 不满足 worktree 前置条件：回退 `aiws change start <change-id> --hooks --no-switch`
3) 若上一步创建了 worktree：切到输出的 `worktree:` 路径，后续所有计划文件都写在该 worktree 中。
4) 生成或更新计划文件：`plan/YYYY-MM-DD_HH-MM-SS-<slug>.md`。
5) 计划至少包含：`Bindings`、`Goal`、`Non-goals`、`Scope`、`Plan`、`Verify`、`Risks & Rollback`、`Evidence`。
6) 若已有 `changes/<change-id>/proposal.md`，对齐 `Plan_File` / `Contract_Row` / `Evidence_Path`。
7) 完成后先运行 `/ws-plan-verify`，通过再进入 `/ws-dev`。
<!-- AIWS_MANAGED_END:opencode:ws-plan -->

可在下方追加本项目对 OpenCode 的额外说明（托管块外内容会被保留）。
