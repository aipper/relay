---
name: ws-plan-verify
description: Thin wrapper for `aiws plan-verify`
---

# ws-plan-verify

Thin skill wrapper. Delegates to `aiws plan-verify`. See `aiws plan-verify --help` for details.

```bash
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws plan-verify
elif command -v aiws >/dev/null 2>&1; then
  aiws plan-verify
else
  npx @aipper/aiws plan-verify
fi
```
