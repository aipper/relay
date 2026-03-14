---
name: ws-preflight
description: 预检（提交前快速检查与建议）
---

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
3) 若缺失任意真值文件：不要继续“写代码/改配置/落盘文件”。先输出缺失项，并给出下一步建议：
   - `npx @aipper/aiws init .`（或 `aiws init .`）初始化真值文件
   - 然后重新执行 `$ws-preflight`
4) 输出：
   - `Root:` <项目根路径>
   - `Found:` <实际读取到的文件列表>
   - `Missing:` <缺失文件列表>
   - `Key rules:` 3–8 条 bullet（范围/禁止项/必须产物/必须验证命令）
5) 若存在 `.gitmodules`：
   - 输出 submodule 列表：`git config --file .gitmodules --get-regexp '^submodule\\..*\\.path$'`
   - 检查每个 submodule 是否配置 `submodule.<name>.branch`（缺失则提示先运行 `$ws-submodule-setup`；否则 `aiws validate .` 会失败）

安全：
- 不打印 secrets；遇到疑似敏感值只提示“存在风险”但不要复述原文。
- 不执行破坏性命令。
