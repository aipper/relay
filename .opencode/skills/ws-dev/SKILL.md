---
name: ws-dev
description: 使用时机：需要修改代码、配置、测试时。触发词：实现、修复、开发、编码、写代码、改bug。注意：需求未冻结先用 ws-intake；极简修复可走 ws-dev-lite。
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：在 AIWS 约束下完成一个可回放、可验证的小步交付。

阶段定位：implementation 阶段。

## 必需输入

- 真值文件：`AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`
- 当前任务的归因目标（`Req_ID` 或 `Problem_ID`）
- 若为 medium/complex：已通过 `$ws-plan` / `$ws-plan-verify` 的计划
- 当前 `change/<change-id>` 上下文或等价变更归因

## 必需输出

- `变更文件（Changed）:` 实际改动清单
- `验证（Verify）:` 实际运行的命令与结果说明
- `证据（Evidence）:` `plan/...`、`.aiws/changes/<change-id>/...`、`.aiws/tmp/...` 等证据路径
- `Next:` 若准备提交，建议 `$ws-review` 或 `$ws-commit`

## 前置条件（硬阻断 — 必须最先检查）

在开始任何代码改动之前，必须完成以下检查：

1. **Design Gate**：若 `.aiws/changes/<change-id>/proposal.md` 不存在：
   - 立即停止，不要写代码
   - 输出：`BLOCKED: 缺少 proposal。请先执行 $ws-plan 创建变更计划与任务分解。`
2. **Task Gate**：若 `.aiws/changes/<change-id>/tasks.md` 不存在：
   - 立即停止，不要写代码
   - 输出：`BLOCKED: 缺少 tasks。请先执行 $ws-plan 创建任务分解。`

> 例外：`ws-dev-lite` 是轻量入口，可豁免 Design Gate，但仅限单文件/typo/config/bugfix 场景。

## TDD 约束（强制）

对于所有需要编写新代码或修改业务逻辑的任务，必须遵守 RED-GREEN-REFACTOR 流程：

1. **RED**：先编写测试用例，运行并确认测试失败（或确认现有测试覆盖缺口）
2. **GREEN**：编写最小实现代码使测试通过
3. **REFACTOR**：重构代码，保持测试通过

禁止：
- 先写实现代码再补测试
- 跳过测试步骤直接提交

自我检查顺序（每次修改后）：`lint → typecheck → test`。若项目无对应脚本则跳过该项。

## 完成判定

改动已落盘、验证已执行或明确未执行原因、证据路径可回放，并可进入 review/commit 阶段。

## 建议流程

### 1. Preflight

定位项目根目录，读取 `AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`，输出约束摘要。

- 中大型任务：建议先用 `$ws-plan` 生成 `plan/` 工件。
- 中大型任务默认执行 [3.1 自我修正循环](#31-自我修正循环evaluate-optimize)——这是必经步骤，不是可选项：实现后先自审修正（最多 2 轮），再进入 review
- 已有计划：先 `$ws-plan-verify`，通过后进入实现。
- `$ws-plan` 已创建 worktree：直接在该 worktree 中继续。

### 1.5 Spec Refresh（进入实现前必做）

在开始任何代码改动前，强制重读并输出摘要：
- `AI_PROJECT.md` 安全边界（哪些目录不能动、哪些约束必须遵守）
- `REQUIREMENTS.md` 中与本次 `Req_ID` 相关的条目（摘要 2-3 段即可）

目的：避免落地时遗忘约束或需求边界。仅需 2-3 段摘要，不需要全文复读。

### 2. 建立变更归因

- 若 `git status --porcelain` 仅有计划/工件文件，属于预期行为，继续即可。
- 若需创建新 change：`aiws change start <change-id> --hooks --no-switch`
- 若需切换分支：先确认无额外未提交改动，再 `git switch change/<change-id>`
- 若存在 submodule（`.gitmodules`）：进入编码前必须准备好 `.aiws/changes/<change-id>/submodules.targets`。`aiws change start` 的 `--submodules` 标志会自动处理。参考 `changes/README.md` 和 `.aiws/changes/<change-id>/submodules.targets` 格式。

### 3. 实现策略：默认 dispatch aiws-worker（Subagent-First）

详细执行循环见 `packages/spec/docs/opencode-subagent-first.md`。

- 主 session **默认不直接写实现代码**；通过 `$ws-delegate` 派发 `aiws-worker`
- `task()` 调用中指定 `role: worker`，让 `aiws-inject-context` 插件自动注入 JSONL 上下文
- worker 返回后，派发 `aiws-reviewer` 做独立审查
- 根据 review 结果决定 fix 或收敛 evidence
- **Inline escape hatch**：如果用户明确说"你直接改"或"do it inline"，主 session 可直接写代码，但必须落盘 evidence 记录理由

**验证先行推荐**：对于非 trivial 改动，建议先确认验证入口再开始实现：
1. 先确认 `AI_WORKSPACE.md` 中对应的验证命令
2. 若验证命令不明确：先补验证入口，再开始实现
3. 可选模式（不强求 TDD）：先写最小验证 → 实现 → 补完整验证

### 3.1 自我修正循环（evaluate-optimize）——必经步骤

在 dispatch subagent 前，主 session 必须执行最多 **2 轮** 自审+修正循环：

1. **实现** → subagent 产出代码
2. **自审** → 主 session 检查：lint/type-check 是否通过？是否符合现有代码模式？是否有明显 bug？
3. **修正** → 如果发现问题，要求 subagent 修正后重新提交
4. **2 轮上限** → 如果 2 轮后仍有问题，升级到 `$ws-review` 做正式审查

**适用场景**：所有非 trivial 改动（单文件修复、配置调整、小步实现、中大型任务均适用）。不适合跨模块架构变更——此类变更直接走 $ws-review。

**注意**：这不是替代 `$ws-review` 的门禁；自审通过后仍需走正式 review gate。

### 4. 其他规则

- 需求调整：先 `$ws-req-review` → 确认后 `$ws-req-change`
- 最小改动：每处改动必须归因到 `REQUIREMENTS.md` 或 `issues/problem-issues.csv`
- 验证：运行 `AI_WORKSPACE.md` 声明的命令；未运行不声称已运行
- 多步任务：使用 `update_plan` 工具跟踪状态
- 提交前门禁：
  ```bash
  aiws validate .
  ```
- 交付收尾：`$ws-finish`

## 输出要求

- `变更文件（Changed）:` 文件清单
- `验证（Verify）:` 实际运行的命令 + 期望结果
- `证据（Evidence）:` 证据路径
