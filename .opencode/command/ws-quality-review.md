---
description: 质量审查：行为回归 / 测试覆盖 / 实现质量
---
<!-- AIWS_MANAGED_BEGIN:opencode:ws-quality-review -->
# ws quality review

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：审查行为回归、边界条件、测试覆盖与实现质量。

步骤（建议）：
1) 读取 `git diff`、已执行的验证结果和相关代码。
   - 若检测到 `.opencode/oh-my-opencode.json` 或当前会话明确可用 `oracle` / `explore`：优先按 `packages/spec/docs/opencode-omo-adapter.md` 借用这些 agent。
   - 质量/回归审查优先 `@oracle`；代码路径与影响面补充优先 `@explore`。
2) 检查：
   - 是否存在行为回归或明显 bug 风险
   - 边界条件 / 失败路径是否覆盖
   - 测试是否足以支撑当前改动
3) 将结论落盘到：
   - 默认：`changes/<change-id>/review/quality-review.md`
   - 回退：`.agentdocs/tmp/review/quality-review.md`
4) 输出：
   - `证据（Evidence）:`
   - `主要发现（Findings）:`
   - `测试缺口（Gaps）:`
   - `下一步（Next）:`
5) 若 oMo 不可用：回退为当前 agent 本地 quality review。
<!-- AIWS_MANAGED_END:opencode:ws-quality-review -->

可在下方追加本项目对 OpenCode 的额外说明（托管块外内容会被保留）。
