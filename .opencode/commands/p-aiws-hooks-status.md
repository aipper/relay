---
description: 私有：Hooks 状态：查看当前仓库 hooks 门禁状态
---
<!-- AIWS_MANAGED_BEGIN:opencode:aiws-hooks-status -->
# aiws hooks status

（私有原子入口；日常优先用 ws-* 链路。）

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 查看当前仓库 hooks 门禁状态（只读）

执行（在仓库根目录）：
```bash
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws hooks status .
elif command -v aiws >/dev/null 2>&1; then
  aiws hooks status .
else
  npx @aipper/aiws hooks status .
fi
```
<!-- AIWS_MANAGED_END:opencode:aiws-hooks-status -->

可在下方追加本项目对 OpenCode 的额外说明（托管块外内容会被保留）。
