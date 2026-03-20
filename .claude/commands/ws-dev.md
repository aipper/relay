<!-- AIWS_MANAGED_BEGIN:claude:ws-dev -->
# ws dev

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：在 AIWS 约束下完成一个可回放、可验证的小步交付。

建议流程：
1) 先运行 `/ws-preflight`（读真值文件并输出约束摘要）。
   - 若 `/ws-plan` 刚创建了 `change/<change-id>` worktree：后续实现必须在该 worktree 中继续；不要回原工作区重复 `aiws change start ...`
2) 建立变更归因（推荐）：
   - ⚠️ 若准备切分支/创建 worktree，先看 `git status --porcelain`；否则切换上下文后，未提交改动可能“看起来丢了”。
   - 若当前目录已经是 `change/<change-id>` worktree（例如由 `/ws-plan` 创建）：直接在这里继续，不要再创建第二个 worktree，也不要回原工作区写代码。
   - 若非空仅因为 `/ws-plan` 生成了 `plan/...` 或 `changes/<change-id>/...`，这是预期行为；此时优先 `aiws change start <change-id> --hooks --no-switch`，若仍要 `--switch/--worktree`，先提交这些规划工件。
   - 推荐更安全（默认）：`aiws change start <change-id> --hooks --no-switch`（只创建分支/工件 + 启用 hooks；不切分支）
   - 准备进入实现时：若当前已在 `change/<change-id>` 直接继续；若需切换到该分支，先确认除规划工件外无额外未提交改动，再执行：`git switch change/<change-id>`
   - 若你明确要“一键切分支”（不推荐，且 dirty 会被拦截）：`aiws change start <change-id> --hooks --switch`
   - superproject + submodule（推荐）：`aiws change start <change-id> --hooks --worktree --submodules`
   - 若后续需要在 detached submodule 内提交：先挂到 `aiws/pin/<target-branch>`；不要直接切 `change/<change-id>` / `main` / `master`
   - 或手工：`git switch -c change/<change-id>`，并创建 `changes/<change-id>/proposal.md` 与 `changes/<change-id>/tasks.md`（参考 `changes/README.md`）
3) 如涉及需求调整：先 `/ws-req-review` → 用户确认后再 `/ws-req-change`（避免需求漂移）。
4) 实施最小改动：任何改动都要能归因到 `REQUIREMENTS.md`（验收）或 `issues/problem-issues.csv`（问题）。
5) 运行 `AI_WORKSPACE.md` 里声明的验证命令；未运行不声称已运行。
6) 提交前强制：`aiws validate .`（commit/push hooks 也会阻断）。
7) 交付收尾（推荐，减少手动 merge 出错）：运行 `/ws-finish`（底层调用 `aiws change finish`，默认 fast-forward 安全合并回目标分支）。

输出要求：
- `变更文件（Changed）:` 文件清单
- `验证（Verify）:` 实际运行的命令 + 期望结果
- `证据（Evidence）:` 证据路径（例如 `changes/<change-id>/review/...`、`changes/<change-id>/...` 或 `.agentdocs/tmp/...`）
<!-- AIWS_MANAGED_END:claude:ws-dev -->

可在下方追加本项目对 Claude Code 的额外说明（托管块外内容会被保留）。
