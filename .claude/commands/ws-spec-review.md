<!-- AIWS_MANAGED_BEGIN:claude:ws-spec-review -->
# ws spec review

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：审查 requirements 归因、plan/change 绑定、evidence 与 workflow gate 完整性。

步骤（建议）：
1) 先运行 `/ws-preflight`。
2) 对照 `AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md` 检查：
   - 当前改动是否能归因到 `Req_ID` / `Problem_ID`
   - `plan/...`、`proposal.md`、`tasks.md`、`evidence/` 是否保持一致
   - 是否存在越界改动、危险操作或缺失 gate
3) 将结论落盘到：
   - 默认：`changes/<change-id>/review/spec-review.md`
   - 回退：`.agentdocs/tmp/review/spec-review.md`
4) 输出：
   - `证据（Evidence）:`
   - `阻断项（Blockers）:`
   - `警告（Warnings）:`
   - `下一步（Next）:`
<!-- AIWS_MANAGED_END:claude:ws-spec-review -->

可在下方追加本项目对 Claude Code 的额外说明（托管块外内容会被保留）。
