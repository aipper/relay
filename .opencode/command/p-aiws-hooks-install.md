---
description: 私有：Hooks 启用：启用 git hooks 门禁
---
<!-- AIWS_MANAGED_BEGIN:opencode:aiws-hooks-install -->
# aiws hooks install

（私有原子入口；日常优先用 ws-* 链路。）

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 启用 git hooks 门禁（core.hooksPath=.githooks）

执行（在仓库根目录）：
```bash
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws hooks install .
elif command -v aiws >/dev/null 2>&1; then
  aiws hooks install .
else
  npx @aipper/aiws hooks install .
fi
```
<!-- AIWS_MANAGED_END:opencode:aiws-hooks-install -->

可在下方追加本项目对 OpenCode 的额外说明（托管块外内容会被保留）。
