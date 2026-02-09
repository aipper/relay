<!-- AIWS_MANAGED_BEGIN:claude:ws-analyze -->
# ws analyze

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：在开始实现/修复前做一次技术分析，产出可执行的最小行动清单，并把证据落盘到 `.agentdocs/tmp/analyze/`。

输入：
- 主题/需求：`$ARGUMENTS`（若为空，先问用户一句“本次分析主题是什么？”）

步骤（建议）：
1) 先运行 `/ws-preflight`。
2) 基于真值文件与当前代码现状，输出：
   - 目标 / 非目标
   - 现状证据（文件路径/接口路径）
   - 推荐方案（1 个）+ 备选（可选）
   - 风险与回滚（3–8 条）
   - 最小验证命令（可复现）
3) 将分析落盘到：`.agentdocs/tmp/analyze/claude-analysis.md`（目录不存在则创建）。
4) 回复中必须包含：`Evidence:` 证据文件路径。

安全：
- 不打印 secrets（尤其 `secrets/test-accounts.json`）。
- 不执行破坏性命令。
<!-- AIWS_MANAGED_END:claude:ws-analyze -->

可在下方追加本项目对 Claude Code 的额外说明（托管块外内容会被保留）。
