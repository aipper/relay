---
description: 拉取：fast-forward 拉取并对齐 submodules（尽量避免 detached）
---
<!-- AIWS_MANAGED_BEGIN:opencode:ws-pull -->
# ws pull

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：安全拉取 superproject，并对齐 submodules，尽量把 submodule 从 detached HEAD “挂回”分支（仅当该分支包含 gitlink commit），减少人为操作差异。

安全约束（强制）：
- 不执行破坏性命令；不自动提交/不自动 push
- 若工作区不干净：停止并要求用户先 commit 或 stash
- `git pull` 默认只允许 fast-forward（避免隐式 merge/rebase）

步骤（建议）：
1) 输出上下文并检查工作区干净：
```bash
git rev-parse --show-toplevel
git branch --show-current
git status --porcelain
```
若 `git status --porcelain` 非空：停止。

2) 拉取 superproject（fast-forward only）：
```bash
git pull --ff-only
```
若失败：停止并说明需要用户明确选择 `git pull --rebase` 或手动处理。

3) 更新 submodules 到 superproject 记录的 gitlink commit：
```bash
if [[ -f .gitmodules ]]; then
  git submodule sync --recursive
  git submodule update --init --recursive
else
  echo "[info] no .gitmodules"
fi
```

4) （可选但推荐）把 submodule 从 detached HEAD 尽量“挂回分支”（不改动 gitlink commit）：
为避免 origin 多分支时“猜错分支”，本步骤只在 `.gitmodules` 明确配置了 `submodule.<name>.branch` 时才执行；否则保持 detached 并提示先运行 `/ws-submodule-setup` 对齐配置。
```bash
if [[ -f .gitmodules ]]; then
  base_branch="$(git branch --show-current)"
  git config --file .gitmodules --get-regexp '^submodule\\..*\\.path$' 2>/dev/null \
    | while read -r key sub_path; do
      [[ -z "${sub_path:-}" ]] && continue
      name="${key#submodule.}"; name="${name%.path}"
      echo "== submodule: ${sub_path} (${name}) =="
      sub_sha="$(git rev-parse "HEAD:${sub_path}" 2>/dev/null || true)"
      [[ -z "${sub_sha:-}" ]] && { echo "[warn] no gitlink sha"; continue; }

      cfg_branch="$(git config --file .gitmodules --get "submodule.${name}.branch" 2>/dev/null || true)"
      if [[ "${cfg_branch:-}" == "." ]]; then cfg_branch="$base_branch"; fi
      if [[ -z "${cfg_branch:-}" ]]; then
        echo "[warn] ${sub_path}: missing .gitmodules submodule.${name}.branch; keep detached (run ws-submodule-setup)"
        continue
      fi
      target_branch="${cfg_branch}"
      pin_branch="aiws/pin/${target_branch}"

      git -C "${sub_path}" fetch origin --prune || true
      if git -C "${sub_path}" show-ref --verify --quiet "refs/remotes/origin/${target_branch}"; then
        if git -C "${sub_path}" merge-base --is-ancestor "${sub_sha}" "origin/${target_branch}"; then
          git -C "${sub_path}" checkout -B "${pin_branch}" "${sub_sha}"
          git -C "${sub_path}" branch --set-upstream-to "origin/${target_branch}" "${pin_branch}" >/dev/null 2>&1 || true
          echo "[ok] attached ${sub_path} to ${pin_branch} (upstream=origin/${target_branch}) at ${sub_sha}"
        else
          echo "[warn] ${sub_path}: ${sub_sha} is not in origin/${target_branch}; keep detached"
        fi
      else
        echo "[warn] ${sub_path}: origin/${target_branch} not found; keep detached"
      fi
    done
fi
```

可选（方案 2，一次性设置 submodule 跟踪分支，会改 `.gitmodules`，需提交）：
```bash
git submodule set-branch --branch main <sub_path>
git add .gitmodules
git commit -m "chore(submodule): set tracking branch"
```
<!-- AIWS_MANAGED_END:opencode:ws-pull -->

可在下方追加本项目对 OpenCode 的额外说明（托管块外内容会被保留）。
