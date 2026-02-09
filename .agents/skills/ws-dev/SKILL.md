---
name: ws-dev
description: 开发（按需求实现并验证；适用于任何需要修改代码/配置的任务）
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：在 AIWS 约束下完成一个可回放、可验证的小步交付。

建议流程：
1) 先做 preflight：定位项目根目录，读取 `AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`，输出约束摘要。
   - 若是中大型任务：建议先用 `$ws-plan` 生成 `plan/` 工件，再进入实现（便于可回放与对齐验证入口）。
2) 建立变更归因（推荐）：
   - 推荐一键：`aiws change start <change-id> --hooks`（切分支 + 初始化变更工件 + 启用 hooks）
   - superproject + submodule（推荐）：`aiws change start <change-id> --hooks --worktree --submodules`（创建独立 worktree；当前目录分支保持不变）
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
- `Changed:` 文件清单
- `Verify:` 实际运行的命令 + 期望结果
- `Evidence:` 证据路径（例如 `plan/...`、`.agentdocs/tmp/aiws-validate/...` 或 `changes/<change-id>/...`）
