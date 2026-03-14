<!-- AIWS_MANAGED_BEGIN:opencode:ws-review -->
# ws review

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：在提交/交付前审计当前改动，对照真值文件检查是否越界，并把审计证据优先落盘到 `changes/<change-id>/review/`（若无法确定 `change-id` 再回退 `.agentdocs/tmp/review/`）。

步骤（建议）：
1) 先运行 `/ws-preflight`。
2) 基于 `git status` / `git diff`（以及你实际运行过的测试结果），对照 `AI_PROJECT.md` 与 `REQUIREMENTS.md` 检查：
   - 是否存在越界目录改动/危险操作
   - 是否有可复现验证命令与证据
   - 是否维护了 `changes/<change-id>/` 或相关 `issues/*.csv`
3) 将审计落盘到（目录不存在则创建）：
   - 默认：`changes/<change-id>/review/opencode-review.md`
   - 回退：`.agentdocs/tmp/review/opencode-review.md`（仅在无法确定 `change-id` 时使用）
4) 回复中输出：
   - `证据（Evidence）:` 证据文件路径
   - `主要风险（Top risks）:` 3–8 条（高→低）
   - `下一步（Next）:` 最小修复清单 + 最小验证命令

安全：
- 不打印 secrets。
- 不执行破坏性命令。
<!-- AIWS_MANAGED_END:opencode:ws-review -->

可在下方追加本项目对 OpenCode 的额外说明（托管块外内容会被保留）。
