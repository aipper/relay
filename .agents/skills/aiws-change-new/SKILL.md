---
name: aiws-change-new
description: 创建 changes/<change-id> 工件
---

目标：
- 创建 `changes/<change-id>/` 工件目录与基础文件（proposal/tasks/可选 design）

要求：
- `change-id` 必须是 kebab-case：`^[a-z0-9]+(-[a-z0-9]+)*$`

执行（在仓库根目录）：
```bash
change_id="<change-id>"
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws change new "$change_id"
elif command -v aiws >/dev/null 2>&1; then
  aiws change new "$change_id"
else
  npx @aipper/aiws change new "$change_id"
fi
```

可选参数：
- `--title <title>`：写入标题
- `--no-design`：不生成 design.md
