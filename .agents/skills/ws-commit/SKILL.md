---
name: ws-commit
description: 提交（当前分支可直提；submodule 感知；先审计/门禁再 commit）
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 支持在**当前分支直接提交**（不要求必须先切 `change/<change-id>`）
- 提交前审计与证据落盘（`$ws-review`）
- 提交前门禁校验与证据落盘（`aiws validate . --stamp`）
- 最后执行 `git commit`（commit 前必须让用户确认 message；不使用 `--no-verify` 绕过 hooks）
- 若仓库含 submodule：提交前识别并提示正确顺序（先 submodule，再 superproject）

安全约束（强制）：
- 不自动 `git add -A`（避免误提交）；只在用户明确指示时才执行 staging 命令
- 不自动 push
- 不写入任何 secrets
- 检测到 submodule 有未提交改动时，不允许直接提交 superproject（先处理 submodule）

执行步骤（建议）：
1) 运行 `$ws-preflight`（确保真值文件就绪）。
2) 运行 `$ws-review`（生成审计证据：`.agentdocs/tmp/review/codex-review.md`）。
3) 运行门禁校验并写 stamp：
```bash
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws validate . --stamp
elif command -v aiws >/dev/null 2>&1; then
  aiws validate . --stamp
else
  npx @aipper/aiws validate . --stamp
fi
```
4) 输出当前提交上下文（必须输出给用户确认）：
```bash
git branch --show-current
git status --porcelain
```
5) 检测是否存在 submodule（有则进入 submodule 感知模式）：
```bash
if [[ -f .gitmodules ]]; then
  git config --file .gitmodules --get-regexp '^submodule\..*\.path$' || true
else
  echo "[info] no .gitmodules"
fi
```
6) 若存在 submodule，逐个检查子仓库工作区是否干净：
```bash
while read -r _ sub_path; do
  [[ -z "${sub_path:-}" ]] && continue
  echo "== submodule: ${sub_path} =="
  git -C "${sub_path}" rev-parse --abbrev-ref HEAD 2>/dev/null || true
  git -C "${sub_path}" status --porcelain || true
done < <(git config --file .gitmodules --get-regexp '^submodule\..*\.path$' 2>/dev/null || true)
```
判定规则（强制）：
- 任一 submodule `git status --porcelain` 非空：停止 superproject commit，先在对应 submodule 完成 commit（必要时先切回可提交分支），再回到 superproject 更新并提交 gitlink。
7) 检查当前 staging 内容（必须输出给用户确认）：
```bash
git status --porcelain
git diff --staged --submodule=short
```
8) 若没有 staged changes：停止并提示用户先明确要提交哪些文件（例如 `git add -p` 或 `git add <path>`）。
9) 让用户提供 commit message（必须确认后再执行）。
10) 执行提交（不带 `--no-verify`）：
```bash
git commit -m "<message>"
```
11) 输出提交结果（可选）：
```bash
git show --stat --oneline -1
```

输出要求：
- `Evidence:` `.agentdocs/tmp/review/codex-review.md` + `.agentdocs/tmp/aiws-validate/*.json`
- `Context:` 当前分支 + 是否检测到 submodule + 若阻断则给出阻断原因
- `Commit:` 最终使用的 commit message（仅当用户确认后）
