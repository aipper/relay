<!-- AIWS_MANAGED_BEGIN:opencode:ws-finish -->
# ws finish

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：安全把 `change/<change-id>` fast-forward 合并回目标分支，避免手输分支名导致的错误。

前置（必须）：
- 工作区干净：`git status --porcelain` 无输出（否则先 commit 或 stash）
- change 分支存在（`change/<change-id>`；也支持 `changes/`、`ws/`、`ws-change/`）

步骤（建议）：
1) 先运行 `/ws-preflight`（确保真值文件齐全）。
2) （推荐）门禁校验并落盘证据：`aiws validate . --stamp`（未安装全局 aiws 时可用 `npx @aipper/aiws validate . --stamp`）。
3) 安全合并并切回目标分支：
   - 若当前就在 `change/<change-id>` 分支上，可直接执行：`aiws change finish`
   - 否则执行：`aiws change finish <change-id>`
4) 若提示无法 fast-forward：先在 change 分支（或对应 worktree）里 `git rebase <target-branch>`，再重试 `aiws change finish`。
5) 合并成功后，按顺序处理每个 submodule（先“并回目标分支”，再 push）：
   - 发现 submodules：`git config --file .gitmodules --get-regexp '^submodule\\..*\\.path$'`
   - 对每个 `<sub_path>`：
     - 读取 superproject 当前 gitlink：`git rev-parse "HEAD:<sub_path>"`
     - 推断目标分支：优先 `.gitmodules` 的 `submodule.<name>.branch`（若为 `.` 则用当前主仓库分支），否则 `origin/HEAD`，再 fallback `main/master`
     - 切到目标分支（必要时创建跟踪分支）：`git -C "<sub_path>" switch <target-branch> || git -C "<sub_path>" switch -c <target-branch> --track origin/<target-branch>`
     - 合并 gitlink commit：`git -C "<sub_path>" merge --ff-only <gitlink-sha>`
     - push：`git -C "<sub_path>" push`（无 upstream 则 `git -C "<sub_path>" push -u origin <target-branch>`）
6) 任一 submodule `merge --ff-only` 失败时立即停止（不要继续 push 主仓库）。
7) submodules 全部成功后，再 push 主仓库当前分支：
   - `git branch --show-current`
   - `git status -sb`
   - `git push`
8) （可选）交付完成后归档变更工件：`aiws change archive <change-id>`。

安全：
- push 前先输出状态并请用户确认远端/分支。
- 不执行破坏性命令。
<!-- AIWS_MANAGED_END:opencode:ws-finish -->

可在下方追加本项目对 OpenCode 的额外说明（托管块外内容会被保留）。
