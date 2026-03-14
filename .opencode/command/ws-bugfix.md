---
description: 缺陷修复：禅道 MCP 拉单、图片证据落盘并汇总 fix_bus_issues.csv
---
<!-- AIWS_MANAGED_BEGIN:opencode:ws-bugfix -->
# ws bugfix

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 通过禅道 MCP 拉单、图片证据落盘并汇总 `issues/fix_bus_issues.csv`。

建议流程：
1) 先运行 `/ws-preflight`。
2) 建立变更工件（推荐）：`aiws change start <change-id> --hooks`（superproject+submodule 可用 `--worktree --submodules`）。
3) 通过已配置 zentao MCP 拉取 bug 字段与附件列表。
4) 落盘证据到 `changes/<change-id>/bug/`：
   - `zentao-bug-<bug-id>.json`
   - `zentao-bug-<bug-id>.md`
   - `images/<bug-id>/...`
5) upsert `issues/fix_bus_issues.csv`（主键 `Bug_ID`）。
6) 进入 `/ws-dev` 修复并回填 `Fix_Status/Verify_Command/Updated_At`。
7) 质量门：`aiws change validate <change-id> --strict` + `aiws validate . --stamp`。

强制约束：
- 不自动 commit / push。
- 不写入 secrets（token、cookie、内网地址）。
- 无法复现时先输出阻塞信息并落盘，不直接改代码。
<!-- AIWS_MANAGED_END:opencode:ws-bugfix -->

可在下方追加本项目对 OpenCode 的额外说明（托管块外内容会被保留）。
