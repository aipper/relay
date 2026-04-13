<!-- AIWS_MANAGED_BEGIN:claude:ws-verify-before-complete -->
# ws verify before complete

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：在 `/ws-finish` 或 `/ws-handoff` 前检查双审查、validate stamp 与 evidence 是否齐全。

步骤（建议）：
1) 检查以下最小 gate：
   - `changes/<change-id>/review/spec-review.md`
   - `changes/<change-id>/review/quality-review.md`
   - `.agentdocs/tmp/aiws-validate/*.json`
2) 若存在 `changes/<change-id>/evidence/`，检查 review / validate / collaboration summary 是否已收敛。
3) 将结果落盘到：
   - 默认：`changes/<change-id>/evidence/verify-before-complete.md`
   - 回退：`.agentdocs/tmp/review/verify-before-complete.md`
4) 输出：
   - `证据（Evidence）:`
   - `结论（Result）: pass|fail`
   - `缺失项（Missing）:`
   - `下一步（Next）:`
<!-- AIWS_MANAGED_END:claude:ws-verify-before-complete -->

可在下方追加本项目对 Claude Code 的额外说明（托管块外内容会被保留）。
