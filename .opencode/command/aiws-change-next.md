---
description: 变更下一步：输出当前 change 的建议动作
---
<!-- AIWS_MANAGED_BEGIN:opencode:aiws-change-next -->
# aiws change next

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 输出 change 下一步建议动作

执行（在仓库根目录）：
```bash
change_id="<change-id>"  # 可留空（若当前分支可推断）
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws change next "${change_id}"
elif command -v aiws >/dev/null 2>&1; then
  aiws change next "${change_id}"
else
  npx @aipper/aiws change next "${change_id}"
fi
```
<!-- AIWS_MANAGED_END:opencode:aiws-change-next -->

可在下方追加本项目对 OpenCode 的额外说明（托管块外内容会被保留）。
