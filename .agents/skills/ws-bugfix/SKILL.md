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
3) 建立变更工件（推荐）：
```bash
aiws change start <change-id> --hooks
# superproject + submodule 推荐：
aiws change start <change-id> --hooks --worktree --submodules
```

建议流程（按顺序）：

## 1) 通过禅道 MCP 拉取 bug
- 使用当前会话中已启用的 zentao MCP 工具获取：
  - `bug_id`、标题、优先级/严重级、模块、状态、指派人
  - 重现步骤、期望结果、实际结果
  - 附件列表（含图片 URL/文件名）
- 若当前环境没有 zentao MCP 工具：立即停止并提示用户先配置，不要猜数据。

## 2) 证据落盘（强制）
在 `changes/<change-id>/bug/` 下落盘：
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
- 目标文件：`issues/fix_bus_issues.csv`
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
- 进入 `$ws-dev` 做最小改动修复。
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
- `CSV:` `issues/fix_bus_issues.csv` 中对应 `Bug_ID` 行的关键字段
- `Evidence:` `changes/<change-id>/bug/zentao-bug-<bug-id>.md` + 图片目录
- `Verify:` 实际运行命令与结果（未运行不声称已运行）
