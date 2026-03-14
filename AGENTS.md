# Repository Guidelines

<!-- AIWS_MANAGED_BEGIN:agents -->
本仓库启用 AIWS（AI Workspace）约定，请先读取并遵守（按优先级）：
1) `AI_PROJECT.md`（规则/边界）
2) `REQUIREMENTS.md`（需求与验收真值）
3) `AI_WORKSPACE.md`（运行/测试入口真值）
4) `changes/README.md`（变更工件流程与归档）

协作约束（建议最小集）：
- 每次变更使用分支 `change/<change-id>`，并维护 `changes/<change-id>/proposal.md`、`tasks.md`（可选 `design.md`）
- 启用本机门禁（推荐）：`aiws hooks install .`（或手工：`git config core.hooksPath .githooks`；`git commit`/`git push` 会自动跑 `aiws validate .`）
- 提交前校验（强制门禁）：`aiws validate .`（包含：漂移检测 + `ws_change_check` + `requirements_contract`）
- Codex（推荐）：本仓库内置 repo skills：`.agents/skills/`（可显式 `$ws-dev`，也可隐式套用工作流）
- Codex skills（常用，一句话说明）：
  - `$ws-preflight`：预检（读取真值文件并输出约束摘要）
  - `$ws-submodule-setup`：子模块分支对齐（写入 `.gitmodules` 的 `submodule.<name>.branch`）
  - `$ws-plan`：规划（生成可落盘 `plan/` 工件；供 `$ws-dev` 执行）
  - `$ws-plan-verify`：计划质检（执行前检查计划是否过长/跑偏）
  - `$ws-dev`：开发（按需求实现并验证；适用于任何需要修改代码/配置的任务）
  - `$ws-bugfix`：缺陷修复（禅道 MCP 拉单 + 图片证据落盘 + `issues/fix_bus_issues.csv` 汇总）
  - `$ws-pull`：拉取并对齐 submodules（尽量避免 detached；减少人为差异）
  - `$ws-push`：推送（submodule 感知：先 submodules 后 superproject；fast-forward 安全）
  - `$ws-review`：评审（提交前审计；证据优先落盘到 `changes/<change-id>/review/`）
  - `$ws-commit`：提交（先审计/门禁再 commit；submodule 感知）
  - `$aiws-init`：初始化工作区（生成真值文件与门禁）
  - `$aiws-validate`：校验工作区（漂移检测 + 门禁）
  - `$aiws-hooks-install`：启用 git hooks 门禁（`core.hooksPath=.githooks`）
  - `$aiws-change-new`：创建 `changes/<change-id>` 工件
- Codex CLI（推荐，可选）：安装全局 skills：`npx @aipper/aiws codex install-skills`（写入 `~/.codex/skills/` 或 `$CODEX_HOME/skills`）
- Codex CLI（遗留，可选）：安装全局 prompts：`npx @aipper/aiws codex install-prompts`（写入 `~/.codex/prompts/` 或 `$CODEX_HOME/prompts`；prompts 已 deprecated）
- 不要把敏感信息写入 git：`secrets/test-accounts.json`、`.env*`、token、内网地址等

如果缺文件：运行 `npx @aipper/aiws init`（或 `aiws init`）。
（如你仍在使用 dotfiles 的 `ws` wrappers，也可用 `ws init/ws migrate`；但本模板默认不依赖 dotfiles。）
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
