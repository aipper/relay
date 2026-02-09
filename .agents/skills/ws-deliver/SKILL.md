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
2) 若 submodule 处于 detached HEAD（`branch --show-current` 为空）：
   - 默认建议：在该 submodule 内创建并切到同名 change 分支（与 superproject 对齐），例如 `change/<change-id>`：
```bash
git -C "$sub_path" switch -c "change/<change-id>"
```
   - 若用户明确不想建分支：停止，解释风险（提交可能不可追溯/难以推送）。
3) 选择性 staging（默认用 `-p` 更安全）：
```bash
git -C "$sub_path" add -p
git -C "$sub_path" diff --staged --stat
git -C "$sub_path" diff --staged
```
4) AI 生成该 submodule 的 commit message（标题+可选 body），并让用户确认（每个 repo 单独确认）。
5) 执行提交（不带 `--no-verify`）：
```bash
git -C "$sub_path" commit -m "<message>"
```
6) 若该 submodule 没有 staged changes：跳过（不要硬提交空 commit）。

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
3) AI 生成 superproject 的 commit message（应包含 “bump submodule <name> -> <sha>” 等关键信息），并让用户确认。
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

## E) 安全合并回目标分支（fast-forward）
优先使用 `$ws-finish`（底层调用 `aiws change finish`）。

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
- `Evidence:` `.agentdocs/tmp/aiws-validate/*.json`（若使用 --stamp）
