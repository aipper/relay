---
description: 推送：submodule 感知（先 submodules 后 superproject；fast-forward；不 force）
---
<!-- AIWS_MANAGED_BEGIN:opencode:ws-push -->
# ws push

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：安全 push 当前仓库；若仓库包含 submodules，则先 push submodules，再 push superproject（默认 fast-forward；不 force）。

强制约束：
- 不自动提交、不自动 `git add -A`
- 不使用 `--force` / `--force-with-lease`
- 若工作区不干净：停止并要求先 commit 或 stash

步骤（建议）：
1) 输出上下文并检查工作区干净：
```bash
git branch --show-current
git status --porcelain
git status -sb
```
若 `git status --porcelain` 非空：停止。

2) 判断是否存在 submodules：
```bash
if [[ -f .gitmodules ]]; then
  git config --file .gitmodules --get-regexp '^submodule\\..*\\.path$' || true
fi
```

3) 若没有 submodules：正常 push（仍需用户确认远端/分支）：
```bash
git remote -v
git push
```

4) 若有 submodules：先检查 `.gitmodules` 的 `submodule.<name>.branch` 是否齐全（缺失则停止并提示 `/ws-submodule-setup`）：
```bash
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
```

5) 逐个 push submodules（fast-forward only），再 push superproject：
```bash
base_branch="$(git branch --show-current)"

git config --file .gitmodules --get-regexp '^submodule\\..*\\.path$' 2>/dev/null \
  | while read -r key sub_path; do
    name="${key#submodule.}"; name="${name%.path}"
    echo "== submodule: ${sub_path} (${name}) =="

    if [[ -n "$(git -C "${sub_path}" status --porcelain 2>/dev/null || true)" ]]; then
      echo "error: submodule dirty: ${sub_path}"
      exit 2
    fi

    cfg_branch="$(git config --file .gitmodules --get "submodule.${name}.branch" 2>/dev/null || true)"
    if [[ "${cfg_branch:-}" == "." ]]; then cfg_branch="$base_branch"; fi
    target_branch="${cfg_branch}"

    git -C "${sub_path}" fetch origin --prune
    if ! git -C "${sub_path}" show-ref --verify --quiet "refs/remotes/origin/${target_branch}"; then
      echo "error: missing origin/${target_branch} for ${sub_path}"
      exit 2
    fi

    if ! git -C "${sub_path}" merge-base --is-ancestor "origin/${target_branch}" HEAD; then
      echo "error: non-fast-forward (submodule=${sub_path}, branch=${target_branch})"
      exit 2
    fi

    git -C "${sub_path}" push origin "HEAD:refs/heads/${target_branch}"
  done

git remote -v
git push
```
<!-- AIWS_MANAGED_END:opencode:ws-push -->

可在下方追加本项目对 OpenCode 的额外说明（托管块外内容会被保留）。

