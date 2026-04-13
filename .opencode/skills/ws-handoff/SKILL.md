---
name: ws-handoff
description: Thin wrapper for `aiws handoff`
---

# ws-handoff

Thin skill wrapper. Delegates to `aiws handoff`. See `aiws handoff --help` for details.

```bash
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws handoff
elif command -v aiws >/dev/null 2>&1; then
  aiws handoff
else
  npx @aipper/aiws handoff
fi
```
