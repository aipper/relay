<!-- AIWS_MANAGED_BEGIN:claude:ws-bugfix -->
# ws bugfix

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 通过禅道 MCP 拉单、图片证据落盘并汇总 `issues/fix_bus_issues.csv`。

建议流程：
1) 先运行 `/ws-preflight`。
2) 若当前不在 `change/<change-id>` 分支 / worktree，先建立 change 上下文：
   - 工作区先保持干净
   - 仓库已有提交：优先 `aiws change start <change-id> --hooks --worktree`
   - superproject + submodule：优先 `aiws change start <change-id> --hooks --worktree --submodules`
   - 仓库尚无提交 / 不满足 worktree 前置条件：回退 `aiws change start <change-id> --hooks --no-switch`
3) 若上一步创建了 worktree：后续 bug 证据、CSV 更新、`/ws-dev` 修复都必须在该 worktree 中继续。
4) 通过已配置 zentao MCP 拉取 bug 字段与附件列表。
5) 落盘证据到当前 active change 上下文的 `changes/<change-id>/bug/`：
   - `zentao-bug-<bug-id>.json`
   - `zentao-bug-<bug-id>.md`
   - `images/<bug-id>/...`
6) upsert 当前 active change 上下文中的 `issues/fix_bus_issues.csv`（主键 `Bug_ID`）。
7) 进入 `/ws-dev` 修复并回填状态字段 `Fix_Status/Verify_Command/Updated_At`。
8) 质量门：`aiws change validate <change-id> --strict` + `aiws validate . --stamp`。

强制约束：
- 不自动 commit / push。
- 不写入 secrets（token、cookie、内网地址）。
- 无法复现时先输出阻塞信息并落盘，不直接改代码。
<!-- AIWS_MANAGED_END:claude:ws-bugfix -->

可在下方追加本项目对 Claude Code 的额外说明（托管块外内容会被保留）。
