---
name: ws-finish
description: 收尾（门禁 + 安全合并 + submodule→主仓库顺序 push）
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 在结束一次变更交付时，用 fast-forward 安全合并 `change/<change-id>` 回目标分支，减少手输分支名导致的错误
- 合并成功后，按 `submodule -> superproject` 顺序 push，避免遗漏导致其它仓库拉取异常
- 不自动删分支；push 前必须让用户确认
 - 若团队希望减少 submodule detached 的人为差异：建议在 `.gitmodules` 配置 `submodule.<name>.branch`，并在日常拉取时使用 `$ws-pull`

前置（必须）：
- 工作区是干净的：`git status --porcelain` 无输出（若有未提交改动：先 commit 或 stash）
- change 分支已存在：`change/<change-id>`（也支持 `changes/`、`ws/`、`ws-change/`）
- 若使用 worktree：在“目标分支所在 worktree”执行（`aiws change finish` 会提示正确的 worktree）
- 若存在 `.gitmodules`：必须为每个 submodule 配置 `submodule.<name>.branch`（否则无法确定性减少 detached；先运行 `$ws-submodule-setup` 并提交 `.gitmodules`）

建议步骤：
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
    echo "hint: run $ws-submodule-setup (and commit .gitmodules), then retry"
    exit 2
  fi
fi
```
1) （推荐）先跑一次门禁并落盘证据：
```bash
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws validate . --stamp
elif command -v aiws >/dev/null 2>&1; then
  aiws validate . --stamp
else
  npx @aipper/aiws validate . --stamp
fi
```
1.1) （强烈建议）收敛持久证据并回填 `Evidence_Path`：
```bash
change_id="<change-id>"
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws change evidence "${change_id}"
elif command -v aiws >/dev/null 2>&1; then
  aiws change evidence "${change_id}"
else
  npx @aipper/aiws change evidence "${change_id}"
fi
```
1.2) （可选）生成状态快照（建议）：
```bash
aiws change state "${change_id}" --write
```
2) 安全合并（默认 fast-forward；并会在需要时切到目标分支）：
```bash
# 若当前就在 change/<change-id> 分支上，可省略 <change-id>
change_id="<change-id>"
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws change finish "${change_id}"
elif command -v aiws >/dev/null 2>&1; then
  aiws change finish "${change_id}"
else
  npx @aipper/aiws change finish "${change_id}"
fi
```
3) 若 fast-forward 失败（提示需要 rebase）：先在 change 分支（或对应 worktree）里 `git rebase <target-branch>`，再重试 `aiws change finish`。
4) 合并成功后，先把每个 submodule 的 gitlink commit 合并回其目标分支（解决 detached HEAD），并按顺序 push：
```bash
# superproject 当前分支（finish 后通常是 base 分支）
base_branch="$(git branch --show-current)"

# 子模块清单（没有则跳过）
git config --file .gitmodules --get-regexp '^submodule\..*\.path$' 2>/dev/null || true

# 对每个 submodule.<name>.path <sub_path>：
# 说明：`git submodule update` 会把 submodule checkout 到固定 gitlink commit，导致 detached HEAD。
# 为减少“游离状态”的协作摩擦，本步骤采用“pin 分支”策略：
# - 仅在 `.gitmodules` 明确配置了 `submodule.<name>.branch` 时执行（避免 origin 多分支导致误判）
# - 不要直接切 `change/<change-id>` / `main` / `master` 等业务分支来“解 detached”
# - 不改动 submodule 现有分支指针（例如不强行移动 main/master）
# - 创建/更新本地 pin 分支：`aiws/pin/<target_branch>` 指向 gitlink commit，并将其 upstream 设为 `origin/<target_branch>`
sub_sha="$(git rev-parse "HEAD:<sub_path>")"
cfg_branch="$(git config --file .gitmodules --get "submodule.<name>.branch" 2>/dev/null || true)"
if [[ "${cfg_branch:-}" == "." ]]; then cfg_branch="$base_branch"; fi
if [[ -z "${cfg_branch:-}" ]]; then
  echo "[warn] <sub_path>: missing .gitmodules submodule.<name>.branch; keep detached and skip auto-push"
  continue
fi
target_branch="$cfg_branch"
pin_branch="aiws/pin/${target_branch}"

git -C "<sub_path>" fetch origin --prune
if ! git -C "<sub_path>" show-ref --verify --quiet "refs/remotes/origin/${target_branch}"; then
  echo "[warn] <sub_path>: origin/${target_branch} not found; keep detached and skip auto-push"
  continue
fi

# 仅当 gitlink commit 属于 origin/<target_branch> 的历史时才“挂回分支”
if ! git -C "<sub_path>" merge-base --is-ancestor "${sub_sha}" "origin/${target_branch}"; then
  echo "[warn] <sub_path>: ${sub_sha} is not in origin/${target_branch}; keep detached and stop (need manual reconcile)"
  exit 1
fi

git -C "<sub_path>" checkout -B "${pin_branch}" "${sub_sha}"
git -C "<sub_path>" branch --set-upstream-to "origin/${target_branch}" "${pin_branch}" >/dev/null 2>&1 || true
git -C "<sub_path>" status -sb

# push：只允许 fast-forward（若远端分叉会被拒绝；此时必须人工处理）
git -C "<sub_path>" push origin "${sub_sha}:refs/heads/${target_branch}"
```
规则：
- 每个 submodule 必须先执行“pin 分支挂回 + fast-forward push”，再 push 主仓库。
- 若任一 submodule 的 gitlink commit 不在 `origin/<target_branch>` 历史中：立即停止，先人工处理分叉，再继续。

5) 仅当 submodules 全部成功后，再 push superproject 当前分支：
```bash
git branch --show-current
git status -sb
git push
```
6) push 成功后，清理 `change/<change-id>` 对应 worktree（若存在且不是当前 worktree）：
```bash
change_id="<change-id>"
change_ref="refs/heads/change/${change_id}"
main_wt="$(git rev-parse --show-toplevel)"
change_wt="$(git worktree list --porcelain | awk -v ref="$change_ref" '
  $1=="worktree" { wt=substr($0,10) }
  $1=="branch" && $2==ref { print wt; exit }
')"

if [[ -n "${change_wt:-}" && "$change_wt" != "$main_wt" ]]; then
  if [[ -n "$(git -C "$change_wt" status --porcelain 2>/dev/null)" ]]; then
    echo "[warn] worktree not clean, skip remove: $change_wt"
    echo "hint: clean it first, then run: git worktree remove \"$change_wt\""
  else
    git worktree remove "$change_wt"
    git worktree prune
  fi
fi
```
规则：
- 清理前先把 `change_wt` 输出给用户确认，避免误删。
- 仅使用 `git worktree remove`（不带 `--force`）。

7) （可选）归档变更工件（完成交付后推荐）：
```bash
change_id="<change-id>"
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws change archive "${change_id}"
elif command -v aiws >/dev/null 2>&1; then
  aiws change archive "${change_id}"
else
  npx @aipper/aiws change archive "${change_id}"
fi
```
