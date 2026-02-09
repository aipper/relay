---
name: aiws-change-start
description: 切分支并初始化变更工件（可选安装 hooks）
---

目标：
- 切到分支 `change/<change-id>` 并初始化 `changes/<change-id>/` 工件
  - 若检测到 `.gitmodules`（git submodules），默认优先使用 `--worktree`（失败则回退为 `--no-switch`），避免切走 superproject 分支导致 submodule 状态混乱

要求：
- 需要 git 仓库；若不是 git 仓库先 `git init`

执行（在仓库根目录）：
```bash
change_id="<change-id>"
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws change start "$change_id"
elif command -v aiws >/dev/null 2>&1; then
  aiws change start "$change_id"
else
  npx @aipper/aiws change start "$change_id"
fi
```

可选参数：
- `--hooks`：同时执行 `aiws hooks install .`
- `--title <title>`：写入标题
- `--no-design`：不生成 design.md
- `--switch`：显式允许切换 superproject 分支（仅在存在 `.gitmodules` 时有意义）
- `--no-switch`：不切换当前分支（仅确保 `change/<change-id>` 分支存在并初始化工件）；适用于 superproject + submodule 场景
- `--worktree`：用 `git worktree` 创建独立工作区并在其中 checkout `change/<change-id>`（推荐用于 superproject + submodule）
  - `--worktree-dir <path>`：覆盖 worktree 目录
  - `--submodules`：在 worktree 内执行 `git submodule update --init --recursive`
