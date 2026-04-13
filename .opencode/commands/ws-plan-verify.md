---
description: 计划质检：执行前检查并给出最小修正项
---
<!-- AIWS_MANAGED_BEGIN:opencode:ws-plan-verify -->
# ws plan verify

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 执行前检查计划是否跑偏，并给出最小修正项。

执行建议：
1) 先运行 `/ws-preflight`。
2) 运行严格门禁：
```bash
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws change validate <change-id> --strict
elif command -v aiws >/dev/null 2>&1; then
  aiws change validate <change-id> --strict
else
  npx @aipper/aiws change validate <change-id> --strict
fi
```
3) 若失败：先修 `proposal.md` / `plan` 的绑定字段与验证命令，再复跑。
4) 通过后再进入 `/ws-dev`。
<!-- AIWS_MANAGED_END:opencode:ws-plan-verify -->

可在下方追加本项目对 OpenCode 的额外说明（托管块外内容会被保留）。
