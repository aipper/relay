---
name: ws-finish
description: 收尾（门禁 + 安全合并 + submodule→主仓库顺序 push）
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 在结束一次变更交付时，用 fast-forward 安全合并 `change/<change-id>` 回目标分支，减少手输分支名导致的错误
- 合并成功后，按 `submodule -> superproject` 顺序 push，避免遗漏导致其它仓库拉取异常
- 不自动删分支；AI 入口执行前应先向用户说明将要 push 的 remote/branch，显式传入 `--push` 即视为确认
- 若团队希望减少 submodule detached 的人为差异：建议在 `.gitmodules` 配置 `submodule.<name>.branch`，并在日常拉取时使用 `$ws-pull`

前置（必须）：
- 工作区是干净的：`git status --porcelain` 无输出（若有未提交改动：先 commit 或 stash）
- change 分支已存在：`change/<change-id>`（也支持 `changes/`、`ws/`、`ws-change/`）
- 若使用 worktree：在目标分支所在 worktree 执行（`aiws change finish` 会提示正确的 worktree）
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
2) 若不存在 `.gitmodules`，或 submodules 已按顺序处理完成，优先直接使用最小收尾闭环：
```bash
# 若当前就在 change/<change-id> 分支上，可省略 <change-id>
change_id="<change-id>"
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws change finish "${change_id}" --push
elif command -v aiws >/dev/null 2>&1; then
  aiws change finish "${change_id}" --push
else
  npx @aipper/aiws change finish "${change_id}" --push
fi
```
说明：
- 该命令会在 fast-forward 合并成功后 push 目标分支。
- 默认会优先使用当前分支的 upstream 配置（`branch.<name>.remote` + `branch.<name>.merge`）；只有在你明确知道要推向别的 remote 时，才追加 `--remote <name>`。
- 若 `change/<change-id>` 位于独立 worktree，且该 worktree 干净，则会在 push 成功后自动执行 `git worktree remove` + `git worktree prune`。
- AI 工具内执行前，应先向用户说明将要 push 的 remote/branch；CLI 本身只有在显式传入 `--push` 时才会真正执行 push。

3) 若你需要先处理 submodules，则不要依赖当前分支名推断目标分支，先显式解析 `base_branch`，再执行 submodule 步骤，最后回到主仓库执行 `aiws change finish --push`：
```bash
change_id="<change-id>"
base_branch="$(python3 - <<'PY'
import json, pathlib
change_id = "<change-id>"
meta = pathlib.Path("changes") / change_id / ".ws-change.json"
data = json.loads(meta.read_text(encoding="utf-8"))
print((data.get("base_branch") or "").strip())
PY
)"
test -n "$base_branch"
```
4) 若 fast-forward 失败（提示需要 rebase）：先在 change 分支（或对应 worktree）里 `git rebase <target-branch>`，再重试 `aiws change finish --push`。
5) 若存在 `.gitmodules`，先把每个 submodule 的 gitlink commit 合并回其目标分支（解决 detached HEAD），并按顺序 push：
```bash
# 不要用当前分支名代替目标分支；这里显式使用 .ws-change.json 的 base_branch
change_id="<change-id>"
targets="changes/${change_id}/submodules.targets"

# 子模块清单（没有则跳过）
git config --file .gitmodules --get-regexp '^submodule\..*\.path$' 2>/dev/null || true

# 对每个 submodule.<name>.path <sub_path>：
# 说明：`git submodule update` 会把 submodule checkout 到固定 gitlink commit，导致 detached HEAD。
# 为减少游离状态的协作摩擦，本步骤采用 pin 分支策略：
# - `changes/<change-id>/submodules.targets` 是本次 finish/push 的真值；每个 submodule path 都必须在该文件中声明目标分支（可选 remote）
# - `.gitmodules` 的 `submodule.<name>.branch` 仍建议保留，用于团队默认配置与校验，但不再作为 finish/push 的 fallback
# - 生成/检查 `submodules.targets` 时可显式说明默认来源：
#   - detached HEAD：默认建议取 `.gitmodules` 的 `submodule.<name>.branch`
#   - 已附着在某个本地分支：默认建议取当前分支
#   - 以上仅为预填建议；真正执行 finish/push 时仍只认 `submodules.targets`
# - 不要直接切 `change/<change-id>` / `main` / `master` 等业务分支来解 detached
# - 不改动 submodule 现有分支指针（例如不强行移动 main/master）
# - 创建/更新本地 pin 分支：`aiws/pin/<target_branch>` 指向 gitlink commit，并将其 upstream 设为 `origin/<target_branch>`
if git config --file .gitmodules --get-regexp '^submodule\\..*\\.path$' >/dev/null 2>&1; then
  if [[ ! -f "${targets}" ]]; then
    echo "error: missing ${targets} (required when .gitmodules declares submodules)"
    exit 2
  fi

  source tools/ws_resolve_sub_target.sh

  while read -r key sub_path; do
      name="${key#submodule.}"; name="${name%.path}"
      [[ -z "${sub_path:-}" ]] && continue
      echo "== submodule: ${sub_path} (${name}) =="

      current_branch="$(git -C "${sub_path}" branch --show-current 2>/dev/null || true)"
      declared_branch="$(git config --file .gitmodules --get "submodule.${name}.branch" 2>/dev/null || true)"
      if [[ -n "${current_branch}" ]]; then
        echo "info: ${sub_path} default suggestion would be current branch: ${current_branch}"
      elif [[ -n "${declared_branch}" ]]; then
        echo "info: ${sub_path} default suggestion would be .gitmodules branch: ${declared_branch}"
      else
        echo "warn: ${sub_path} has no current branch and no .gitmodules branch; submodules.targets must be filled manually"
      fi

      sub_sha="$(git rev-parse "HEAD:${sub_path}")"

      ws_resolve_sub_target "${sub_path}" "${name}" "${targets}" "${base_branch}" || exit 2
      target_branch="${_resolved_branch}"
      remote="${_resolved_remote}"
      pin_branch="aiws/pin/${target_branch}"

      git -C "${sub_path}" fetch "${remote}" --prune
      if ! git -C "${sub_path}" show-ref --verify --quiet "refs/remotes/${remote}/${target_branch}"; then
        echo "error: ${sub_path}: missing ${remote}/${target_branch}; refusing to push superproject (would break gitlink fetchability)"
        exit 2
      fi

      # 仅当 gitlink commit 属于 <remote>/<target_branch> 的历史时才挂回分支
      if ! git -C "${sub_path}" merge-base --is-ancestor "${sub_sha}" "${remote}/${target_branch}"; then
        echo "[warn] ${sub_path}: ${sub_sha} is not in ${remote}/${target_branch}; keep detached and stop (need manual reconcile)"
        exit 1
      fi

      git -C "${sub_path}" checkout -B "${pin_branch}" "${sub_sha}"
      git -C "${sub_path}" branch --set-upstream-to "${remote}/${target_branch}" "${pin_branch}" >/dev/null 2>&1 || true
      git -C "${sub_path}" status -sb

      # push：只允许 fast-forward（若远端分叉会被拒绝；此时必须人工处理）
      git -C "${sub_path}" push "${remote}" "${sub_sha}:refs/heads/${target_branch}"
    done < <(git config --file .gitmodules --get-regexp '^submodule\\..*\\.path$' 2>/dev/null || true)
fi
```
规则：
- 每个 submodule 必须先执行 pin 分支挂回 + fast-forward push，再 push 主仓库。
- 若任一 submodule 的 gitlink commit 不在 `<remote>/<target_branch>` 历史中：立即停止，先人工处理分叉，再继续。

6) 仅当 submodules 全部成功后，再在 superproject 当前分支执行最小收尾闭环：
```bash
change_id="<change-id>"
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws change finish "${change_id}" --push
elif command -v aiws >/dev/null 2>&1; then
  aiws change finish "${change_id}" --push
else
  npx @aipper/aiws change finish "${change_id}" --push
fi
```
说明：
- 该命令内部已经包含主仓库 push 成功后安全清理独立 change worktree 的逻辑。

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
