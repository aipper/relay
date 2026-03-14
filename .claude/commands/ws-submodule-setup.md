<!-- AIWS_MANAGED_BEGIN:claude:ws-submodule-setup -->
# ws submodule setup

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：为每个 submodule 写入 `.gitmodules` 的 `submodule.<name>.branch`（团队真值），让 `/ws-pull`、`/ws-finish` 能确定性地减少 detached 与人为差异；并使 `aiws validate` 的 submodule 分支门禁通过。

约束：
- 不自动提交、不自动 push（必须先输出 diff 并让用户确认）
- 不做破坏性命令

步骤（建议）：
1) 确认工作区干净：
```bash
git status --porcelain
```
非空则停止。

2) 列出 submodules：
```bash
test -f .gitmodules || { echo "no .gitmodules"; exit 0; }
git config --file .gitmodules --get-regexp '^submodule\\..*\\.path$'
```

3) 对每个 submodule 让用户选择目标分支（推荐具体分支名；可选 `.` 表示跟随 superproject 分支名）：
```bash
while read -r key sub_path; do
  name="${key#submodule.}"; name="${name%.path}"
  echo "== submodule: ${name} path=${sub_path} =="
  echo "[current] branch=$(git config --file .gitmodules --get submodule.${name}.branch || true)"
  echo "[origin]  HEAD=$(git -C \"${sub_path}\" symbolic-ref --short refs/remotes/origin/HEAD 2>/dev/null || true)"
done < <(git config --file .gitmodules --get-regexp '^submodule\\..*\\.path$')
```

4) 写入分支配置：
```bash
git submodule set-branch --branch <branch-or-dot> <sub_path>
```

5) 输出 diff 并让用户确认是否提交：
```bash
git diff -- .gitmodules
git status --porcelain
```

6) 用户确认后提交：
```bash
git add .gitmodules
git commit -m "chore(submodule): set tracking branches"
```
<!-- AIWS_MANAGED_END:claude:ws-submodule-setup -->

可在下方追加本项目对 Claude Code 的额外说明（托管块外内容会被保留）。

