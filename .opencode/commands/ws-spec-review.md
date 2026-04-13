---
description: 规范审查：requirements / plan / evidence / gate 完整性
---
<!-- AIWS_MANAGED_BEGIN:opencode:ws-spec-review -->
# ws spec review

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：审查 requirements 归因、plan/change 绑定、evidence 与 workflow gate 完整性。

步骤（建议）：
1) 先运行 `/ws-preflight`。
   - 若检测到 `.opencode/oh-my-opencode.json` 或当前会话明确可用 `oracle` / `librarian`：优先按 `packages/spec/docs/opencode-omo-adapter.md` 借用这些 agent。
   - requirements / gate / evidence 审查优先 `@oracle`；规范与文档真值补充优先 `@librarian`。
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
5) 若 oMo 不可用：回退为当前 agent 本地 spec review。
<!-- AIWS_MANAGED_END:opencode:ws-spec-review -->

可在下方追加本项目对 OpenCode 的额外说明（托管块外内容会被保留）。
