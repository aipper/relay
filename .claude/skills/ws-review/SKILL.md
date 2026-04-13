---
name: ws-review
description: 评审（提交前审计与证据落盘）
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：在提交/交付前审计当前改动，对照真值文件检查是否越界，并把审计证据优先落盘到 `changes/<change-id>/review/`（若无法确定 `change-id` 再回退 `.agentdocs/tmp/review/`）。

阶段定位：
- review 阶段；负责对当前改动做规范、风险和验证完整性的审计。

必需输入：
- 当前 `git status` / `git diff`
- 已执行的验证结果
- 真值文件：`AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`
- 当前 `change/<change-id>` 上下文（若能识别）
- 若存在：`changes/<change-id>/analysis/`、`patches/`、已有 `review/` 文件

必需输出：
- 审计文件：`changes/<change-id>/review/codex-review.md` 或回退 `.agentdocs/tmp/review/codex-review.md`
- `主要风险（Top risks）:` 3-8 条
- `下一步（Next）:` 最小修复清单 + 最小验证命令

阻断条件：
- 没有可审计的改动或验证上下文
- 审计证据无法写盘

完成判定：
- 审计证据已落盘，主要风险和下一步已明确，可作为 commit/deliver 前置输入。

步骤（建议）：
1) 先做 preflight：定位项目根目录，读取 `AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`，输出约束摘要。
2) 基于 `git status` / `git diff`（以及你实际运行过的测试结果），对照 `AI_PROJECT.md` 与 `REQUIREMENTS.md` 检查：
   - 是否存在越界目录改动/危险操作
   - 是否有可复现验证命令与证据
   - 是否维护了 `changes/<change-id>/` 或相关 `issues/*.csv`
   - 若存在 `analysis/` / `patches/`：审查这些委托工件是否已被主 agent 理解、是否需要采用/拒绝，并把结论写入 review 文件
3) 将审计落盘到（目录不存在则创建）：
   - 默认：`changes/<change-id>/review/codex-review.md`
   - 回退：`.agentdocs/tmp/review/codex-review.md`（仅在无法确定 `change-id` 时使用）
   - 若已有其它 reviewer 文件：不要覆盖它们；当前 reviewer 应写自己的文件或更新自己的汇总文件
4) 回复中输出：
   - `证据（Evidence）:` 证据文件路径
   - `主要风险（Top risks）:` 3–8 条（高→低）
   - `下一步（Next）:` 最小修复清单 + 最小验证命令

安全：
- 不打印 secrets。
- 不执行破坏性命令。
