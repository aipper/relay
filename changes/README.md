# changes/（变更工件目录）

本目录用于保存每一次“需求交付 / 问题修复”的可审计产物，避免关键约定只存在于聊天记录。

真值文件仍然是：
- `AI_PROJECT.md`（约束真值）
- `AI_WORKSPACE.md`（运行/测试真值）
- `REQUIREMENTS.md`（需求/验收真值）

建议结构：
```
changes/
  README.md
  <change-id>/
    proposal.md
    design.md        # 可选
    tasks.md
    .ws-change.json
  archive/
    YYYY-MM-DD-<change-id>/
```

常用命令（推荐使用 `aiws`；不依赖 dotfiles）：
- `aiws change start <change-id>`（默认：切到 `change/<change-id>` 并初始化工件目录；若检测到 `.gitmodules` 则默认优先使用 worktree，失败则回退为不切分支，避免 submodule 状态混乱）
- `aiws change start <change-id> --no-switch`（superproject + submodule 场景：不切分支，仅准备 `change/<change-id>` 分支与工件目录）
- `aiws change start <change-id> --switch`（显式允许切换 superproject 分支；仅在存在 `.gitmodules` 时有意义）
- `aiws change start <change-id> --worktree`（推荐用于 superproject + submodule：创建独立 worktree；当前目录分支保持不变）
  - 可选：`--worktree-dir <path>` 覆盖 worktree 目录
  - 可选：`--submodules` 在 worktree 内执行 `git submodule update --init --recursive`
- `aiws change finish <change-id>`（安全合并：fast-forward 合并回目标分支；在 `change/<change-id>` 分支上执行时会尝试使用 `.ws-change.json` 的 `base_branch` 作为目标分支）
- `aiws change new <change-id>`
- `aiws change list`
- `aiws change status <change-id>`
- `aiws change next <change-id>`
- `aiws change validate <change-id>`
- `aiws change sync <change-id>`
- `aiws change archive <change-id>`

Active change（推荐，团队共享）：
- 使用分支名声明当前变更：`change/<change-id>`（也支持 `changes/`、`ws/`、`ws-change/`）
- 切到该分支后，可省略 `<change-id>` 执行：`aiws change status|next|validate|sync|archive`

模板覆盖（可选）：
- 在工作区创建 `changes/templates/` 可覆盖默认模板：
  - `changes/templates/proposal.md`
  - `changes/templates/tasks.md`
  - `changes/templates/design.md`
- 快速初始化模板：`aiws change templates init`
- 查看模板来源：`aiws change templates which`

注意：
- 不要把任何 secrets 写进 proposal/design/tasks（账号、token、内网地址等用本地私有文件/环境变量）。
- 若真值文件（`AI_PROJECT.md` / `AI_WORKSPACE.md` / `REQUIREMENTS.md`）在变更期间发生变化，严格校验/归档会要求先运行 `aiws change sync <change-id>` 确认基线。

Hooks/CI（推荐，硬约束）：
- 工作区会安装 `.githooks/{pre-commit,pre-push}`（默认执行 `aiws validate .`）与门禁脚本（`tools/ws_change_check.py`、`tools/requirements_contract.py`）。
- **启用 hooks 需要本地配置**（不会自动提交到 git）：
  - 直接启用：`git config core.hooksPath .githooks`
  - 或使用：`aiws hooks install .`（等价）
- CI 建议增加一步（对 PR 分支执行）：`aiws validate .`。
- 紧急跳过（不推荐）：`WS_CHANGE_HOOK_BYPASS=1 ...`（CI 不应允许跳过）。

合并建议（减少人为 merge 引入新问题）：
- 优先使用 worktree + fast-forward 合并：
  - 完成后在主工作区（例如 `main`）执行：`aiws change finish <change-id>`（等价于 `git merge --ff-only change/<change-id>`）
  - 若失败：先在 worktree 里 `git rebase main`（或更新 main 后再 rebase），再重试 `--ff-only`
