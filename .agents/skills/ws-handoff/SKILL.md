---
name: ws-handoff
description: 交接（归档后生成/查看 changes/archive/.../handoff.md）
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 让 change 的交接信息可回放：`handoff.md`（包含：本次完成/改动文件/关键决策/下一步建议/绑定）
- 指引在归档与依赖场景下如何使用 handoff

说明：
- `handoff.md` 由 `aiws change archive` 自动生成：`changes/archive/<date>-<change-id>/handoff.md`
- `Depends_On` 依赖若已归档，`aiws change start` 会尝试读取依赖的 `handoff.md` 并输出摘要

执行建议：
1) 先运行 `$ws-preflight`。
2) 若本 change 已完成并准备归档：运行 `aiws change archive <change-id>`（会先做严格校验，并移动目录到 `changes/archive/`，同时生成 `handoff.md`）。
3) 查看 handoff：
```bash
change_id="<change-id>"
ls -1 changes/archive/*-"${change_id}"/handoff.md
sed -n '1,160p' changes/archive/*-"${change_id}"/handoff.md
```
4) 若要让后续 change 可接力：在 `proposal.md` 里声明 `Blocks`（下一步建议会据此生成）。

输出要求：
- `Change_ID:` <change-id>
- `Handoff:` `changes/archive/.../handoff.md`
- `Next:` 若还有后续工作，建议创建新 change 并在其 `Depends_On` 引用本 change

