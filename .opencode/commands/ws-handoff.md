---
description: 交接：查看归档 change 的 handoff.md（标准由 aiws change finish --push 自动归档生成）
---
<!-- AIWS_MANAGED_BEGIN:opencode:ws-handoff -->
# ws handoff

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 交接与回放：通过 `handoff.md` 让已归档的 change 可被下一位/下一次会话快速接力。
- handoff 文件位置：`changes/archive/<date>-<change-id>/handoff.md`（默认由 `aiws change finish --push` 自动归档时生成）。

执行建议：
1) 先确认 change 已通过 `/ws-finish` 或 `aiws change finish --push` 完成自动归档。
2) 查看 handoff：
```bash
change_id="<change-id>"
ls -1 changes/archive/*-"${change_id}"/handoff.md
sed -n '1,160p' changes/archive/*-"${change_id}"/handoff.md
```
3) 依赖提示：
- 若你在某个 change 的 `proposal.md` 声明了 `Depends_On`，`aiws change start` 会尝试读取依赖 change 的 `handoff.md` 并输出摘要（前提：依赖已归档且 handoff 存在）。
- 若是历史/异常场景尚未归档，再手工运行 `/p-aiws-change-archive`。
<!-- AIWS_MANAGED_END:opencode:ws-handoff -->

可在下方追加本项目对 OpenCode 的额外说明（托管块外内容会被保留）。
