<!-- AIWS_MANAGED_BEGIN:claude:ws-preflight -->
# ws preflight

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：在开始任何“写代码/改配置/落盘文件”之前，对齐工作区真值文件，避免规则漂移。

执行步骤（强制）：
1) 定位项目根目录：
   - 优先：`git rev-parse --show-toplevel`
   - 若失败：停止并让用户确认当前目录是否为项目根（不要猜测）。
2) 在项目根目录读取以下文件（存在则必须读取；缺失则明确报告缺失项，不要臆测内容）：
   - `AI_PROJECT.md`
   - `REQUIREMENTS.md`
   - `AI_WORKSPACE.md`
3) 输出：
   - `Root:` <项目根路径>
   - `Found:` <实际读取到的文件列表>
   - `Missing:` <缺失文件列表>
   - `Key rules:` 3–8 条 bullet（范围/禁止项/必须产物/必须验证命令）

安全：
- 不打印 secrets；遇到疑似敏感值只提示“存在风险”但不要复述原文。
- 不执行破坏性命令。
<!-- AIWS_MANAGED_END:claude:ws-preflight -->

可在下方追加本项目对 Claude Code 的额外说明（托管块外内容会被保留）。
