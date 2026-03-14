---
name: ws-plan-verify
description: 计划质检（执行前检查计划是否过长/跑偏，并给出最小修正清单）
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 在进入 `$ws-dev` 前，对 `plan/...` 与 `changes/<change-id>/proposal.md` 做一次“硬约束”质检
- 避免计划过长、步骤不够具体、验证不可复现、与需求合同绑定不一致
- 失败时只给最小修正项，修完后再进入实现

适用时机：
- 已执行过 `$ws-plan`，且准备开始编码
- 或用户反馈“计划太长/容易跑偏”，需要先压缩并对齐

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
5) 复跑 strict 校验，直到通过。
6) 输出“可执行最小计划摘要”（3-8 步），再进入 `$ws-dev`。

输出要求：
- `Quality gate:` pass/fail
- `Fix list:` 仅保留必须修改项（按阻断级别排序）
- `Next:` 通过后建议 `$ws-dev`；未通过则继续修正并复跑 strict
