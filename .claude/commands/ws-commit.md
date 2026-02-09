<!-- AIWS_MANAGED_BEGIN:claude:ws-commit -->
# ws commit

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：在**当前分支可直提**的前提下，先做审计与门禁，并在存在 submodule 时给出正确的提交顺序（先 submodule，再 superproject），最后执行 `git commit`。

安全约束（强制）：
- 不自动 `git add -A`；只在用户明确指示时才做 staging
- 不使用 `--no-verify` 绕过 hooks
- 不自动 push
- 不打印 secrets

步骤（建议）：
1) 先运行 `/ws-preflight`。
2) 运行 `/ws-review`（落盘审计证据）。
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
4) 输出提交上下文（必须输出给用户确认）：
```bash
git branch --show-current
git status --porcelain
```
5) submodule 感知检查：
```bash
if [[ -f .gitmodules ]]; then
  git config --file .gitmodules --get-regexp '^submodule\\..*\\.path$' || true
fi
while read -r _ sub_path; do
  [[ -z "${sub_path:-}" ]] && continue
  echo "== submodule: ${sub_path} =="
  git -C "${sub_path}" rev-parse --abbrev-ref HEAD 2>/dev/null || true
  git -C "${sub_path}" status --porcelain || true
done < <(git config --file .gitmodules --get-regexp '^submodule\\..*\\.path$' 2>/dev/null || true)
```
判定规则（强制）：
- 任一 submodule `git status --porcelain` 非空：停止提交 superproject；先在对应 submodule 完成 commit（必要时先切回可提交分支），再回到 superproject 更新并提交 gitlink。
6) 检查 staging（必须输出给用户确认）：
```bash
git status --porcelain
git diff --staged --submodule=short
```
7) 若没有 staged changes：停止并提示用户先明确要提交哪些文件（例如 `git add -p` 或 `git add <path>`）。
8) 让用户提供 commit message（必须确认后再执行）。
9) 执行提交（不带 `--no-verify`）：
```bash
git commit -m "<message>"
```

输出必须包含：
- `Evidence:` `.agentdocs/tmp/review/*` + `.agentdocs/tmp/aiws-validate/*`
- `Context:` 当前分支 + 是否检测到 submodule + 若阻断则给出阻断原因
- `Next:` 若存在 submodule 改动，先提示用户进入 submodule 提交
<!-- AIWS_MANAGED_END:claude:ws-commit -->

可在下方追加本项目对 Claude Code 的额外说明（托管块外内容会被保留）。
