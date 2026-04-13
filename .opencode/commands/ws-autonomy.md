---
description: 自主协作实验：声明 completion/retry/rescue 合同，并按需启用 tmux helper
---
<!-- AIWS_MANAGED_BEGIN:opencode:ws-autonomy -->
# ws autonomy

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：声明当前 OpenCode/oMo 任务的 autonomous execution mode，并写清 completion/retry/rescue 合同。

执行建议：
1) 先运行 `/ws-preflight`，再读取 `packages/spec/docs/opencode-autonomous-swarm.md` 与 `packages/spec/docs/opencode-omo-adapter.md`。
2) 输出：
   - `Execution Mode: single-agent / omo-native / omo-native + tmux-swarm`
   - `Completion Contract:`
   - `Retry Contract:`
   - `Rescue Policy:`
   - `Evidence:`
3) 若选择 `omo-native + tmux-swarm`：
   - 先运行 `.opencode/helpers/tmux-swarm-scan.sh`
   - 仅在需要安全白名单救援时运行 `.opencode/helpers/tmux-swarm-rescue.sh`
4) 明确这是实验层，不接管 runtime controller，不替代 `ws-plan` / `ws-dev` / `ws-review`。
<!-- AIWS_MANAGED_END:opencode:ws-autonomy -->

可在下方追加本项目对 autonomous swarm 的额外说明（托管块外内容会被保留）。
