---
name: p-aiws-rollback
description: 私有：回滚工作区（从备份恢复）
---

目标：
- 从 `.aiws/backups/` 恢复到某次备份快照

执行（在仓库根目录）：
```bash
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws rollback . latest
elif command -v aiws >/dev/null 2>&1; then
  aiws rollback . latest
else
  npx @aipper/aiws rollback . latest
fi
```
