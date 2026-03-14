---
name: ws-push
description: 推送（submodule 感知：先 submodules 后 superproject；fast-forward 安全）
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 在不自动提交的前提下，安全 push 当前仓库
- 若仓库包含 submodules：按 `submodules -> superproject` 顺序 push，减少“别人拉取后子模块 detached/分叉”的协作摩擦
- 若不包含 submodules：按普通 git 仓库的规则 push

安全约束（强制）：
- 不自动提交、不自动 `git add -A`
- 不使用 `--force` / `--force-with-lease`
- 默认只允许 fast-forward push（发现分叉则停止并提示人工处理）
- 若工作区不干净：停止并要求先 commit 或 stash

执行步骤（建议）：
0) 输出上下文：
```bash
git branch --show-current
git status --porcelain
git status -sb
```
若 `git status --porcelain` 非空：停止。

1) 判断是否存在 submodules：
```bash
if [[ -f .gitmodules ]]; then
  git config --file .gitmodules --get-regexp '^submodule\\..*\\.path$' || true
else
  echo "[info] no .gitmodules"
fi
```

2) 若不存在 `.gitmodules` 或没有 submodule 条目：按普通仓库 push（仍需用户确认）：
```bash
git remote -v
git push
```

3) 若存在 submodules：先检查 `.gitmodules` 的 branch 真值是否齐全（缺失则停止并提示 setup）：
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
  echo "hint: run $ws-submodule-setup (and commit .gitmodules), then retry"
  exit 2
fi
```

4) 逐个 push submodules（fast-forward only），再 push superproject：
```bash
base_branch="$(git branch --show-current)"

git config --file .gitmodules --get-regexp '^submodule\\..*\\.path$' 2>/dev/null \
  | while read -r key sub_path; do
    name="${key#submodule.}"; name="${name%.path}"
    echo "== submodule: ${sub_path} (${name}) =="

    # submodule 工作区必须干净
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

    # fast-forward only: origin/<branch> 必须是 HEAD 的祖先
    if ! git -C "${sub_path}" merge-base --is-ancestor "origin/${target_branch}" HEAD; then
      echo "error: non-fast-forward (submodule=${sub_path}, branch=${target_branch})"
      echo "hint: rebase/merge in submodule, then retry"
      exit 2
    fi

    # push HEAD -> origin/<branch>（不 force）
    git -C "${sub_path}" push origin "HEAD:refs/heads/${target_branch}"
  done

# 最后 push superproject（仍需用户确认远端/分支）
git remote -v
git push
```

输出要求：
- `Context:` 当前分支 + 是否有 submodules
- `Submodules:` 每个 submodule push 的目标分支与结果（成功/阻断原因）
- `Superproject:` push 结果

