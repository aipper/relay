---
name: ws-submodule-setup
description: Thin wrapper for `aiws submodule-setup`
---

# ws-submodule-setup

Thin skill wrapper. Delegates to `aiws submodule-setup`. See `aiws submodule-setup --help` for details.

```bash
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws submodule-setup
elif command -v aiws >/dev/null 2>&1; then
  aiws submodule-setup
else
  npx @aipper/aiws submodule-setup
fi
```
