---
name: ws-migrate
description: Thin wrapper for `aiws migrate`
---

# ws-migrate

Thin skill wrapper. Delegates to `aiws migrate`. See `aiws migrate --help` for details.

```bash
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws migrate
elif command -v aiws >/dev/null 2>&1; then
  aiws migrate
else
  npx @aipper/aiws migrate
fi
```
