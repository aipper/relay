---
description: 变更同步：同步 change 与真值基线
---
<!-- AIWS_MANAGED_BEGIN:opencode:aiws-change-sync -->
# aiws change sync

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 同步当前 change 与真值文件基线

执行（在仓库根目录）：
```bash
change_id="<change-id>"
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws change sync "${change_id}"
elif command -v aiws >/dev/null 2>&1; then
  aiws change sync "${change_id}"
else
  npx @aipper/aiws change sync "${change_id}"
fi
```
<!-- AIWS_MANAGED_END:opencode:aiws-change-sync -->

可在下方追加本项目对 OpenCode 的额外说明（托管块外内容会被保留）。
