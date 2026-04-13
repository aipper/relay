---
description: 自动 bootstrap：检查 update、确保 watchdog 已启动，并给出下一步
---
<!-- AIWS_MANAGED_BEGIN:opencode:ws-auto -->
# ws auto

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：为当前 OpenCode 工作区执行一次显式 auto bootstrap，先补齐托管内容，再按条件启动 watchdog。

执行建议：
1) 先运行 `/ws-preflight`，确认 `AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md` 已读取。
2) 在项目根运行 `aiws opencode auto .`。
3) 输出：
   - `Auto Bootstrap:`
   - `Update:`
   - `OpenCode mode:`
   - `Watchdog:`
   - `Next:`
4) 若输出 `watchdog:` 为已启动状态：
   - 若当前任务意图明确：继续 `/using-aiws`、`/ws-plan`、`/ws-dev` 或 `/ws-review`
   - 若当前只是要声明自主协作合同：继续 `/ws-autonomy`
5) 若输出 `watchdog: skipped (...)`：
   - 明确写出原因
   - 不要假装已经进入 autonomous 模式

边界：
- 允许通过 `aiws update .` 刷新 AIWS 托管内容。
- 不自动复制 `.opencode/oh-my-opencode.json.example` 成真实配置。
- 不自动批准 host permission，不自动执行 tmux rescue。
<!-- AIWS_MANAGED_END:opencode:ws-auto -->

可在下方追加本项目对 auto bootstrap 的额外说明（托管块外内容会被保留）。
