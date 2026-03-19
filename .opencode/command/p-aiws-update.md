---
description: 私有：更新：刷新模板与 tool-native 文件（先备份）
---
<!-- AIWS_MANAGED_BEGIN:opencode:aiws-update -->
# aiws update

（私有原子入口；日常优先用 ws-* 链路。）

目标：
- 基于当前 `@aipper/aiws-spec` 刷新模板与 tool-native 文件
- 更新前备份到 `.aiws/backups/<timestamp>/`

建议执行：
1) `npx @aipper/aiws update`
2) `npx @aipper/aiws validate`

约束：
- 不写入任何 secrets
- 只更新托管块或 spec 指定的 `replace_file` 文件
<!-- AIWS_MANAGED_END:opencode:aiws-update -->

可在下方追加本项目对 OpenCode 的额外说明（托管块外内容会被保留）。
