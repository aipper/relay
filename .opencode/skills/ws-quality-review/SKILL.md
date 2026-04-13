---
name: ws-quality-review
description: 使用时机：需要审查实现质量、测试覆盖时。触发词：质量审查、质量、回归、覆盖、代码体检。注意：流程完整性审查请用 ws-spec-review。
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 审查当前改动的行为正确性、边界条件、测试覆盖和实现质量
- 把“代码/行为层 findings”优先落盘到 `.aiws/changes/<change-id>/review/quality-review.md`

OpenCode + oMo 优先策略：
- 若检测到 `.opencode/oh-my-opencode.json`，或当前会话明确可用 `oracle` / `explore`，优先借用这些 agent 做质量审查。
- `@oracle` 优先负责独立质量/回归审查；`@explore` 负责补代码路径、依赖关系和影响面探索。
- 主 agent 负责把 findings / gaps / next 收敛并落盘。

阶段定位：
- review 子 gate；负责实现质量、行为回归与验证覆盖审查。

必需输入：
- 当前 `git diff`
- 已执行的验证结果
- 相关代码 / 配置 / 测试文件
- 若存在：`.aiws/changes/<change-id>/analysis/`、`patches/`、已有 review 文件

必需输出：
- `证据（Evidence）:` `.aiws/changes/<change-id>/review/quality-review.md` 或回退 `.aiws/tmp/review/quality-review.md`
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
   - 若检测到 oMo：优先让 `@oracle` 做 quality review 草稿；必要时再调用 `@explore` 补代码路径上下文。
2) 检查：
   - 行为是否可能回归
   - 边界条件 / 失败路径是否覆盖
   - 测试是否足以支撑改动
    - 是否存在明显复杂度、耦合、可维护性或性能问题
    - **AI-Slop 检查**（source: `workflow-review-gates.json` aiSlopChecks）：
      - unnecessary_abstraction：过度抽象（单实现接口、未使用的泛化层）
      - fake_comments：伪注释（表述代码行为但不解释 why，或与代码不一致）
      - over_defensive：过度防御（不必要的安全检查、对不可能情况的处理）
      - cargo_cult：货舱崇拜（照搬模式但不理解原因，如不必要的 observer/strategy）
3) 将结论落盘到：
   - 默认：`.aiws/changes/<change-id>/review/quality-review.md`
   - 回退：`.aiws/tmp/review/quality-review.md`
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
- 若 oMo agent 不可用，回退为当前 agent 本地 quality review。
