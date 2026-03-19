---
name: ws-dev
description: 开发（按需求实现并验证；适用于任何需要修改代码/配置的任务）
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：在 AIWS 约束下完成一个可回放、可验证的小步交付。

建议流程：
1) 先做 preflight：定位项目根目录，读取 `AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`，输出约束摘要。
   - 若是中大型任务：建议先用 `$ws-plan` 生成 `plan/` 工件，再进入实现（便于可回放与对齐验证入口）。
   - 若已有 `plan/` 工件：先执行 `$ws-plan-verify`；通过后再进入实现（防止计划过长/跑偏）。
2) 建立变更归因（推荐）：
   - ⚠️ 开始前先确认工作区干净：`git status --porcelain` 为空；否则切分支/创建 worktree 后，未提交改动可能“看起来丢了”（worktree 只从 `HEAD` checkout，未提交内容会留在原目录）。
   - 强制（当你准备执行 `aiws change start ... --worktree/--switch` 创建新 change 时）：先同步线上代码（含 submodules），避免基线过旧导致的 rebase/冲突与“别人已更新但本地没拉”的协作摩擦。
     - 若你已经在 `change/<change-id>` 上继续开发：不要在此处强制 pull（避免把远端变动拉进变更分支）；改为按需 `git fetch`/rebase，并保持门禁与验证可复现。
     - 在 AI 工具中运行：`$ws-pull`（推荐；会在工作区不干净时阻断）
     - 或等价手工（必须工作区干净；失败则停止并人工处理）：
```bash
cur="$(git branch --show-current)"
if [[ "${cur}" =~ ^(change|changes|ws|ws-change)/ ]]; then
  echo "info: already on change branch (${cur}); skip pull here"
else
  git status --porcelain
  git pull --ff-only
  if [[ -f .gitmodules ]]; then
    git submodule sync --recursive
    git submodule update --init --recursive
  fi
fi
```
   - 推荐更安全（默认）：`aiws change start <change-id> --hooks --no-switch`（只创建分支/工件 + 启用 hooks；不切分支）
   - 准备进入实现（且工作区干净）后再切换：`git switch change/<change-id>`
   - 若你明确要“一键切分支”（不推荐，且 dirty 会被拦截）：`aiws change start <change-id> --hooks --switch`
   - superproject + submodule（推荐）：`aiws change start <change-id> --hooks --worktree --submodules`（创建独立 worktree；当前目录分支保持不变；会在新 worktree 内初始化 submodules；若忘了 `--submodules` 也会强制初始化）
   - 若后续需要在 detached submodule 内提交：先挂到 `aiws/pin/<target-branch>`；不要直接切 `change/<change-id>` / `main` / `master`
   - 若仓库存在 submodule（`.gitmodules` 声明了 submodule 条目）：进入编码前必须准备好 `changes/<change-id>/submodules.targets`，并把每个 submodule 挂到对应的 `aiws/pin/<target_branch>`（必要时切到 `<remote>/<target_branch>`；这可能会改变 superproject 的 gitlink 指针，属于预期的“选渠道”行为；缺失该文件会被门禁阻断）
```bash
change_id="<change-id>"
targets="changes/${change_id}/submodules.targets"
has_submodules=0
if [[ -f .gitmodules ]]; then
  if git config --file .gitmodules --get-regexp '^submodule\\..*\\.path$' >/dev/null 2>&1; then
    has_submodules=1
  fi
fi
if [[ "${has_submodules}" -eq 1 && ! -f "${targets}" ]]; then
  echo "error: missing ${targets} (required when .gitmodules declares submodules)"
  exit 2
fi

if [[ -f "${targets}" ]]; then
  echo "info: applying submodule targets: ${targets}"
  while read -r sub_path target_branch remote; do
    [[ -z "${sub_path:-}" ]] && continue
    [[ "${sub_path}" == \#* ]] && continue
    [[ -z "${target_branch:-}" ]] && { echo "error: missing target_branch for path=${sub_path}"; exit 2; }
    remote="${remote:-origin}"

    echo "== submodule: ${sub_path} target=${remote}/${target_branch} =="
    if [[ -n "$(git -C "${sub_path}" status --porcelain 2>/dev/null)" ]]; then
      echo "error: submodule dirty: ${sub_path} (commit/stash first)"
      exit 2
    fi
    git -C "${sub_path}" fetch "${remote}" --prune
    # 选渠道：默认切到远端目标分支，并用 pin 分支承载本地提交（避免 detached）
    git -C "${sub_path}" checkout -B "aiws/pin/${target_branch}" "${remote}/${target_branch}"
    git -C "${sub_path}" branch --set-upstream-to "${remote}/${target_branch}" "aiws/pin/${target_branch}" >/dev/null 2>&1 || true
    git -C "${sub_path}" status -sb
  done < "${targets}"

  # 检查 superproject 的 gitlink 是否发生变化（预期：若切了不同渠道，会看到差异）
  git diff --submodule
fi
```
   - 若你明确要在 superproject 直接切分支：`aiws change start <change-id> --hooks --switch`（仅在存在 `.gitmodules` 时有意义；会尝试让 submodules 工作区跟随 superproject 指针）
   - 或手工：`git switch -c change/<change-id>`，并创建 `changes/<change-id>/proposal.md` 与 `changes/<change-id>/tasks.md`（参考 `changes/README.md`）
3) 如涉及需求调整：先做需求评审（可用 `$ws-req-review`）→ 用户确认后再做需求落盘（可用 `$ws-req-change`）（避免需求漂移）。
4) 实施最小改动：任何改动都要能归因到 `REQUIREMENTS.md`（验收）或 `issues/problem-issues.csv`（问题）。
5) 运行 `AI_WORKSPACE.md` 里声明的验证命令；未运行不声称已运行。
6) 多步任务（≥2 步）：使用 `update_plan` 工具跟踪 `pending → in_progress → completed`，每完成一步立即更新（不要事后批量更新）。
7) 提交前强制门禁（commit/push hooks 也会阻断）：
```bash
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws validate .
elif command -v aiws >/dev/null 2>&1; then
  aiws validate .
else
  npx @aipper/aiws validate .
fi
```
8) 交付收尾（推荐，减少手动 merge 出错）：运行 `$ws-finish`（底层调用 `aiws change finish`，默认 fast-forward 安全合并回目标分支）。

输出要求：
- `变更文件（Changed）:` 文件清单
- `验证（Verify）:` 实际运行的命令 + 期望结果
- `证据（Evidence）:` 证据路径（例如 `changes/<change-id>/review/...`、`plan/...`、`.agentdocs/tmp/aiws-validate/...` 或 `changes/<change-id>/...`）
