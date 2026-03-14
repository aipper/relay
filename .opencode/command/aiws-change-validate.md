---
description: 变更校验：按 strict 校验变更工件与绑定
---
<!-- AIWS_MANAGED_BEGIN:opencode:aiws-change-validate -->
# aiws change validate

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 按严格模式校验变更工件与绑定关系

执行（在仓库根目录）：
```bash
change_id="<change-id>"
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws change validate "${change_id}"
elif command -v aiws >/dev/null 2>&1; then
  aiws change validate "${change_id}"
else
  npx @aipper/aiws change validate "${change_id}"
fi
```
<!-- AIWS_MANAGED_END:opencode:aiws-change-validate -->

可在下方追加本项目对 OpenCode 的额外说明（托管块外内容会被保留）。
