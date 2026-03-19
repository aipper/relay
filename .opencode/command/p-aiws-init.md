---
description: 私有：初始化：落盘 AIWS 真值与门禁文件
---
<!-- AIWS_MANAGED_BEGIN:opencode:aiws-init -->
# aiws init

（私有原子入口；日常优先用 ws-* 链路。）

目标：
- 生成/补齐真值文件与门禁文件（以 `AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md` 为准）
- 写入/更新 `.gitignore` 的 aiws 托管块
- 生成/更新 `.aiws/manifest.json`（用于漂移检测）

建议执行：
1) `npx @aipper/aiws init`
2) `npx @aipper/aiws validate`

约束：
- 不写入任何 secrets
- 只允许更新托管块（`AIWS_MANAGED_BEGIN/END`）或 spec 指定的 `replace_file` 文件
<!-- AIWS_MANAGED_END:opencode:aiws-init -->

可在下方追加本项目对 OpenCode 的额外说明（托管块外内容会被保留）。
