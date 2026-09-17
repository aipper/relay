# Change Proposal: paseo-absorb-v1

> Title: paseo-absorb-v1
>
> Created: 2026-09-17T03:14:30Z

Change_ID: paseo-absorb-v1
Req_ID: PAS-000
Contract_Row: PAS-000, PAS-001, PAS-002, PAS-003, PAS-004
Plan_File: .aiws/plan/2026-09-17_11-04-38-paseo-absorb.md
Evidence_Path: .aiws/changes/paseo-absorb-v1/evidence/

## 目标与非目标

**目标：**
- 按性价比顺序把 paseo（`getpaseo/paseo@6830c46`）四项设计落到 relay，全后向兼容（只加字段、不改名、不删字段）：
  1. PWA 本地 slash 截断（`/quit /clear` 精确匹配，不进 `run.send_input`）
  2. server 侧 5ms 前沿输出合并（首包直发、burst 合并，降 `run.output` 包数）
  3. 审批结构化 `actions[]`（`run.permission_requested.data` 加字段）
  4. `seq` 可见性 + 快照追赶（复用已有 per-`run_id` `seq` 与 `run.ack`/spool 重放）

**非目标：**
- 不做二进制复用帧；不引入 provider-adapter 层；不做 word-paced reveal；不改 runner 启动语义；不修线上 `relay.aipper.de:8787` 不可达（另起 change）

## 变更归因（强制二选一）

- 需求交付：`Req_ID` = PAS-000（umbrella，覆盖 PAS-001~PAS-004；映射 REQUIREMENTS.md H3/M2/M3/M5 + spool replay 验收）
- 问题修复：无（`issues/` 目录不存在，未建问题工件）
- 合同行：`Contract_Row` = PAS-000, PAS-001, PAS-002, PAS-003, PAS-004（`.aiws/requirements/requirements-issues.csv`，2026-09-17 close-out 补）
- 计划文件：`.aiws/plan/2026-09-17_11-04-38-paseo-absorb.md`（plan-verify 已通过：`ok: plan verification passed`）
- 验证入口真值：`AI_WORKSPACE.md`（dev-up.sh 隔离栈 `--port 8790` + `scripts/e2e.sh` + web build + Playwright/chromium-1208，见本 proposal 验证计划）

> 备注：若“问题阻塞需求”，两边都要在各自 CSV 的 `Notes` 字段互相引用对方 ID。

## 现状与问题

- relay `run.output` 逐 chunk 直转 WS + 落库，无合并、无 `rev`、无快照；TUI 重绘风暴下包数爆炸。`run.permission_requested` 只有 `approve_text/deny_text`，PWA 只能渲染二选一。PWA 无本地命令截断，`/quit` 会误发进 PTY。
- 线上验证结论：`relay.aipper.de` PWA 可达但默认 API base `https://relay.aipper.de:8787` 超时，远端 echo 不可验；本地 `127.0.0.1:8787` 健康但 hosts 全 offline（hostd JWT 过期）。本 change 全部验证走隔离栈 `--port 8790` + web dev `5173` + 本地 chromium-1208。

## 方案概述（What changes）

- Phase A：`ChatInput.svelte` 精确匹配本地命令表（exact-match、无参、无附件才拦截），零后端风险先行。
- Phase B：server 推送侧加 5ms 前沿合并器（语义抄 paseo `TerminalOutputCoalescer`，按 run 独立，spool 保持原始 chunk 保 replay 完整性）。
- Phase C：`run.permission_requested.data` 加 `actions[{id,label,behavior}]`（+可选 `suggestions[]`），hostd 发射→server 透传→PWA 多按钮渲染→`approve/deny` 支持 `selectedActionId` 回传。
- Phase D：确认 envelope `seq` 随 `run.output` 下发 web；messages API 支持 `after_seq` 增量回填；web 断线重订按 `seq` 查缺口（按 ID，不按文本内容匹配）。

## 影响范围（Scope）

- 影响的服务/模块/目录：
  - `web/src/lib/components/ChatInput.svelte`（A）
  - `server/src/main.rs`、`server/src/db.rs`、`protocol/src/lib.rs`（B/C/D）
  - `hostd/src/runners/*.rs`、`hostd/src/local_api.rs`（C 发射点）
  - `web/src/lib/components/ApprovalCard.svelte`、`ApprovalModal.svelte`、`SessionDetail.svelte`、`web/src/lib/stores/relay-store.svelte.ts`（C/D）
  - `docs/protocol.md`（C/D 协议加字段说明）
- 可能影响的外部接口/使用方：
  - 旧版 web/CLI/hostd 必须忽略未知字段（后向兼容约束）；`run.output` 包数下降（行为优化，下游按文本拼装不受影响）。

## 风险与回滚

- 风险：
  - 合并器放错位置丢 replay 完整性 → 默认放 server 推送侧，spool 保持原始 chunk。
  - `Contract_Row` 已补：PAS-001/PAS-002/PAS-003/PAS-004（2026-09-17 close-out）。
- 回滚方案（必须可执行）：
  - 按文件 `git checkout -- <file>`；协议文档同步回退；不做 `git reset --hard` / `rm -rf` 类破坏性操作。

## 验证计划（必须可复现）

- 命令：
  - `cargo fmt --check` / `cargo clippy --all-targets --all-features` / `cargo test --workspace` / `bash scripts/e2e.sh`
  - `bash scripts/dev-up.sh --port 8790` + `curl --noproxy "*" http://127.0.0.1:8790/health`
  - `cd web && bun run build`
  - Playwright（`~/.cache/ms-playwright/chromium-1208/chrome-linux64/chrome`，`executablePath` + `--no-sandbox`，连 8790 栈 + web dev 5173，marker 定界不用 sleep）
- 期望结果：
  - cargo/e2e 全过；`[e2e] ok: input idempotency`；health OK
  - `/quit` 零新增 `run.input`；`echo hello` 首包 `<50ms`；`seq 1 100` 包数远小于行数且文本全对；burst 中重连 `seq` 无 gap 无重复；审批 `actions[]` 一轮 approve 生效

## 真值文件/合同更新清单

- `REQUIREMENTS.md`：不需要（H3/M2/M3/M5 已覆盖四项设计；若 D 阶段发现 messages 分页语义缺口再补 `ws-req-change`）
- `requirements/CHANGELOG.md`：不需要（`requirements/` 目录不存在）
- `.aiws/requirements/requirements-issues.csv`：已补 PAS-001~PAS-004 四行，Impl_Status=DONE，证据指向 phase-a/b/c/d.md
- `issues/problem-issues.csv`：不需要（无问题修复归因）
- 证据落盘（`.aiws/changes/paseo-absorb-v1/evidence/`）：每 phase 一次 Playwright/单测证据 + 最终 delivery-summary
