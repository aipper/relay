<!-- AIWS_MANAGED_BEGIN:claude:ws-plan -->
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
5) 计划至少包含：`绑定信息（Bindings）`、`目标（Goal）`、`非目标（Non-goals）`、`范围（Scope）`、`执行计划（Plan）`、`验证（Verify）`、`风险与回滚（Risks & Rollback）`、`证据（Evidence）`。
6) 若已有 `changes/<change-id>/proposal.md`，对齐 `计划文件（Plan_File）` / `合同行（Contract_Row）` / `证据路径（Evidence_Path）`。
7) 完成后先运行 `/ws-plan-verify`，通过再进入 `/ws-dev`。
<!-- AIWS_MANAGED_END:claude:ws-plan -->

可在下方追加本项目对 Claude Code 的额外说明（托管块外内容会被保留）。
