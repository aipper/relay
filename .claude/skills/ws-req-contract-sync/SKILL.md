---
name: ws-req-contract-sync
description: 合同同步（requirements contract sync）
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

用途：从 `REQUIREMENTS.md` 的 FlowSpec 补齐 `requirements/requirements-issues.csv`（只生成骨架，不猜测完成状态）。

执行（在 workspace 根目录）：
- `python3 tools/requirements_contract_sync.py --workspace .`

输出要求：
- 说明新增/更新了多少条
- 明确下一步：手工补齐 CRUD/Inputs/Outputs/Business_Logic/Tests，并将可开工条目标为 `Spec_Status=READY`

下一步建议：`$ws-req-contract-validate`
