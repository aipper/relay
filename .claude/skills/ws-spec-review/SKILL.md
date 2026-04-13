---
name: ws-spec-review
description: 规范审查（requirements / plan / evidence / gate 完整性）
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 审查当前改动是否满足真值文件、change 绑定、证据路径和 gate 完整性要求
- 把“流程/规范层 blocker”与“代码层问题”区分开，优先落盘到 `changes/<change-id>/review/spec-review.md`

阶段定位：
- review 子 gate；负责 requirements / plan / evidence / workflow gate 完整性审查。

必需输入：
- `AI_PROJECT.md`
- `REQUIREMENTS.md`
- `AI_WORKSPACE.md`
- 当前 `git diff`
- 若存在：`plan/...`、`changes/<change-id>/proposal.md`、`tasks.md`、`review/`、`evidence/`

必需输出：
- `证据（Evidence）:` `changes/<change-id>/review/spec-review.md` 或回退 `.agentdocs/tmp/review/spec-review.md`
- `阻断项（Blockers）:` requirements 归因 / gate / evidence 缺口
- `下一步（Next）:` 修复项与最小验证命令

阻断条件：
- 无法定位项目根或真值文件
- 无法判断当前 change / 归因上下文
- 无法写 review 证据

完成判定：
- 已落盘 spec review 证据，且明确指出 blocker / warning / next。

步骤（建议）：
1) 先运行 `$ws-preflight`。
2) 对照 `AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md` 检查：
   - 当前改动能否归因到 `Req_ID` / `Problem_ID`
   - `plan/...`、`proposal.md`、`tasks.md`、`evidence/` 是否与改动保持一致
   - 是否存在越界目录改动、危险操作、未声明的非目标扩张
   - 是否已经准备好可复现验证入口
3) 把结论落盘到：
   - 默认：`changes/<change-id>/review/spec-review.md`
   - 回退：`.agentdocs/tmp/review/spec-review.md`
4) 输出：
   - `证据（Evidence）:`
   - `阻断项（Blockers）:`
   - `警告（Warnings）:`
   - `下一步（Next）:`

重点：
- 这是 spec / gate review，不是代码质量 review。
- 若发现实现质量或回归问题，转交给 `$ws-quality-review`。

安全：
- 不打印 secrets。
- 不执行破坏性命令。
