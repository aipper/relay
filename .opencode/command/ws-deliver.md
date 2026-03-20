---
description: 交付：submodules+superproject 分步提交并安全合并回 base
---
<!-- AIWS_MANAGED_BEGIN:opencode:ws-deliver -->
# ws deliver

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：适配 superproject + submodule（数量不固定）的交付收尾，降低提交顺序和合并回 base 分支的出错概率：
1) 先逐个提交 submodule（每个 repo 单独确认 commit message；默认 `git add -p`）
2) 再提交 superproject（包含 submodule gitlink 指针更新 + 自身改动/变更工件）
3) 最后 fast-forward 合并回目标分支（复用 `aiws change finish`；建议用 `/ws-finish`）

强制约束：
- 不自动 `git add -A`。
- 不自动 push。
- 不自动删除分支。

建议流程（按顺序）：
1) 先运行 `/ws-preflight`。
2) 如果存在 `.gitmodules` 但缺少 `submodule.<name>.branch`，先运行 `/ws-submodule-setup` 并提交 `.gitmodules`（否则 `aiws validate .` 会失败，且 submodule 工作流会产生人为差异）。
2.1) 若存在 submodule：必须准备 `changes/<change-id>/submodules.targets`，并覆盖所有 submodule path；这是本次交付的目标分支真值。
   - 生成该文件时可先做默认预填：detached HEAD 默认建议取 `.gitmodules` 声明分支；已附着在本地分支时默认建议取当前分支。
   - 上述都只是建议值；真正的 deliver/finish 只认 `submodules.targets`。
2) 发现 submodules：
   - `git submodule status --recursive`
3) 逐个提交 submodules（按上一步顺序）：
   - `git -C "<sub_path>" status --porcelain`
   - 先说明该 submodule 目标分支的来源：attached branch 默认建议取当前分支；detached HEAD 默认建议取 `.gitmodules`；若与 `submodules.targets` 已落盘值冲突，则以 `submodules.targets` 为准
   - 若当前为 detached HEAD：不要直接切 `change/<change-id>` / `main` / `master`；先按 `submodules.targets`（若分支为 `.` 则展开为 `.ws-change.json` 的 `base_branch`）挂到 `aiws/pin/<target-branch>`
   - `git -C "<sub_path>" add -p`
   - `git -C "<sub_path>" diff --staged --stat`
   - 生成并让用户确认该 submodule 的 commit message（每个 repo 单独确认）
   - `git -C "<sub_path>" commit -m "<message>"`
4) 提交 superproject（gitlinks + 自身改动）：
   - `git diff --submodule`
   - `git add <submodule-path-1> <submodule-path-2> ...`
   - `git add -p`
   - 生成并让用户确认 superproject 的 commit message
   - `git commit -m "<message>"`
5) （推荐）门禁 + 证据：
   - `aiws validate . --stamp`（未安装全局 aiws 时可用 `npx @aipper/aiws validate . --stamp`）
5.1) （强烈建议）生成持久证据并回填 `Evidence_Path`：
   - `aiws change evidence <change-id>`（未安装全局 aiws 时可用 `npx @aipper/aiws change evidence <change-id>`）
5.2) （可选）生成状态快照（建议）：
   - `aiws change state <change-id> --write`
6) 安全合并回目标分支：
   - 优先运行 `/ws-finish`（底层调用 `aiws change finish`，默认 `--ff-only`；push 成功后会清理对应 change worktree）

输出要求：
- `Submodules:` 每个 submodule 的 commit 摘要（repo/path → sha → message）
- `Superproject:` commit 摘要
- `Merge:` `/ws-finish` 输出（into/from）
- `Worktree cleanup:` 若存在独立 change worktree，输出清理结果（removed/skipped + reason）
- `Evidence:` `.agentdocs/tmp/aiws-validate/*.json`（若使用 --stamp）
<!-- AIWS_MANAGED_END:opencode:ws-deliver -->

可在下方追加本项目对 OpenCode 的额外说明（托管块外内容会被保留）。
