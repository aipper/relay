---
name: ws-handoff
description: 交接（归档后生成/查看 changes/archive/.../handoff.md）
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 让 change 的交接信息可回放：`handoff.md`（包含：本次完成/改动文件/关键决策/下一步建议/绑定）
- 若存在协同工件：在 handoff 中一并概述 `analysis/`、`patches/`、`review/`、`evidence/`
- 指引在归档与依赖场景下如何使用 handoff

说明：
- `handoff.md` 默认由 `aiws change finish --push` 自动归档时生成：`changes/archive/<date>-<change-id>/handoff.md`
- `Depends_On` 依赖若已归档，`aiws change start` 会尝试读取依赖的 `handoff.md` 并输出摘要

阶段定位：
- handoff / archive 阶段；负责查看或补充已归档 change 的交接说明。

必需输入：
- 已归档的 `change/<change-id>`
- 对应 archive 目录

必需输出：
- `Change_ID:` 当前交接对象
- `Handoff:` `changes/archive/.../handoff.md`
- `Next:` 推荐后续 change / `Depends_On` 关系

阻断条件：
- 无法定位目标 change
- `handoff.md` 无法生成或读取

完成判定：
- 下一位协作者可以只依赖 `handoff.md` 和绑定信息继续工作，而不需要回溯整段会话。

执行建议：
1) 先运行 `$ws-preflight`。
2) 先确认本 change 已通过 `aiws change finish <change-id> --push` 完成自动归档。
3) 查看 handoff：
```bash
change_id="<change-id>"
ls -1 changes/archive/*-"${change_id}"/handoff.md
sed -n '1,160p' changes/archive/*-"${change_id}"/handoff.md
```
4) 若是历史/异常场景，尚未归档且需要手工恢复，再运行 `aiws change archive <change-id>`。
5) 若要让后续 change 可接力：在 `proposal.md` 里声明 `Blocks`（下一步建议会据此生成）。
6) 若本 change 使用了外部 / 子 agent 协作：确认关键结论已经进入 `review/` 或 `evidence/`，避免 handoff 只留下原始 patch/分析文件而没有最终结论。

输出要求：
- `Change_ID:` <change-id>
- `Handoff:` `changes/archive/.../handoff.md`
- `Next:` 若还有后续工作，建议创建新 change 并在其 `Depends_On` 引用本 change
