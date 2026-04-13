---
name: p-aiws-change-status
description: 私有：查看单个变更工件状态（只读）
---

目标：
- 查看指定 `change-id` 的工件状态与下一步建议

执行（在仓库根目录）：
```bash
change_id="<change-id>"
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws change status "$change_id"
elif command -v aiws >/dev/null 2>&1; then
  aiws change status "$change_id"
else
  npx @aipper/aiws change status "$change_id"
fi
```
