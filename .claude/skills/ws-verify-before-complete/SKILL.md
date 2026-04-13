---
name: ws-verify-before-complete
description: 完成前验证（finish / handoff 前检查双审查与 validate/evidence 是否齐全）
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 在进入 `$ws-finish` / `$ws-handoff` 前，检查 review、validate stamp 和证据是否齐全
- 输出明确的 pass/fail 结论，避免“看起来完成了但 gate 没过”的伪完成

阶段定位：
- finish 前 gate；负责 completion readiness 检查，不直接做 merge / push / handoff。

必需输入：
- `changes/<change-id>/review/spec-review.md`
- `changes/<change-id>/review/quality-review.md`
- `.agentdocs/tmp/aiws-validate/*.json`
- 若存在：`changes/<change-id>/evidence/...`、`git status`

必需输出：
- `证据（Evidence）:` `changes/<change-id>/evidence/verify-before-complete.md` 或回退 `.agentdocs/tmp/review/verify-before-complete.md`
- `结论（Result）:` pass / fail
- `缺失项（Missing）:` 未满足的 gate
- `下一步（Next）:` 进入 `$ws-finish` / `$ws-handoff`，或回退前置 gate

阻断条件：
- 缺少 spec review
- 缺少 quality review
- 缺少 validate stamp
- review 中仍有未关闭 blocker
- 无法写 verification 证据

完成判定：
- 已落盘 verify-before-complete 证据，并明确能否进入 `$ws-finish` / `$ws-handoff`。

步骤（建议）：
1) 识别当前 `change/<change-id>`。
2) 检查以下最小 gate：
   - `changes/<change-id>/review/spec-review.md`
   - `changes/<change-id>/review/quality-review.md`
   - `.agentdocs/tmp/aiws-validate/*.json`
3) 若存在 `changes/<change-id>/evidence/`，检查是否已经收敛 review / validate / collaboration summary。
4) 将结果落盘到：
   - 默认：`changes/<change-id>/evidence/verify-before-complete.md`
   - 回退：`.agentdocs/tmp/review/verify-before-complete.md`
5) 输出：
   - `证据（Evidence）:`
   - `结论（Result）: pass|fail`
   - `缺失项（Missing）:`
   - `下一步（Next）:`

重点：
- 这个 gate 不替代 `$ws-finish`；它只判断“是否具备进入 finish / handoff 的前置条件”。
- 若 fail，必须明确回退到哪个 gate：`$ws-spec-review`、`$ws-quality-review`、`aiws validate . --stamp` 或 `aiws change evidence <change-id>`。

安全：
- 不打印 secrets。
- 不执行破坏性命令。
