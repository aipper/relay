<!-- AIWS_MANAGED_BEGIN:claude:ws-plan -->
# ws plan

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 生成可落盘执行计划（供 /ws-dev 执行）。

执行建议：
1) 先运行 `/ws-preflight`（对齐 `AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`）。
2) 生成或更新计划文件：`plan/YYYY-MM-DD_HH-MM-SS-<slug>.md`。
3) 计划至少包含：`绑定信息（Bindings）`、`目标（Goal）`、`非目标（Non-goals）`、`范围（Scope）`、`执行计划（Plan）`、`验证（Verify）`、`风险与回滚（Risks & Rollback）`、`证据（Evidence）`。
4) 若已有 `changes/<change-id>/proposal.md`，对齐 `计划文件（Plan_File）` / `合同行（Contract_Row）` / `证据路径（Evidence_Path）`。
5) 完成后先运行 `/ws-plan-verify`，通过再进入 `/ws-dev`。
<!-- AIWS_MANAGED_END:claude:ws-plan -->

可在下方追加本项目对 Claude Code 的额外说明（托管块外内容会被保留）。
