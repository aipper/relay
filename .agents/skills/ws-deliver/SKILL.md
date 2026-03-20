---
name: ws-deliver
description: 交付（submodules + superproject 分步提交，并安全合并回目标分支）
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 适配 superproject + submodule（数量不固定）的交付收尾：
  1) 先逐个提交 submodule（每个 repo 单独确认 commit message）
  2) 再提交 superproject（包含 submodule gitlink 指针更新 + 变更工件/代码）
  3) 最后 fast-forward 合并回目标分支（复用 `aiws change finish`，减少手动 merge 出错）

非目标（强制）：
- 不自动 `git add -A`（避免误提交）
- 不自动 push
- 不自动删除分支

前置（强制）：
1) 先运行 `$ws-preflight`。
2) 确认当前处于 change 分支（推荐）：`change/<change-id>`（也支持 `changes/`、`ws/`、`ws-change/`）。
   - 若不在 change 分支：要求用户先切换到 `change/<change-id>`（或在命令里显式提供 `<change-id>`）。
3) 任何自动提交都必须在提交前输出：
   - 该 repo 的 `git status --porcelain`
   - 该 repo 的 `git diff --staged`
   并让用户确认 commit message（每个 repo 单独确认）。

建议流程（按顺序）：

## 0) submodule branch 真值检查（减少 detached 与人为差异）
如果存在 `.gitmodules` 但缺少 `submodule.<name>.branch`，先运行 `$ws-submodule-setup` 并提交 `.gitmodules`，否则后续 `aiws validate .` 会失败，且 `ws-pull/ws-finish` 无法确定性工作。
> 说明：若同一 superproject 分支内存在多渠道 submodule 目标分支的交付需求，可在 `changes/<change-id>/submodules.targets` 额外声明本次 change 的目标分支；交付时会优先使用该文件（不改变 `.gitmodules` 的团队默认真值）。
> 生成该文件时，可以按当前状态做默认预填，但必须显式说明来源并让用户确认：detached HEAD 默认建议取 `.gitmodules` 声明分支；已附着在某个本地分支时默认建议取当前分支；finish/push 最终只认 `submodules.targets`。
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

  # 强约束：当 .gitmodules 声明 submodules 时，要求本次 change 存在 submodules.targets 且覆盖所有 submodule path
  if git config --file .gitmodules --get-regexp '^submodule\\..*\\.path$' >/dev/null 2>&1; then
    change_id="$(git branch --show-current | sed -n 's|^change/||p')"
    targets="changes/${change_id}/submodules.targets"
    if [[ ! -f "${targets}" ]]; then
      echo "error: missing ${targets} (required when .gitmodules declares submodules)"
      exit 2
    fi
    t_missing=0
    while read -r _ sub_path; do
      [[ -z "${sub_path:-}" ]] && continue
      if ! awk -v p="${sub_path}" '$1==p && $0 !~ /^[[:space:]]*#/ && $2!="" { found=1 } END { exit(found?0:1) }' "${targets}" 2>/dev/null; then
        echo "error: ${targets} missing entry for submodule path=${sub_path}"
        t_missing=1
      fi
    done < <(git config --file .gitmodules --get-regexp '^submodule\\..*\\.path$' 2>/dev/null || true)
    if [[ "${t_missing}" -ne 0 ]]; then
      echo "hint: fill ${targets} as: <path> <target_branch> [remote]"
      exit 2
    fi
  fi
fi
```

## A) 发现 submodules 清单（数量不固定）
在 superproject 根目录执行：
```bash
git submodule status --recursive
```
如果没有 submodule：跳到 C)。

## B) 逐个提交 submodules（先 submodule，后 superproject）
对每个 submodule path（可递归）重复以下步骤（建议按 `git submodule status --recursive` 顺序）：
1) 定位并检查状态：
```bash
sub_path="<path>"
git -C "$sub_path" branch --show-current
git -C "$sub_path" status --porcelain
```
2) 先确定该 submodule 的目标分支来源，并显式说明给用户：
   - `branch --show-current` 非空：默认建议用当前分支
   - `branch --show-current` 为空（detached HEAD）：默认建议用 `.gitmodules` 的 `submodule.<name>.branch`
   - 若建议值与 `changes/<change-id>/submodules.targets` 已落盘值不一致：以 `submodules.targets` 为准，并先提示差异
3) 若 submodule 处于 detached HEAD（`branch --show-current` 为空）：
   - 说明：这通常是因为 superproject 的 gitlink checkout（例如 `git submodule update`）导致 detached。
   - 不要直接切 `change/<change-id>` / `main` / `master` 来解 detached。
   - 若你要在该 submodule 里提交：先按目标分支挂到 pin 分支 `aiws/pin/<target-branch>`，再在其上提交：
     - 目标分支真值优先级：`changes/<change-id>/submodules.targets`（若存在）> `.gitmodules submodule.<name>.branch`
```bash
change_id="<change-id>"
targets="changes/${change_id}/submodules.targets"
sub_name="$(git config --file .gitmodules --get-regexp '^submodule\\..*\\.path$' 2>/dev/null | awk -v p="${sub_path}" '$2==p { name=$1; sub(/^submodule\\./,"",name); sub(/\\.path$/,"",name); print name; exit }')"
base_branch="$(python3 - <<'PY'
import json, pathlib
change_id = "<change-id>"
meta = pathlib.Path("changes") / change_id / ".ws-change.json"
data = json.loads(meta.read_text(encoding="utf-8"))
print((data.get("base_branch") or "").strip())
PY
)"
test -n "${sub_name}"
test -n "${base_branch}"

source tools/ws_resolve_sub_target.sh
ws_resolve_sub_target "${sub_path}" "${sub_name}" "${targets}" "${base_branch}" || exit 2
target_branch="${_resolved_branch}"
remote="${_resolved_remote}"

git -C "$sub_path" fetch "${remote}" --prune
if ! git -C "$sub_path" show-ref --verify --quiet "refs/remotes/${remote}/${target_branch}"; then
  echo "error: missing ${remote}/${target_branch} for submodule path=${sub_path}"
  exit 2
fi
git -C "$sub_path" checkout -B "aiws/pin/${target_branch}" HEAD
git -C "$sub_path" branch --set-upstream-to "${remote}/${target_branch}" "aiws/pin/${target_branch}" >/dev/null 2>&1 || true
```
   - 若 `origin/<target-branch>` 不存在，或用户明确不想使用 pin 分支：停止，解释风险（提交可能不可追溯/难以推送）。
4) 选择性 staging（默认用 `-p` 更安全）：
```bash
git -C "$sub_path" add -p
git -C "$sub_path" diff --staged --stat
git -C "$sub_path" diff --staged
```
5) AI 生成该 submodule 的 commit message（标题+可选 body），并让用户确认（每个 repo 单独确认）。
6) 执行提交（不带 `--no-verify`）：
```bash
git -C "$sub_path" commit -m "<message>"
```
7) 若该 submodule 没有 staged changes：跳过（不要硬提交空 commit）。

## C) 提交 superproject（更新 gitlinks + 自身改动 + changes 工件）
1) 先检查 submodule 指针差异（gitlinks）：
```bash
git diff --submodule
```
2) 选择性 staging：
   - 先 stage 发生指针变化的 submodule 路径（明确列出）：
```bash
git add <submodule-path-1> <submodule-path-2>
```
   - 再 stage superproject 自身改动（默认 `-p`）：
```bash
git add -p
git diff --staged --stat
git diff --staged
```
3) AI 生成 superproject 的 commit message（应包含 `bump submodule <name> -> <sha>` 等关键信息），并让用户确认。
4) 提交：
```bash
git commit -m "<message>"
```

## D) 门禁与证据（推荐）
```bash
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws validate . --stamp
elif command -v aiws >/dev/null 2>&1; then
  aiws validate . --stamp
else
  npx @aipper/aiws validate . --stamp
fi
```

## D2) 生成持久证据并回填 Evidence_Path（强烈建议）
> 说明：`.agentdocs/tmp/...` 默认 gitignored；交付前建议把关键结果落到 `changes/<change-id>/evidence/...` 并回填 `proposal.md`/`plan` 的 `Evidence_Path`，避免后续评审/二次会话读不到证据。
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

## D3) 生成状态快照（可选，建议）
```bash
aiws change state "${change_id}" --write
```

## E) 安全合并回目标分支（fast-forward）
优先使用 `$ws-finish`（底层调用 `aiws change finish`，并在 push 成功后清理对应 change worktree）。

若需要显式指定目标分支：
```bash
aiws change finish <change-id> --into <base-branch>
```

## F) （可选）归档变更工件
```bash
aiws change archive <change-id>
```

输出要求：
- `Submodules:` 每个 submodule 的分支/提交摘要（repo → commit sha → message）
- `Superproject:` 提交摘要
- `Merge:` `aiws change finish` 的输出（into/from）
- `Worktree cleanup:` 若存在独立 change worktree，输出清理结果（removed/skipped + reason）
- `Evidence:` `.agentdocs/tmp/aiws-validate/*.json`（若使用 --stamp）
