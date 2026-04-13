---
name: ws-quality-review
description: 质量审查（行为回归 / 测试覆盖 / 实现质量）
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 审查当前改动的行为正确性、边界条件、测试覆盖和实现质量
- 把“代码/行为层 findings”优先落盘到 `changes/<change-id>/review/quality-review.md`

阶段定位：
- review 子 gate；负责实现质量、行为回归与验证覆盖审查。

必需输入：
- 当前 `git diff`
- 已执行的验证结果
- 相关代码 / 配置 / 测试文件
- 若存在：`changes/<change-id>/analysis/`、`patches/`、已有 review 文件

必需输出：
- `证据（Evidence）:` `changes/<change-id>/review/quality-review.md` 或回退 `.agentdocs/tmp/review/quality-review.md`
- `主要发现（Findings）:` 高到低排序的问题 / 风险 / 缺失测试
- `下一步（Next）:` 最小修复项与回归命令

阻断条件：
- 没有可审改动
- 没有任何验证上下文
- 无法写 review 证据

完成判定：
- 已落盘 quality review 证据，且 findings / 测试缺口 / next 明确。

步骤（建议）：
1) 先读取 `git diff`、验证结果与相关代码。
2) 检查：
   - 行为是否可能回归
   - 边界条件 / 失败路径是否覆盖
   - 测试是否足以支撑改动
   - 是否存在明显复杂度、耦合、可维护性或性能问题
3) 将结论落盘到：
   - 默认：`changes/<change-id>/review/quality-review.md`
   - 回退：`.agentdocs/tmp/review/quality-review.md`
4) 输出：
   - `证据（Evidence）:`
   - `主要发现（Findings）:`
   - `测试缺口（Gaps）:`
   - `下一步（Next）:`

重点：
- 这是质量 / 回归 review，不替代 requirements / gate review。
- 若发现流程、归因、evidence 缺口，转交给 `$ws-spec-review`。

安全：
- 不打印 secrets。
- 不执行破坏性命令。
