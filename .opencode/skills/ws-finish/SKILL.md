---
name: ws-finish
description: `aiws finish` 的收尾入口（finish_resume_required）
---

# ws-finish

`aiws finish` 的收尾入口。

关键契约：
- 若 `aiws change status <change-id>` 输出 `governance_rule: finish_resume_required`，继续执行 `aiws change finish <change-id> --push`
- 普通 finish 的 `validate/evidence/state` 仍应在 `change/<change-id>` worktree 完成，不要在目标 worktree 里跑 `aiws validate . --stamp`

```bash
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws finish
elif command -v aiws >/dev/null 2>&1; then
  aiws finish
else
  npx @aipper/aiws finish
fi
```
