# Repository Guidelines

<!-- AIWS_MANAGED_BEGIN:agents -->
本仓库启用 AIWS（AI Workspace）约定，请先读取并遵守（按优先级）：
1) `AI_PROJECT.md`（规则/边界）
2) `REQUIREMENTS.md`（需求与验收真值）
3) `AI_WORKSPACE.md`（运行/测试入口真值）
4) `changes/README.md`（变更工件流程与归档）

最小协作约束：
- 不确定先跑 `/using-aiws`（Codex 对应 `$using-aiws`）；它会先读真值，再把任务路由到合适的 `ws-*` 入口。
- 若你已经明确只是做预检，也可以直接跑 `/ws-preflight`（Codex 对应 `$ws-preflight`）；所有实现/修复都以 `AI_PROJECT.md` / `REQUIREMENTS.md` / `AI_WORKSPACE.md` 为准。
- 每次变更建议进入 `change/<change-id>`，并维护 `changes/<change-id>/proposal.md`、`tasks.md`（可选 `design.md`）。
- 若仓库存在 `.gitmodules`：优先 `aiws change start <change-id> --worktree`；不要在当前 superproject worktree 里直接切分支。
- 提交前执行 `aiws validate .`；需要本机门禁时运行 `aiws hooks install .`（或 `git config core.hooksPath .githooks`）。
- 不要把敏感信息写入 git：`secrets/test-accounts.json`、`.env*`、token、内网地址等。

标准链路（建议默认）：
- `using-aiws`：默认 bootstrap/router；先读真值，先判 workflow，再进入具体阶段；意图不明确时先澄清。
- `ws-preflight`：读取真值文件，输出约束摘要；若真值缺失，先停止，不直接开工。
- `ws-intake`：计划前置澄清；适用于新需求或中大型变更，按“一题一线程”逐条冻结问题，产出 `plan/*.intake.md` 轻量草案。
- `ws-plan`：建立 `change/<change-id>` 绑定，落盘 `plan/...`，明确 Verify / Risks / Evidence。
- `ws-plan-verify`：在编码前做计划质检 + 多视角方案审查；不过门禁就先修 plan，不跳步进实现。
- `ws-dev`：只做可归因、可验证的小步实现；输出 Changed / Verify / Evidence。
- `ws-dev-lite`：`ws-dev` 的轻量入口；适用于 simple/local 单点修复，默认不建 `plan/...`、不跑 `ws-plan-verify`，治理归属仍收敛到 `ws-dev`。
- `ws-delegate`：仅在任务已绑定且 scope 可控时，使用原生多 agent / sub-agent；否则明确降级为单 agent + 协同工件模式。
- `ws-review`：对当前改动做通用审计；高风险或准备交付时，继续细化到 `ws-spec-review` + `ws-quality-review`。
- `ws-spec-review`：审查 requirements 归因、plan/change 绑定、evidence 与 workflow gate 完整性。
- `ws-quality-review`：审查行为回归、边界条件、测试覆盖与实现质量。
- `ws-commit`：串联 review + validate stamp + commit message 确认；不跳过 hooks。
- `ws-verify-before-complete`：在 finish / handoff 前检查双审查、validate stamp 与交付证据是否齐全。
- `ws-deliver` / `ws-finish`：做交付收尾、fast-forward 合并、submodule 感知 push，并在完整 finish 后自动归档。
- `ws-handoff`：查看/补充归档后的 handoff，给下一次会话或下一位协作者接力。

阶段产物（最少）：
- intake：`plan/*.intake.md`
- planning：`plan/...`、`changes/<change-id>/proposal.md`、`tasks.md`
- implementation：代码 / 配置改动 + Verify 命令记录
- collaboration：`changes/<change-id>/analysis/...`、`patches/...`、`review/...`
- review：`changes/<change-id>/review/...` 或回退 `.agentdocs/tmp/review/...`
- validate：`.agentdocs/tmp/aiws-validate/*.json`
- handoff / archive：`changes/<change-id>/evidence/...`、`changes/archive/.../handoff.md`

OpenCode（如果你主要用 OpenCode，优先）：
- native skills 在 `.opencode/skills/`；native commands 在 `.opencode/commands/`
- 若项目启用了 `.opencode/oh-my-opencode.json`：`ws-plan` / `ws-review` / `ws-spec-review` / `ws-quality-review` / `ws-delegate` 会优先借用 oMo 的 `planner-sisyphus` / `explore` / `librarian` / `oracle`
- 若你想按项目固定这组 agent，可参考 `.opencode/oh-my-opencode.json.example`；它只覆盖 `agents`，不接管 hooks/MCP/LSP
- 常用手动入口继续保留在 `.opencode/commands/`（migration window 内 `.opencode/command/` 也会保留）；直接使用 `/ws-*`
- 若你不确定当前任务该先 plan、dev、review 还是 finish：先用 `/using-aiws`
- 需求还没冻结、想逐条把问题聊清楚：先用 `/ws-intake`
- `/ws-preflight`：读取真值文件并输出约束摘要
- `/ws-plan` / `/ws-plan-verify`：先生成计划，再做执行前质检与多视角方案审查
- `/ws-dev`：常规实现/改配置/改测试
- `/ws-dev-lite`：小问题直修；若复杂度升高，立刻回到 `/ws-dev` 或 `/ws-plan`
- `/ws-delegate`：按 AIWS 委托合同拆分子任务，并优先借用 oMo agent
- `/ws-bugfix`：缺陷修复 + 证据落盘 + CSV 汇总
- `/ws-review` / `/ws-commit`：提交前审计、门禁与 commit
- `/ws-spec-review` / `/ws-quality-review`：高风险改动建议显式拆成流程审查 + 质量审查
- `/ws-verify-before-complete`：finish / handoff 前检查双审查和 validate/evidence
- `/ws-finish` / `/ws-deliver`：交付收尾（fast-forward / submodule 感知）
- `/ws-pull` / `/ws-push` / `/ws-submodule-setup`：submodule 场景辅助
- `/p-aiws-*` 为底层原子入口；模板会同时写入 `.opencode/commands/` 与 `.opencode/command/`，一般不需要直接调用

Claude Code（如果你主要用 Claude Code）：
- native skills 在 `.claude/skills/`；command-style 兼容入口保留在 `.claude/commands/`
- 常用链路可按 `ws-*` / `p-aiws-*` 理解；native skills 与 compatibility commands 指向同一套 AIWS workflow
- 若你不确定当前任务该走哪个阶段：先用 `/using-aiws`
- 需求还没冻结、想逐条把问题聊清楚：先用 `/ws-intake`
- 高风险改动或准备 finish 时，建议额外跑 `/ws-spec-review`、`/ws-quality-review`、`/ws-verify-before-complete`

Codex（对应入口，可选）：
- 若你不想先记住阶段，先用 `$using-aiws` 让 router 判定下一步
- 对应入口在 `.agents/skills/`；显式调用时使用 `$ws-*`
- `$using-aiws`
- `$ws-preflight` / `$ws-intake` / `$ws-plan` / `$ws-plan-verify` / `$ws-dev` / `$ws-dev-lite` / `$ws-delegate` / `$ws-frontend-design`
- `$ws-review` / `$ws-spec-review` / `$ws-quality-review` / `$ws-commit`
- `$ws-verify-before-complete` / `$ws-finish` / `$ws-deliver`
- `$ws-pull` / `$ws-push` / `$ws-submodule-setup`
- 其它入口见 `.agents/skills/`：`ws-*` 为常用链路；`p-*` 为底层原子入口，一般不需要直接调用

Codex 全局入口（可选）：
- `npx @aipper/aiws codex install-skills`（推荐；安装全局 skills）
- `npx @aipper/aiws codex install-prompts`（遗留兼容；prompts 已 deprecated）

缺文件或模板漂移时：
- `npx @aipper/aiws init`（初始化）
- `npx @aipper/aiws update`（按模板更新托管内容）
<!-- AIWS_MANAGED_END:agents -->

## Overview

This repo is a multi-component system to run AI coding CLIs on host machines and manage them remotely from a PWA.

- **`server/` (Rust)**: central controller (auth, routing, storage, WebSocket for PWA and hosts)
- **`hostd/` (Rust)**: host daemon (PTY process runner, event spool, connects outbound to `server`)
- **`cli/` (Bun)**: local command wrapper (starts runs via `hostd`, does not directly own tool processes)
- **`web/` (Svelte PWA)**: mobile-friendly UI (runs list, live logs, approve/deny/input)
- **`docs/`**: protocols, API notes, operational docs

## Build, Test, and Dev Commands

Rust (workspace):

- `cargo fmt` / `cargo fmt --check`: format Rust code
- `cargo clippy --all-targets --all-features`: lint Rust code
- `cargo test`: run Rust tests

Bun CLI:

- `cd cli && bun install`: install dependencies
- `cd cli && bun run dev`: run CLI in dev mode (if provided)

Web (Svelte PWA):

- `cd web && bun install`: install dependencies
- `cd web && bun run dev`: start dev server
- `cd web && bun run build`: build production assets

## Coding Style & Naming

- Keep changes scoped; avoid refactors unless required.
- Rust: `rustfmt` + clippy-clean; prefer explicit types at API boundaries.
- TypeScript: `eslint`/`prettier` if configured; keep file/module names kebab-case, exports named.
- Event types are stable API: add new fields compatibly; never remove/rename without a version bump.

## Testing Guidelines

- Prefer fast unit tests for pure logic (redaction, protocol parsing, routing).
- Add integration tests for host↔server event flow when stable.
- Test names should describe behavior (e.g., `redacts_bearer_tokens`).

## Commit & PR Guidelines

- Use clear, scoped messages (e.g., `server: add ws auth`, `hostd: spool ack logic`).
- PRs should include: what changed, how to verify, and any rollback steps.

## Security & Configuration

- Never commit secrets. Keep runtime config in `conf/env` (or `.env`) and add to `.gitignore`.
- Log retention is short by default (3 days). Inputs are stored **redacted**; raw input storage is off by default.


# fast-context MCP 工具使用指南
# AI 语义代码搜索工具使用优先级

## 核心原则
**任何需要理解代码上下文、探索性搜索、或自然语言定位代码的场景，优先使用 `mcp__fast-context__fast_context_search`**

## 使用场景

### 1️⃣ 必须用 fast_context_search
- 探索性搜索（不确定代码在哪个文件/目录）
- 用自然语言描述要找的逻辑（如"XX部署流程"、"XX事件处理"）
- 理解业务逻辑和调用链路
- 跨模块、跨层级查询（如从 router 追到 service 到 model）
- 新任务开始前的代码调研和架构理解
- 中文语义搜索（工具支持中英文双语查询）

### 2️⃣ 根据需求选择工具
- **语义搜索 / 不确定位置** → `fast_context_search`（返回文件+行号范围+grep关键词建议）
- **精确关键词搜索** → Grep
- **已知文件路径，查看内容** → Read
- **按文件名模式查找** → Glob
- **编辑已有文件** → Edit

### 3️⃣ fast_context_search 参数调优
- `tree_depth=1, max_turns=1` — 快速粗查，适合小项目或初步定位
- `tree_depth=3, max_turns=3`（默认）— 平衡精度与速度，适合大多数场景
- `max_turns=5` — 深度搜索，适合复杂调用链追踪
- `project_path` — 指定搜索的项目根目录，默认为当前工作目录
