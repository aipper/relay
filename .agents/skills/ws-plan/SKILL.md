---
name: ws-plan
description: 规划（生成可落盘 plan/ 工件；供 ws-dev 执行）
---

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 对齐真值文件（`AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md`）
- 为当前任务生成一份可追踪的执行计划文件：`plan/<timestamp>-<slug>.md`
- 计划必须包含可复现验证命令（优先引用 `AI_WORKSPACE.md`）
- 计划必须包含“主索引绑定”：`Change_ID` / (`Req_ID` or `Problem_ID`) / `Contract_Row` / `Plan_File` / `Evidence_Path`

约束：
- 不写入任何 secrets（token、账号、内网端点等不得进入 git）
- 本 skill 只负责“想清楚怎么做 + 落盘计划”，不要直接大规模改动代码
- 未运行不声称已运行；验证命令要写清“预期结果”
- 若存在 `changes/<change-id>/proposal.md`，计划与 proposal 的绑定字段必须保持一致（不一致时先修正再继续）

执行步骤（建议）：
1) 先运行 `$ws-preflight`（读取真值文件并输出约束摘要）。
2) 若用户任务描述不清：先问 1-3 个关键澄清问题（不要猜）。
3) 判断复杂度：`simple / medium / complex`（给出一句理由），并估算步骤数。
4) 识别主索引上下文（若存在）：
   - 若存在 `changes/<change-id>/proposal.md`：读取其中 `Change_ID` / `Req_ID` / `Problem_ID` / `Contract_Row` / `Evidence_Path`
   - 若缺失关键绑定：先补齐 proposal（至少 `Change_ID`、`Req_ID|Problem_ID`、`Contract_Row`）再继续生成计划
5) 生成计划文件：
   - 文件名：`plan/YYYY-MM-DD_HH-MM-SS-<slug>.md`（`<slug>` 用 kebab-case；同一任务调整计划时尽量复用同一文件）
   - 若 `plan/` 不存在先创建
   - 必须实际写入到磁盘（不要只在对话里输出）；如因权限/策略无法写盘，必须明确说明原因并输出可复制的完整内容
6) 计划内容至少包含（不要留空）：
   - `Bindings`：`Change_ID` / `Req_ID` / `Problem_ID` / `Contract_Row` / `Plan_File` / `Evidence_Path`
   - `Goal`：要达成什么
   - `Non-goals`：明确不做什么（避免 scope creep）
   - `Scope`：将改动的文件/目录清单（不确定就写 `TBD` 并说明如何确定）
   - `Plan`：分步执行（每步尽量落到具体文件/命令；必要时拆 Phase）
   - `Submodules`（当存在 `.gitmodules` 且声明了 submodule 条目时，强制）：声明“本次 change 的 submodule 目标分支真值”（用于同一 superproject 分支内的多渠道交付；也避免仅靠 `.gitmodules` 默认分支导致交付推送到错误分支）
   - `Verify`：可复现命令 + 期望结果（优先引用 `AI_WORKSPACE.md` 的入口；必要时补充 e2e）
   - `Risks & Rollback`：风险点 + 回滚方案（例如 git 回滚、`aiws rollback`、恢复备份等）
   - `Evidence`：计划文件路径；若创建了变更工件则附 `changes/<change-id>/...`
7) 若存在 change proposal：回填并对齐 `proposal.md` 的 `Plan_File`（必要时同步 `Contract_Row` / `Evidence_Path`），保证 plan/proposal 一致。
8) 运行 `$ws-plan-verify` 作为执行前质量门（计划不过长、不跑偏、验证可复现）。
9) 若计划涉及“需求/验收”变更：先用 `$ws-req-review` 评审 → 用户确认后再 `$ws-req-change` 落盘（避免需求漂移）。
10) 多步任务（≥2 步）：后续进入实现时，使用 `update_plan` 工具跟踪 `pending → in_progress → completed`。

补充：submodule 目标分支真值（强约束；同一 superproject 分支内可多渠道）
- 背景：`.gitmodules submodule.<name>.branch` 适合作为“团队默认分支真值”，但当同一 superproject 分支需要在不同交付中选择不同 submodule 目标分支（多渠道）时，仅靠 `.gitmodules` 不足。
- 强约束：当 `.gitmodules` 声明了 submodule 条目时，门禁会要求本次 change 存在该文件且覆盖所有 submodule path（否则 `aiws validate .` / `aiws change validate --strict` 阻断）。
- 约定：为本次 change 落盘一个“交付目标分支映射”文件，并在后续 `$ws-dev`/`$ws-deliver`/`$ws-finish` 优先使用它：
  - 文件：`changes/<change-id>/submodules.targets`
  - 格式：每行一个 submodule（忽略空行与 `#` 注释），字段用空白分隔（推荐 `TAB`）：
    - 第 1 列：submodule path（例如 `vendor/foo`）
    - 第 2 列：target branch（例如 `release/channel-a`）
    - 第 3 列（可选）：remote 名（默认 `origin`）
- 生成模板（建议在确认 `Change_ID` 后执行；如文件已存在先备份再覆盖）：
```bash
change_id="<change-id>"
targets="changes/${change_id}/submodules.targets"
mkdir -p "changes/${change_id}"
if [[ -f "${targets}" ]]; then
  bak="${targets}.bak.$(date -u +%Y%m%d-%H%M%SZ)"
  cp "${targets}" "${bak}"
  echo "info: backup: ${bak}"
fi
: > "${targets}"
echo "# path<TAB>target_branch<TAB>remote(optional, default=origin)" >> "${targets}"
while read -r key sub_path; do
  name="${key#submodule.}"; name="${name%.path}"
  b="$(git config --file .gitmodules --get "submodule.${name}.branch" 2>/dev/null || true)"
  [[ "${b:-}" == "." ]] && b="$(git branch --show-current)"  # '.' means "follow superproject branch"
  printf "%s\t%s\t%s\n" "${sub_path}" "${b:-<fill-me>}" "origin" >> "${targets}"
done < <(git config --file .gitmodules --get-regexp '^submodule\\..*\\.path$' 2>/dev/null || true)
echo "ok: wrote ${targets}"
```
- 计划里必须写清：本次交付选择的 `targets` 内容，以及后续在 `$ws-dev` 进入编码前会把 submodules 挂到 `aiws/pin/<target_branch>`（必要时先 `fetch`）。

输出要求：
- `Plan file:` <实际写入的路径>
- `Next:` 推荐下一步（先 `$ws-plan-verify`，通过后再 `$ws-dev`；或 `aiws change start <change-id> --hooks`，superproject + submodule 可用 `--worktree`）
