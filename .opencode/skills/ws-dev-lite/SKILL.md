---
name: ws-dev-lite
description: 轻量开发（simple/local 单点修复；最小改动 + 最小验证）
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：在不引入完整重流程的前提下，完成一次 simple/local 小问题修复，并保持可验证、可追溯。

定位：
- `ws-dev` 的轻量入口，不是新的 workflow stage。
- 适用于单点修复、局部配置调整、明确回归修复；不适用于中大型任务。

适用前提：
- 目标明确，且能归因到 `Req_ID` 或 `Problem_ID`
- 验证入口明确，且能做最小可复现验证
- 一般只影响单文件或紧密相关的小范围文件
- 不需要先改 `REQUIREMENTS.md`，也不需要先单独做 review

立即升级回主流程的情形：
- 发现任务其实是 medium/complex、跨模块、跨目录或需要方案设计
- 无法明确归因、verify、change 上下文
- 需要新建复杂 change/worktree 或处理 submodule 目标分支真值
- 修复过程中出现连锁改动、需要补系统性测试或需求调整

默认约束：
- 先做 `$ws-preflight`
- 默认不创建 `plan/...`
- 默认不跑 `$ws-plan-verify`
- 默认不要求先做双 review
- 若后续准备提交/交付，仍需进入 `$ws-review` / `$ws-commit` / `$ws-finish`

执行步骤：
1) 先读取 `AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`，输出约束摘要。
2) 先判断自己是否真的属于 lite：
   - 用一句话写清 `Goal`
   - 用一句话写清 `Why lite`
   - 若说不清，立刻回退到 `$ws-dev` 或 `$ws-plan`
3) 在当前 change 上下文内实施最小改动：
   - 不为“形式完整”额外扩 scope
   - 不默认补与本问题无关的重构或测试矩阵
4) 运行最小可复现验证：
   - 优先使用 `AI_WORKSPACE.md` 已声明的命令
   - 若只需局部回归，允许运行更窄的验证，但要说明为什么足够
5) 留下至少一个可追溯证据：
   - 实际改动文件
   - 或 `.agentdocs/tmp/...`
   - 或 `changes/<change-id>/...`
6) 输出：
   - `变更文件（Changed）:`
   - `验证（Verify）:`
   - `证据（Evidence）:`
   - `Next:` 若准备提交，进入 `$ws-review` 或 `$ws-commit`

输出要求：
- 明确说明为什么这是 lite，而不是完整 `ws-dev`
- 未运行不声称已运行
- 一旦发现复杂度升高，立刻停止 lite 叙事，切回 `$ws-dev` 或 `$ws-plan`
