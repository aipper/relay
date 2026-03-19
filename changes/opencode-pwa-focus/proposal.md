# Change Proposal: opencode-pwa-focus

> Title: 项目方向调整为 OpenCode-first PWA
>
> Created: 2026-03-19T03:02:10Z

## Bindings

- Change_ID: opencode-pwa-focus
- Req_ID: WEB-073
- Additional_Req_IDs: WEB-074, WEB-075, HST-022
- Contract_Row: Req_ID=WEB-073, Req_ID=WEB-074, Req_ID=WEB-075, Req_ID=HST-022
- Plan_File: plan/2026-03-19_11-02-10-opencode-pwa-focus.md
- Evidence_Path(s): .agentdocs/tmp/opencode-pwa-focus-feasibility-20260319.md

## 目标与非目标

**目标：**
- 评审“项目转向 OpenCode-first”是否可行，并把后续执行顺序落成可复现计划。
- 明确 PWA 在只考虑 `opencode` 时需要补齐的关键能力：切模型、切 session、新开 session、查看对话、列出 todo/计划、任务完成后自动更新计划状态。
- 明确当前仓库哪些能力已经具备，哪些能力仍缺失。

**非目标：**
- 本次不直接实现 OpenCode-first 功能。
- 本次不处理 `codex` / `claude` / `iflow` 的同等级产品化路径。
- 本次不替换为 OpenCode 官方 web UI。

## 变更归因（强制二选一）

- 需求交付：`Req_ID` = WEB-073
- 需求交付（补充）：`Req_ID` = WEB-074, WEB-075, HST-022

## 现状与问题

- 当前仓库已经具备 `hostd` structured OpenCode 运行、model discovery、PWA 启动页模型选择、generic sessions/messages API。
- 但当前产品语义仍以 relay generic `run/session` 为主，并未把 OpenCode 原生 `sessionID`、todo API、session lifecycle 提升为一等模型。
- PWA todo 仅为 `web/src/App.svelte` 的 `localStorage` 雏形，没有 server-backed 持久化，也没有基于结构化状态的自动完成。

## 方案概述（What changes）

- 先做需求与协议真值更新，把 OpenCode-first 作为正式方向写进真值文件与合同。
- 然后按 `hostd -> server -> web -> cli` 分层提升 OpenCode session/todo/model 为一等能力。
- 保留 relay 当前远程控制架构，不做替换式重构。

## 影响范围（Scope）

- 影响的服务/模块/目录：
  - `REQUIREMENTS.md`
  - `requirements/requirements-issues.csv`
  - `docs/protocol.md`
  - `hostd/src/*`
  - `server/src/*`
  - `web/src/*`
  - `cli/src/index.ts`
- 可能影响的外部接口/使用方：
  - `/sessions` / `/sessions/:id` / `/sessions/:id/messages`
  - host info / start-run payload / PWA session list / todo panel

## 风险与回滚

- 风险：
  - 若未先更新 requirement rows，就直接进入实现，会导致方向漂移和 gate 阻断。
  - 若过早删除 generic run/session 路径，可能影响现有非 OpenCode runner。
- 回滚方案（必须可执行）：
  - 保留现有 generic session/messages/todo fallback，新增能力一律 additive。
  - 若方向调整未被采纳，回退本次 planning artifacts 即可，不影响运行时代码。

## 验证计划（必须可复现）

- 命令：
  - `python3 tools/requirements_contract.py validate`
  - `aiws validate .`
- 期望结果：
  - 本次方向调整对应的 requirement rows 补齐后，合同与工作区校验可通过。

## 真值文件/合同更新清单

- `REQUIREMENTS.md`：需要（新增 OpenCode-first session/todo/model/lifecycle 条款）
- `requirements/CHANGELOG.md`：需要
- `requirements/requirements-issues.csv`：需要
- `issues/problem-issues.csv`：不需要
- 证据落盘（`.agentdocs/tmp/...`）：`.agentdocs/tmp/opencode-pwa-focus-feasibility-20260319.md`

## 参考真值文件

- `AI_PROJECT.md`
- `AI_WORKSPACE.md`
- `REQUIREMENTS.md`
