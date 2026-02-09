---
name: ws-finish
description: 收尾（门禁 + 安全合并 + submodule→主仓库顺序 push）
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 在结束一次变更交付时，用 fast-forward 安全合并 `change/<change-id>` 回目标分支，减少手输分支名导致的错误
- 合并成功后，按 `submodule -> superproject` 顺序 push，避免遗漏导致其它仓库拉取异常
- 不自动删分支；push 前必须让用户确认

前置（必须）：
- 工作区是干净的：`git status --porcelain` 无输出（若有未提交改动：先 commit 或 stash）
- change 分支已存在：`change/<change-id>`（也支持 `changes/`、`ws/`、`ws-change/`）
- 若使用 worktree：在“目标分支所在 worktree”执行（`aiws change finish` 会提示正确的 worktree）

建议步骤：
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
# 1) 计算 superproject 当前 HEAD 记录的 gitlink commit
sub_sha="$(git rev-parse "HEAD:<sub_path>")"
# 2) 推断目标分支（优先：.gitmodules 的 submodule.<name>.branch；否则 origin/HEAD；否则 main/master）
cfg_branch="$(git config --file .gitmodules --get "submodule.<name>.branch" 2>/dev/null || true)"
if [[ "${cfg_branch:-}" == "." ]]; then cfg_branch="$base_branch"; fi
origin_head="$(git -C "<sub_path>" symbolic-ref --short refs/remotes/origin/HEAD 2>/dev/null | sed 's#^origin/##' || true)"
target_branch="${cfg_branch:-${origin_head:-}}"
if [[ -z "${target_branch:-}" ]]; then
  if git -C "<sub_path>" show-ref --verify --quiet refs/heads/main || git -C "<sub_path>" show-ref --verify --quiet refs/remotes/origin/main; then
    target_branch="main"
  else
    target_branch="master"
  fi
fi

# 3) 切到目标分支（若本地无分支则从 origin 跟踪创建）
git -C "<sub_path>" fetch origin
git -C "<sub_path>" switch "$target_branch" || git -C "<sub_path>" switch -c "$target_branch" --track "origin/$target_branch"

# 4) fast-forward 到 gitlink commit（非 ff 直接停止，避免错误合并）
git -C "<sub_path>" status -sb
if git -C "<sub_path>" merge-base --is-ancestor "$sub_sha" HEAD; then
  echo "[skip] <sub_path> already contains $sub_sha"
else
  git -C "<sub_path>" merge --ff-only "$sub_sha"
fi

# 5) push（若无 upstream 则首次 -u）
git -C "<sub_path>" push || git -C "<sub_path>" push -u origin "$target_branch"
```
规则：
- 每个 submodule 必须先执行“切目标分支 + `merge --ff-only <gitlink-sha>`”，再 push；禁止在 detached HEAD 直接 push。
- 若任一 submodule 不能 fast-forward（`merge --ff-only` 失败），立即停止，先人工处理冲突/分叉，再继续。

5) 仅当 submodules 全部成功后，再 push superproject 当前分支：
```bash
git branch --show-current
git status -sb
git push
```
6) （可选）归档变更工件（完成交付后推荐）：
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
