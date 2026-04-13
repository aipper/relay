---
name: ws-plan-verify
description: 计划质检（执行前检查计划是否过长/跑偏，并给出最小修正清单）
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 在进入 `$ws-dev` 前，对 `plan/...` 与 `changes/<change-id>/proposal.md` 做一次“硬约束”质检
- 避免计划过长、步骤不够具体、验证不可复现、与需求合同绑定不一致
- 显式做一次多视角方案审查，避免计划“格式合格但方向跑偏”
- 失败时只给最小修正项，修完后再进入实现

适用时机：
- 已执行过 `$ws-plan`，且准备开始编码
- 或用户反馈“计划太长/容易跑偏”，需要先压缩并对齐

阶段定位：
- planning gate；负责在编码前检查计划是否满足 change 严格契约。

必需输入：
- 当前 `plan/...`
- `changes/<change-id>/proposal.md`
- 当前 change 上下文

必需输出：
- `Quality gate:` pass/fail
- `Perspective findings:` 多视角审查结论
- `Fix list:` 最小修正项
- `Next:` 通过则 `$ws-dev`，未通过则继续修正并复跑

阻断条件：
- 无法定位当前 change 或计划文件
- `aiws change validate <change-id> --strict` 未通过

完成判定：
- 计划满足严格门禁，且后续实现可以在不补需求/补绑定的情况下直接开始。

执行步骤（建议）：
1) 先运行 `$ws-preflight`。
2) 识别 change 上下文：
   - 优先从当前分支推断 `change/<change-id>`
   - 若无法推断：读取 `changes/*/proposal.md` 中的 `Change_ID`，并让用户确认本次要质检的 change
3) 运行严格门禁（这是硬约束入口）：
```bash
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws change validate <change-id> --strict
elif command -v aiws >/dev/null 2>&1; then
  aiws change validate <change-id> --strict
else
  npx @aipper/aiws change validate <change-id> --strict
fi
```
4) 若失败：按报错逐条修正 `proposal.md` / `plan/...`，优先级如下：
   - 先修绑定：`Change_ID` / `Req_ID|Problem_ID` / `Contract_Row` / `Plan_File` / `Evidence_Path`
   - 再修计划质量：必需章节、步骤数量、步骤具体性、验证命令与预期
5) 在 strict 通过后，增加多视角方案审查（不要跳过）：
   - 产品/范围：目标是否清楚、是否有 scope creep、是否把不该做的内容带进来
   - 工程实现：数据流、边界条件、失败模式、回滚、验证是否闭合
   - 体验 / DX（按需）：若任务涉及 UI、CLI、API、文档或工具链，检查关键路径是否可用、成本是否可接受
   - 验收完整性：是否能凭当前 Verify 判断 done / not done
6) 若多视角审查发现阻断问题：`Quality gate: fail`，只输出最小修正项，不进入实现。
7) 复跑 strict 校验，直到通过。
8) 输出“可执行最小计划摘要”（3-8 步），再进入 `$ws-dev`。

输出要求：
- `Quality gate:` pass/fail
- `Perspective findings:` 按视角列出阻断点 / 警告 / 通过结论
- `Fix list:` 仅保留必须修改项（按阻断级别排序）
- `Next:` 通过后建议 `$ws-dev`；未通过则继续修正并复跑 strict
