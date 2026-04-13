---
name: ws-pull
description: 拉取并对齐 submodules（避免 detached；减少人为差异）
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

背景：
- Git submodule 在别的电脑 `git submodule update` 后常见现象是子模块处于 detached HEAD（因为主仓库记录的是固定 gitlink commit）。
- detached 本身不一定是错，但会增加“需要手动切分支/容易忘”的操作差异。

目标：
- 安全拉取 superproject（默认只允许 fast-forward）
- 初始化/同步 submodules（`--init --recursive`）
- 在**不改动 superproject gitlink commit** 的前提下，尽量把每个 submodule 从 detached HEAD “挂回”到一个**确定的目标分支**（仅当该分支包含 gitlink commit 时）
- 若未配置 `.gitmodules` 的 `submodule.<name>.branch`：保持 detached，并提示先运行 `$ws-submodule-setup`

安全约束（强制）：
- 不执行破坏性命令（不 `reset --hard` 主仓库；不改动远端）
- 不自动提交/不自动 push
- 若工作区不干净：停止并要求用户先 commit 或 stash（不要自动处理）

执行步骤（建议）：
0) 输出上下文：
```bash
git rev-parse --show-toplevel
git branch --show-current
git status --porcelain
```
若 `git status --porcelain` 非空：停止，要求用户先清理工作区（commit 或 stash）。

1) 拉取 superproject（默认只允许 fast-forward，避免隐式 merge/rebase）：
```bash
git pull --ff-only
```
若失败（需要 merge/rebase）：停止并向用户解释原因，让用户明确选择 `git pull --rebase` 或手动处理。

2) 同步并更新 submodules 到 superproject 记录的 gitlink commit：
```bash
if [[ -f .gitmodules ]]; then
  git submodule sync --recursive
  git submodule update --init --recursive
else
  echo "[info] no .gitmodules"
fi
```

3) （可选但推荐）把 submodule 从 detached HEAD 尽量“挂回分支”，减少手工操作差异：

说明：
- 该步骤不会改变 superproject 的 gitlink commit（也不会让主仓库出现“submodule modified”）。
- 只在目标分支包含该 gitlink commit 时才执行“挂回”；否则保持 detached 并提示原因。
- 为了避免 origin 分支较多时“猜错分支”，本步骤**只在** `.gitmodules` 明确配置了 `submodule.<name>.branch` 时才执行“挂回分支”；否则保持 detached 并提示先运行 `$ws-submodule-setup` 对齐配置。

```bash
if [[ -f .gitmodules ]]; then
  base_branch="$(git branch --show-current)"
  git config --file .gitmodules --get-regexp '^submodule\\..*\\.path$' 2>/dev/null \
    | while read -r key sub_path; do
      [[ -z "${sub_path:-}" ]] && continue
      name="${key#submodule.}"
      name="${name%.path}"
      echo "== submodule: ${sub_path} (${name}) =="

      sub_sha="$(git rev-parse "HEAD:${sub_path}" 2>/dev/null || true)"
      if [[ -z "${sub_sha:-}" ]]; then
        echo "[warn] failed to read gitlink sha for ${sub_path}"
        continue
      fi

      cfg_branch="$(git config --file .gitmodules --get "submodule.${name}.branch" 2>/dev/null || true)"
      if [[ "${cfg_branch:-}" == "." ]]; then cfg_branch="$base_branch"; fi
      if [[ -z "${cfg_branch:-}" ]]; then
        echo "[warn] ${sub_path}: missing .gitmodules submodule.${name}.branch; keep detached (run ws-submodule-setup to set it)"
        continue
      fi

      target_branch="${cfg_branch}"
      pin_branch="aiws/pin/${target_branch}"

      git -C "${sub_path}" fetch origin --prune || true

      if git -C "${sub_path}" show-ref --verify --quiet "refs/remotes/origin/${target_branch}"; then
        if git -C "${sub_path}" merge-base --is-ancestor "${sub_sha}" "origin/${target_branch}"; then
          # 不改动现有分支（避免把已有 main/master 等分支强行指回旧 commit）
          # 仅创建/更新一个 AIWS 专用 pin 分支指向 gitlink commit，从 detached “挂回分支”。
          git -C "${sub_path}" checkout -B "${pin_branch}" "${sub_sha}"
          git -C "${sub_path}" branch --set-upstream-to "origin/${target_branch}" "${pin_branch}" >/dev/null 2>&1 || true
          echo "[ok] attached ${sub_path} to ${pin_branch} (upstream=origin/${target_branch}) at ${sub_sha}"
        else
          echo "[warn] ${sub_path}: ${sub_sha} is not in origin/${target_branch}; keep detached"
        fi
      else
        echo "[warn] ${sub_path}: origin/${target_branch} not found; keep detached"
      fi

      git -C "${sub_path}" rev-parse --abbrev-ref HEAD 2>/dev/null || true
      git -C "${sub_path}" status -sb || true
    done
fi
```

4) 一次性配置（方案 2）：为每个 submodule 写入跟踪分支，便于团队统一（需要用户确认，会改动 `.gitmodules`）：
```bash
# 示例：把某个 submodule 固定跟踪 main（会写入 .gitmodules 的 submodule.<name>.branch）
git submodule set-branch --branch main <sub_path>

# 提示：该变更需要提交到 superproject
git add .gitmodules
git commit -m "chore(submodule): set tracking branch"
```
建议：
- 只有当团队明确希望“子模块按分支滚动”（而不是锁定固定 commit）时，才采用方案 2。
- 若只是想避免 detached 但仍锁定 gitlink commit：优先使用步骤 3（不改 `.gitmodules`）。
- 推荐用 `$ws-submodule-setup` 交互式对齐所有 submodules 的 branch，并提交 `.gitmodules`。

输出要求：
- `Context:` 当前分支 + 是否存在 `.gitmodules` + submodule 列表
- `Result:` pull 是否 fast-forward；每个 submodule 是否成功“挂回分支”，失败原因是什么
