---
name: ws-bugfix
description: 缺陷修复（通过禅道 MCP 拉取 bug 与附件，下载图片证据，汇总到 issues/fix_bus_issues.csv，并绑定到 changes/<change-id>/）
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 用禅道 MCP 拉取 bug 详情与附件（尤其图片）
- 把证据落盘到 `changes/<change-id>/bug/`（避免只停留在对话）
- 把修复任务汇总/更新到 `issues/fix_bus_issues.csv`
- 与 `ws-dev` / `aiws change` 流程绑定，确保可追溯、可验证

非目标（强制）：
- 不自动 commit / push
- 不写入任何 secrets（token、cookie、内网地址）
- 不在无法复现时直接改代码（先产出阻塞信息）

前置：
1) 先运行 `$ws-preflight`。
2) 准备 `change-id`（建议：`bug-<bug-id>` 或 `bugfix-<bug-id>-<slug>`）。
3) 建立 change 上下文（推荐先于任何落盘）：
   - 若当前还不在 `change/<change-id>` 分支 / worktree，先调用 `aiws change start`
   - 工作区必须先干净；否则不要先写 `changes/<change-id>/bug/` 或 `issues/fix_bus_issues.csv`，避免后续切 worktree 时工件留在原工作区
   - 仓库已有提交：优先 `--worktree`
   - superproject + submodule：优先 `--worktree --submodules`
   - 仓库尚无提交 / 不满足 worktree 前置条件：回退 `--no-switch`
```bash
if [[ -n "$(git status --porcelain)" ]]; then
  echo "error: working tree dirty before ws-bugfix creates change context"
  exit 2
fi

if git rev-parse --verify HEAD >/dev/null 2>&1; then
  if [[ -f .gitmodules ]] && git config --file .gitmodules --get-regexp '^submodule\\..*\\.path$' >/dev/null 2>&1; then
    aiws change start <change-id> --hooks --worktree --submodules
  else
    aiws change start <change-id> --hooks --worktree
  fi
else
  aiws change start <change-id> --hooks --no-switch
fi
```
   - 若上一步创建了 worktree：后续 bug 证据、CSV 更新、`$ws-dev` 修复都必须在该 worktree 中继续；不要回原工作区重复创建 change
   - 若该 change 涉及 submodule：
     - 优先复用 `$ws-dev` 的 `submodules.targets` 生成/确认流程
     - detached HEAD 时默认建议取 `.gitmodules` 声明的分支
     - 已附着在某个本地分支时默认建议取当前分支
     - 以上都只是建议值，最终必须显式写入 `changes/<change-id>/submodules.targets`

建议流程（按顺序）：

## 1) 通过禅道 MCP 拉取 bug
- 使用当前会话中已启用的 zentao MCP 工具获取：
  - `bug_id`、标题、优先级/严重级、模块、状态、指派人
  - 重现步骤、期望结果、实际结果
  - 附件列表（含图片 URL/文件名）
- 若当前环境没有 zentao MCP 工具：立即停止并提示用户先配置，不要猜数据。

## 2) 证据落盘（强制）
在当前 active change 上下文的 `changes/<change-id>/bug/` 下落盘：
- `zentao-bug-<bug-id>.json`：原始字段快照（避免信息丢失）
- `zentao-bug-<bug-id>.md`：人类可读摘要（复现步骤/期望/实际/风险）
- `images/<bug-id>/...`：下载的图片附件（保留原扩展名）

建议目录：
```text
changes/<change-id>/bug/
  zentao-bug-<bug-id>.json
  zentao-bug-<bug-id>.md
  images/<bug-id>/
```

## 3) 汇总到 issues/fix_bus_issues.csv（upsert）
- 目标文件：当前 active change 上下文中的 `issues/fix_bus_issues.csv`
- 若文件不存在，先创建表头：
```csv
Bug_ID,Title,Severity,Module,Status,Assigned_To,Change_ID,Image_Count,Image_Paths,Evidence_Path,Verify_Command,Fix_Status,Updated_At,Notes
```
- 以 `Bug_ID` 为主键 upsert：
  - 已存在：更新状态/证据/图片路径
  - 不存在：新增一行

字段约束：
- `Change_ID`：必须等于当前 `change-id`
- `Evidence_Path`：指向 `changes/<change-id>/bug/zentao-bug-<bug-id>.md`
- `Image_Paths`：多个路径用 `;` 分隔
- `Fix_Status`：`TODO|DOING|DONE|BLOCKED`

## 4) 修复执行与回填
- 进入 `$ws-dev` 做最小改动修复；若 `ws-bugfix` 创建了 worktree，则必须在该 worktree 中继续。
- 完成后回填 `issues/fix_bus_issues.csv`：
  - `Fix_Status`
  - `Verify_Command`
  - `Updated_At`
  - `Notes`（必要时写阻塞原因）

## 5) 验证与交付
```bash
aiws change validate <change-id> --strict
aiws validate . --stamp
```
- 需要提交时走 `$ws-commit`。
- 需要收尾合并时走 `$ws-finish`（或在 superproject + submodule 场景走 `$ws-deliver`）。

输出要求：
- `Change_ID:` `<change-id>`
- `Change context:` `<当前分支或 worktree 路径>`
- `CSV:` `issues/fix_bus_issues.csv` 中对应 `Bug_ID` 行的关键字段
- `Evidence:` `changes/<change-id>/bug/zentao-bug-<bug-id>.md` + 图片目录
- `Verify:` 实际运行命令与结果（未运行不声称已运行）
