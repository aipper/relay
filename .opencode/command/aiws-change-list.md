---
description: 变更列表：列出当前仓库 change 工件
---
<!-- AIWS_MANAGED_BEGIN:opencode:aiws-change-list -->
# aiws change list

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 列出当前仓库 change 工件

执行（在仓库根目录）：
```bash
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws change list
elif command -v aiws >/dev/null 2>&1; then
  aiws change list
else
  npx @aipper/aiws change list
fi
```
<!-- AIWS_MANAGED_END:opencode:aiws-change-list -->

可在下方追加本项目对 OpenCode 的额外说明（托管块外内容会被保留）。
