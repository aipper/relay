---
name: ws-migrate
description: 迁移（补齐/升级 AIWS 工作区文件）
---

用中文输出；命令与路径保持原样不翻译。

目标：把当前仓库补齐为 aiws workspace 模板，并启用可验证门禁。

执行（在项目根目录）：
```bash
if [[ -f ".aiws/manifest.json" ]]; then
  if [[ -x "./node_modules/.bin/aiws" ]]; then
    ./node_modules/.bin/aiws update .
  elif command -v aiws >/dev/null 2>&1; then
    aiws update .
  else
    npx @aipper/aiws update .
  fi
else
  if [[ -x "./node_modules/.bin/aiws" ]]; then
    ./node_modules/.bin/aiws init .
  elif command -v aiws >/dev/null 2>&1; then
    aiws init .
  else
    npx @aipper/aiws init .
  fi
fi

if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws validate .
elif command -v aiws >/dev/null 2>&1; then
  aiws validate .
else
  npx @aipper/aiws validate .
fi

git config core.hooksPath .githooks
```

回滚（恢复最近一次快照）：
```bash
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws rollback . latest
elif command -v aiws >/dev/null 2>&1; then
  aiws rollback . latest
else
  npx @aipper/aiws rollback . latest
fi
```

约束：
- 不写入任何 secrets。
- 不执行破坏性命令。
