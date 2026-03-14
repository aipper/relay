<!-- AIWS_MANAGED_BEGIN:claude:ws-finish -->
# ws finish

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：安全把 `change/<change-id>` fast-forward 合并回目标分支，避免手输分支名导致的错误。
补充：若团队希望减少 submodule detached 的人为差异，建议在 `.gitmodules` 配置 `submodule.<name>.branch`，并在日常拉取时使用 `/ws-pull`。

前置（必须）：
- 工作区干净：`git status --porcelain` 无输出（否则先 commit 或 stash）
- change 分支存在（`change/<change-id>`；也支持 `changes/`、`ws/`、`ws-change/`）
 - 若存在 `.gitmodules`：必须为每个 submodule 配置 `submodule.<name>.branch`（否则先运行 `/ws-submodule-setup` 并提交 `.gitmodules`）

步骤（建议）：
0) 若存在 `.gitmodules`，先检查 submodule branch 配置是否齐全（缺失则停止并提示 setup）：
```bash
if [[ -f .gitmodules ]]; then
  missing=0
  while read -r key sub_path; do
    name="${key#submodule.}"; name="${name%.path}"
    b="$(git config --file .gitmodules --get "submodule.${name}.branch" 2>/dev/null || true)"
    if [[ -z "${b:-}" ]]; then
      echo "error: missing .gitmodules submodule.${name}.branch (path=${sub_path})"
      missing=1
    fi
  done < <(git config --file .gitmodules --get-regexp '^submodule\\..*\\.path$' 2>/dev/null || true)
  if [[ "$missing" -ne 0 ]]; then
    echo "hint: run /ws-submodule-setup (and commit .gitmodules), then retry"
    exit 2
  fi
fi
```
1) 先运行 `/ws-preflight`（确保真值文件齐全）。
2) （推荐）门禁校验并落盘证据：`aiws validate . --stamp`（未安装全局 aiws 时可用 `npx @aipper/aiws validate . --stamp`）。
2.1) （强烈建议）收敛持久证据并回填 `Evidence_Path`：`aiws change evidence <change-id>`（未安装全局 aiws 时可用 `npx @aipper/aiws change evidence <change-id>`）。
2.2) （可选）生成状态快照（建议）：`aiws change state <change-id> --write`。
3) 安全合并并切回目标分支：
   - 若当前就在 `change/<change-id>` 分支上，可直接执行：`aiws change finish`
   - 否则执行：`aiws change finish <change-id>`
4) 若提示无法 fast-forward：先在 change 分支（或对应 worktree）里 `git rebase <target-branch>`，再重试 `aiws change finish`。
5) 合并成功后，按顺序处理每个 submodule（减少 detached；再 push）：
   - 发现 submodules：`git config --file .gitmodules --get-regexp '^submodule\\..*\\.path$'`
   - 对每个 `<sub_path>`：
     - 读取 superproject 当前 gitlink：`git rev-parse "HEAD:<sub_path>"`
     - 目标分支：必须在 `.gitmodules` 配置 `submodule.<name>.branch`（若为 `.` 则用当前主仓库分支；避免 origin 多分支时误判）
     - 不要直接切 `change/<change-id>` / `main` / `master` 来“解 detached”
     - 用 pin 分支挂回（不改动现有 main/master 指针）：`git -C "<sub_path>" checkout -B "aiws/pin/<target-branch>" <gitlink-sha>`
     - 仅当 `<gitlink-sha>` 属于 `origin/<target-branch>` 历史时才允许 push；否则停止并人工处理分叉
     - push（只允许 fast-forward）：`git -C "<sub_path>" push origin "<gitlink-sha>:refs/heads/<target-branch>"`
6) 任一 submodule 不满足 fast-forward 条件时立即停止（不要继续 push 主仓库）。
7) submodules 全部成功后，再 push 主仓库当前分支：
   - `git branch --show-current`
   - `git status -sb`
   - `git push`
8) push 成功后，清理 `change/<change-id>` 对应 worktree（若存在且不是当前 worktree）：
   - `change_ref="refs/heads/change/<change-id>"`
   - `main_wt="$(git rev-parse --show-toplevel)"`
   - `change_wt="$(git worktree list --porcelain | awk -v ref="$change_ref" '$1=="worktree"{wt=substr($0,10)} $1=="branch"&&$2==ref{print wt; exit}')"`
   - 若 `change_wt` 非空且不等于 `main_wt`：先输出并让用户确认，再执行 `git worktree remove "$change_wt"`（不带 `--force`），最后 `git worktree prune`
   - 若 `git -C "$change_wt" status --porcelain` 非空：停止并提示先清理该 worktree
9) （可选）交付完成后归档变更工件：`aiws change archive <change-id>`。

安全：
- push 前先输出状态并请用户确认远端/分支。
- 不执行破坏性命令。
<!-- AIWS_MANAGED_END:claude:ws-finish -->

可在下方追加本项目对 Claude Code 的额外说明（托管块外内容会被保留）。
