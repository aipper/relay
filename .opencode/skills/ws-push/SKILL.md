---
name: ws-push
description: Thin wrapper for `aiws push`
---

# ws-push

Thin skill wrapper. Delegates to `aiws push`. See `aiws push --help` for details.

```bash
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws push
elif command -v aiws >/dev/null 2>&1; then
  aiws push
else
  npx @aipper/aiws push
fi
```
