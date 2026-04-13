# Repository Guidelines

<!-- AIWS_MANAGED_BEGIN:agents -->
本仓库启用 AIWS（AI Workspace）约定。真值文件（按优先读取）：
1) `AI_PROJECT.md`（规则/边界）→ 2) `REQUIREMENTS.md`（需求与验收真值）→ 3) `AI_WORKSPACE.md`（运行/测试入口真值）

快速分流：
- 小修（≤3 文件，≤100 行）→ `/ws-dev-lite`
- 常规实现 → `/ws-plan` → `ws-dev`
- 高风险变更 → 加 `ws-spec-review` + `ws-quality-review` 双审查
- 不确定 → `/using-aiws`（先读真值，再路由）

协作约束：
- 变更绑定 `change/<id>`，走 plan→verify→dev→review→commit→finish
- review 完成 → `ws-verify-before-complete` 确认验证通过 → 再 finish
- review 三方（spec/quality/verify）需 triage 确定是否有 HIGH blocker
- 主 session 编排收敛，不直接写业务代码；实现与验证由 subagent 产出
- 提交前 `aiws validate .`；敏感信息不入 git
- 缺真值文件先 stop，不直接开工
- handoff 产出 `handoff-evidence.md` 供后续 session 恢复

Red Flags（这些想法都是错的）：
| 你想的 | 实际 |
|--------|------|
| "看着简单直接改" | 简单→ws-dev-lite，不确定→先 /using-aiws |
| "先动手再说" | 评估先于行动，分流在实现之前 |
| "跳过 review" | review 是门禁，不是建议 |
| "主 session 自己写代码" | 编排收敛，subagent 实现 |
| "不想走流程直接改" | escape-hatch: 可走 ws-dev-lite，但须遵守最小约束（降级需标明） |

阶段产物（最少）：intake=`.aiws/plan/*.intake.md` | planning=`.aiws/plan/...`+`proposal.md`+`tasks.md` | dev=代码+Verify | review=`review/...` | validate=`aiws-validate/*.json` | archive=`evidence/...`+`handoff.md`

平台前缀：OpenCode=`/` | 例：`/ws-dev`
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
