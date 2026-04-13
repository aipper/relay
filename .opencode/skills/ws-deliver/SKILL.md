---
name: ws-deliver
description: Thin wrapper for `aiws deliver`
---

# ws-deliver

Thin skill wrapper. Delegates to `aiws deliver`. See `aiws deliver --help` for details.

```bash
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws deliver
elif command -v aiws >/dev/null 2>&1; then
  aiws deliver
else
  npx @aipper/aiws deliver
fi
```
