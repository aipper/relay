# Change Proposal: opencode-pwa-focus

> Title: OpenCode-first 运行时收敛与 PWA 对齐
>
> Created: 2026-03-19T03:02:10Z

## Bindings

- Change_ID: opencode-pwa-focus
- Req_ID: WEB-073
- Additional_Req_IDs: WEB-074, WEB-075, HST-022
- Contract_Row: Req_ID=WEB-073, Req_ID=WEB-074, Req_ID=WEB-075, Req_ID=HST-022
- Plan_File: plan/2026-03-19_11-02-10-opencode-pwa-focus.md
- Evidence_Path(s): .agentdocs/tmp/opencode-pwa-focus-feasibility-20260319.md, changes/opencode-pwa-focus/evidence/change-status-20260319-070517Z.json, changes/opencode-pwa-focus/evidence/change-validate-strict-20260319-070517Z.json, changes/opencode-pwa-focus/evidence/aiws-validate-stamp-20260319-070517Z.json, changes/opencode-pwa-focus/evidence/change-sync-stamp-20260319-070517Z.json, changes/opencode-pwa-focus/evidence/delivery-summary-20260319-070517Z.md, changes/opencode-pwa-focus/evidence/change-status-20260319-071820Z.json, changes/opencode-pwa-focus/evidence/change-validate-strict-20260319-071820Z.json, changes/opencode-pwa-focus/evidence/aiws-validate-stamp-20260319-071820Z.json, changes/opencode-pwa-focus/evidence/change-sync-stamp-20260319-071820Z.json, changes/opencode-pwa-focus/evidence/delivery-summary-20260319-071820Z.md, changes/opencode-pwa-focus/evidence/change-status-20260319-104658Z.json, changes/opencode-pwa-focus/evidence/change-validate-strict-20260319-104658Z.json, changes/opencode-pwa-focus/evidence/aiws-validate-stamp-20260319-104658Z.json, changes/opencode-pwa-focus/evidence/change-sync-stamp-20260319-104658Z.json, changes/opencode-pwa-focus/evidence/delivery-summary-20260319-104658Z.md

## 目标与非目标

**目标：**
- 将当前产品范围收敛为 `opencode` 优先，并移除 `claude` / `iflow` 的对外支持面。
- 强化 `opencode` structured 模式的可观测性：对无输出挂起给出显式诊断，并阻止已知不兼容模型进入启动路径。
- 让 PWA / hostd / CLI / 文档 / REQUIREMENTS 对“当前仅支持 opencode”的口径保持一致。

**非目标：**
- 本次不恢复 `codex` 的运行时支持；`codex` 仅保留为后续计划或历史文档语境。
- 本次不替换为 OpenCode 官方 web UI。
- 本次不扩展新的非 `opencode` runner。

## 变更归因（强制二选一）

- 需求交付：`Req_ID` = WEB-073
- 需求交付（补充）：`Req_ID` = WEB-074, WEB-075, HST-022

## 现状与问题

- 当前仓库已经具备 `hostd` structured OpenCode 运行、model discovery、PWA 启动页模型选择、generic sessions/messages API。
- 但此前运行时与文档仍同时暴露 `codex` / `claude` / `iflow` 等入口，导致用户可选到当前并不准备继续支持的路径。
- 此外，structured OpenCode 在部分模型/提供方组合上会出现“无 JSON / 无 stderr”的静默挂起，需要明确的 watchdog 诊断与模型护栏。

## 方案概述（What changes）

- 在 `hostd` 启动路径上把当前 build 硬性收敛为仅支持 `opencode`。
- 在 `web` / `CLI` / `docs` / `REQUIREMENTS` 中同步删除或下线非 `opencode` 的当前支持声明。
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
  - `relay-cli/src/main.rs`
- 可能影响的外部接口/使用方：
  - `/sessions` / `/sessions/:id` / `/sessions/:id/messages`
  - host info / start-run payload / PWA session list / todo panel

## 风险与回滚

- 风险：
- 若工件仍停留在“只做 planning”，而代码已演进为实际实现，交付时会出现工件与代码漂移。
- 若未同步更新 UI / host capability / CLI 帮助文本，用户仍可能走到已下线 runner 路径。
- 回滚方案（必须可执行）：
  - 保留现有 generic session/messages/todo fallback，新增能力一律 additive。
  - 若需要恢复多后端支持，可基于 git 历史恢复 `claude` / `iflow` runner 与相关文档，不影响 `opencode` 主路径。

## 验证计划（必须可复现）

- 命令：
  - `python3 tools/requirements_contract.py validate`
  - `aiws validate .`
- 期望结果：
  - 本次方向调整对应的 requirement rows 补齐后，合同与工作区校验可通过。

## 真值文件/合同更新清单

- `REQUIREMENTS.md`：已更新（新增当前运行时仅支持 `opencode` 的约束）
- `requirements/CHANGELOG.md`：本次未更新
- `requirements/requirements-issues.csv`：本次未更新
- `issues/problem-issues.csv`：不需要
- 证据落盘（`.agentdocs/tmp/...`）：`.agentdocs/tmp/opencode-pwa-focus-feasibility-20260319.md`

## 参考真值文件

- `AI_PROJECT.md`
- `AI_WORKSPACE.md`
- `REQUIREMENTS.md`
