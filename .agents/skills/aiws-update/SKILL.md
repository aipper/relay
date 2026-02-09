---
name: aiws-update
description: 更新工作区（刷新模板与托管块）
---

目标：
- 基于当前 `@aipper/aiws-spec` 刷新模板与 tool-native 文件
- 更新前备份到 `.aiws/backups/<timestamp>/`

执行（在仓库根目录）：
```bash
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws update .
  ./node_modules/.bin/aiws validate .
elif command -v aiws >/dev/null 2>&1; then
  aiws update .
  aiws validate .
else
  npx @aipper/aiws update .
  npx @aipper/aiws validate .
fi
```

约束：
- 不写入任何 secrets
- 只更新托管块或 spec 指定的 `replace_file` 文件
