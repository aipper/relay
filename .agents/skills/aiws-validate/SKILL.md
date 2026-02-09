---
name: aiws-validate
description: 校验工作区（漂移检测 + 门禁）
---

目标：
- 作为 CI/本地门禁：校验 required 文件结构、托管块、`.aiws/manifest.json` 漂移
- 强门禁：缺 `python3`/缺 required 脚本也应失败

执行（在仓库根目录）：
```bash
if [[ -x "./node_modules/.bin/aiws" ]]; then
  AIWS_VALIDATE_STAMP=1 ./node_modules/.bin/aiws validate .
elif command -v aiws >/dev/null 2>&1; then
  AIWS_VALIDATE_STAMP=1 aiws validate .
else
  AIWS_VALIDATE_STAMP=1 npx @aipper/aiws validate .
fi
```

证据（可选）：
- stamp：`.agentdocs/tmp/aiws-validate/*.json`（由 `.gitignore` 忽略）
