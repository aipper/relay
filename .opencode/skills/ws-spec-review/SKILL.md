---
name: ws-spec-review
description: 使用时机：需要审查流程完整性、requirements 归因时。触发词：规范审查、流程审计、spec review。注意：实现质量审查请用 ws-quality-review。
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 审查当前改动是否满足真值文件、change 绑定、证据路径和 gate 完整性要求
- 把“流程/规范层 blocker”与“代码层问题”区分开，优先落盘到 `.aiws/changes/<change-id>/review/spec-review.md`

OpenCode + oMo 优先策略：
- 若检测到 `.opencode/oh-my-opencode.json`，或当前会话明确可用 `oracle` / `librarian`，优先借用它们做 spec / gate 审查。
- `@oracle` 优先负责 requirements / gate / evidence 独立审查；`@librarian` 负责补文档、规范、依赖与路径真值。
- 主 agent 负责把 blocker / warning / next 收敛到最终 review 文件。

阶段定位：
- review 子 gate；负责 requirements / plan / evidence / workflow gate 完整性审查。

必需输入：
- `AI_PROJECT.md`
- `REQUIREMENTS.md`
- `AI_WORKSPACE.md`
- 当前 `git diff`
- 若存在：`plan/...`、`.aiws/changes/<change-id>/proposal.md`、`tasks.md`、`review/`、`evidence/`

必需输出：
- `证据（Evidence）:` `.aiws/changes/<change-id>/review/spec-review.md` 或回退 `.aiws/tmp/review/spec-review.md`
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
   - 若检测到 oMo：优先让 `@oracle` 做 spec review 草稿；需要补规范上下文时再调用 `@librarian`。
2) 对照 `AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md` 检查：
   - 当前改动能否归因到 `Req_ID` / `Problem_ID`
   - `plan/...`、`proposal.md`、`tasks.md`、`evidence/` 是否与改动保持一致
   - 是否存在越界目录改动、危险操作、未声明的非目标扩张
   - 是否已经准备好可复现验证入口
3) 把结论落盘到：
   - 默认：`.aiws/changes/<change-id>/review/spec-review.md`
   - 回退：`.aiws/tmp/review/spec-review.md`
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
- 若 oMo agent 不可用，回退为当前 agent 本地 spec review。
