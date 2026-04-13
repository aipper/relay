---
name: ws-pull
description: Thin wrapper for `aiws pull`
---

# ws-pull

Thin skill wrapper. Delegates to `aiws pull`. See `aiws pull --help` for details.

```bash
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws pull
elif command -v aiws >/dev/null 2>&1; then
  aiws pull
else
  npx @aipper/aiws pull
fi
```
