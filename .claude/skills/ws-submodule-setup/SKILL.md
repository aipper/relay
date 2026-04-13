---
name: ws-submodule-setup
description: 子模块分支对齐（写入 .gitmodules 的 submodule.<name>.branch；减少 detached 与人为差异）
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 为每个 submodule 写入 `.gitmodules` 的 `submodule.<name>.branch`，让 `ws-pull` / `ws-finish` 能确定性地“挂回分支/fast-forward push”，避免 origin 多分支时靠猜导致偏差。
- 该变更是 **superproject 的团队真值**：需要提交 `.gitmodules`。

安全约束（强制）：
- 不自动提交、不自动 push（必须先输出 diff 并让用户确认）
- 不在 submodule 中做破坏性操作（不 `reset --hard` / 不改动远端）

步骤（建议）：
1) 确认工作区干净（否则停止）：
```bash
git status --porcelain
```

2) 列出 submodules（没有则停止并说明无需配置）：
```bash
test -f .gitmodules || { echo "no .gitmodules"; exit 0; }
git config --file .gitmodules --get-regexp '^submodule\\..*\\.path$'
```

3) 对每个 submodule，输出当前配置与建议分支（让用户确认每个 submodule 的 branch）：
```bash
while read -r key sub_path; do
  name="${key#submodule.}"; name="${name%.path}"
  echo "== submodule: ${name} path=${sub_path} =="
  echo "[current] branch=$(git config --file .gitmodules --get submodule.${name}.branch || true)"
  echo "[origin]  HEAD=$(git -C \"${sub_path}\" symbolic-ref --short refs/remotes/origin/HEAD 2>/dev/null || true)"
  git -C "${sub_path}" branch -r --list "origin/*" | sed -n '1,30p' || true
  echo "[choose] set one of:"
  echo "  - a concrete branch, e.g. main / master / release/x.y"
  echo "  - '.' to follow superproject current branch name (only if your team uses matching branch names)"
done < <(git config --file .gitmodules --get-regexp '^submodule\\..*\\.path$')
```

4) 逐个写入分支配置（每次写完都回显，避免误配）：
```bash
# Example:
# git submodule set-branch --branch main path/to/submodule
# git submodule set-branch --branch .    path/to/submodule
```

5) 输出变更并让用户确认是否提交：
```bash
git diff -- .gitmodules
git status --porcelain
```

6) 若用户确认要提交：
```bash
git add .gitmodules
git commit -m "chore(submodule): set tracking branches"
```

输出要求：
- `Submodules:` name/path + 选择的 branch（每个都列出）
- `Diff:` `.gitmodules` 的 diff（或至少 `git diff -- .gitmodules` 的摘要）
- `Next:` 提示后续用 `$ws-pull` 拉取可自动减少 detached；`aiws validate` 会检查该配置是否齐全

