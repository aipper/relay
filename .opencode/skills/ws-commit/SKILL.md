---
name: ws-commit
description: Thin wrapper for `aiws commit`
---

# ws-commit

Thin skill wrapper. Delegates to `aiws commit`. See `aiws commit --help` for details.

```bash
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws commit
elif command -v aiws >/dev/null 2>&1; then
  aiws commit
else
  npx @aipper/aiws commit
fi
```
