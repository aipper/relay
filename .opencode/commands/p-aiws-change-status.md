---
description: 私有：变更状态：查看 change 状态与阻断项
---
<!-- AIWS_MANAGED_BEGIN:opencode:aiws-change-status -->
# aiws change status

（私有原子入口；日常优先用 ws-* 链路。）

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 查看 change 当前状态与阻断项

执行（在仓库根目录）：
```bash
change_id="<change-id>"  # 可留空（若当前分支可推断）
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws change status "${change_id}"
elif command -v aiws >/dev/null 2>&1; then
  aiws change status "${change_id}"
else
  npx @aipper/aiws change status "${change_id}"
fi
```
<!-- AIWS_MANAGED_END:opencode:aiws-change-status -->

可在下方追加本项目对 OpenCode 的额外说明（托管块外内容会被保留）。
