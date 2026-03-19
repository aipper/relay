# Golutra / HAPI 对比分析与 Relay 优化路线图

本文档目标：把我们从 **golutra** 与 **hapi** 两个“同类产品”中观察到的优点，整理成 relay 可落地的优化清单与里程碑。

## 1. 三个项目定位（一句话）

- **relay**：Rust `server` + Rust `hostd` + Bun `cli` + Svelte `web`，面向“远程控制主机上的 AI CLI 会话”，以 **WebSocket 事件协议 + 离线 spool 回放** 为核心。
- **golutra**：Tauri 桌面应用，面向“本地终端 AI 工作流编排”，以 **终端智能化（注入/语义 flush/触发器）** 为核心。
- **hapi**：local-first（CLI + Hub + Web/PWA + Telegram），以 **Seamless Handoff（本地↔远程无损切换）+ Terminal Anywhere** 为核心。

## 2. 能力矩阵（我们该学什么）

| 维度 | golutra 强项 | hapi 强项 | relay 现状 | 我们的优化方向 |
|---|---|---|---|---|
| 终端随时可控 | `terminal_dispatch` 注入 + 缓冲/就绪门槛 | `terminal:*` 事件链路（web 直连终端）+ `terminal:ready` | `run.send_input` 偏“awaiting 输入”语义；structured/MCP 不走 PTY writer | **Terminal Injection（强制注入）** + 统一审计 |
| 终端状态管理 | shell_ready + 语义 flush gate | `TerminalRegistry`（数量限制/idle timeout） | run 状态较粗，UI 很难判断 ready | **run.ready / readiness** + 终端 idle 清理 |
| 可操作卡片（移动端闭环） | 桌面端更偏终端本身 | ToolCard/PermissionFooter/AskUserQuestionFooter/RequestUserInputFooter | 协议/DB 已有 pending 字段，web 逻辑集中在 `App.svelte` | **组件化卡片流** + 回放一致 |
| 消息/事件同步 | 本地存储 | Hub（SQLite）+ SSE 推送 + Telegram 通知 | Server（SQLite）+ WS + spool/ack | 继续强化：**可回放审计 + UI 对齐** |
| 安全/部署模型 | 本地桌面 | 单二进制 hub + 可选 relay（WireGuard+TLS） | server + hostd（远程） | 借鉴 hapi 的 **namespace/令牌隔离** 思路（可选） |

## 3. Golutra 借鉴点（以“终端注入”为核心）

### 3.1 终端注入的关键模式

golutra 的 `terminal_dispatch` 不是“直接 write stdin”这么简单：

- **会话活跃/就绪检查**：先确保 session active，并读取 `shell_ready` 决定是否立即写。
- **缓冲与刷写**：可能先 buffer，满足条件后统一 `flush_input_buffer(&writer, buffered)`。
- **审计/诊断打点**：每次 dispatch 都会落诊断事件（terminal_id/workspace_id/member_id/dataLen/writeNow/bufferedCount 等）。
- **DND（Do Not Disturb）门禁**：某些 member 状态下直接丢弃注入。

> relay 的 v1 可以先做“强制注入 + 审计”，缓冲/ready gate/DND 做 v2。

参考：
- `golutra/src-tauri/src/terminal_engine/session/mod.rs`：`dispatch_input_with_context(...)`
- `golutra/src-tauri/src/terminal_engine/session/commands.rs`：`terminal_dispatch(...)`

## 4. HAPI 借鉴点（以“Terminal Anywhere + Handoff + 可操作卡片”）

### 4.1 架构与数据流（hapi 的主张）

- CLI ↔ Hub：**Socket.IO**（会话注册、消息、权限、RPC、终端流）。
- Hub ↔ Web：**REST + SSE**（动作走 REST，更新走 SSE，天然支持多端）。
- 可选 Telegram：审批/就绪通知。

参考：
- `hapi/docs/guide/how-it-works.md`
- `hapi/hub/README.md`

### 4.2 “Terminal Anywhere”的实现骨架

hapi 把终端看作一个独立通道：

- 协议层（shared）：定义 `terminal:open/write/resize/close/ready/output/exit/error`。
  - `hapi/shared/src/socket.ts`
- Hub 层：`registerTerminalHandlers(...)` 把 web socket 的 terminal 事件转发到 CLI socket。
  - `hapi/hub/src/socket/handlers/terminal.ts`
- 资源管理：`TerminalRegistry` 维护 `terminalId -> {sessionId, socketId, cliSocketId}`，并支持：
  - `maxTerminalsPerSocket` / `maxTerminalsPerSession`
  - `idleTimeoutMs` 到期自动清理
  - `markActivity()` 续命
  - `hapi/hub/src/socket/terminalRegistry.ts`
- Web 端：`useTerminalSocket()` 负责建连、收到 `terminal:ready` 后才进入 connected 状态。
  - `hapi/web/src/hooks/useTerminalSocket.ts`
- 终端渲染：React + xterm + FitAddon/WebLinks/Canvas。
  - `hapi/web/src/components/Terminal/TerminalView.tsx`

> 对 relay 的启发：
1) 明确区分“消息输入/审批”与“终端通道”；
2) 引入 `run.ready` / `terminal.ready` 语义；
3) 做 terminal 资源注册表与 idle 清理（否则移动端切后台后容易留垃圾连接）。

### 4.3 可操作卡片（ToolCard）

hapi 的 ToolCard 体系做得很“产品化”，不仅展示，还能在卡片上完成动作：

- `ToolCard.tsx` 通过 `getToolPresentation()/knownTools` 决定 title/subtitle/icon；
- 根据 tool 类型挂不同 footer：
  - `PermissionFooter`（批准/拒绝）
  - `AskUserQuestionFooter` / `RequestUserInputFooter`
- 对 `Edit/MultiEdit/Write/Bash` 等输入有专门渲染（DiffView、CodeBlock、MarkdownRenderer）。

参考：
- `hapi/web/src/components/ToolCard/ToolCard.tsx`
- `hapi/web/src/components/ToolCard/views/*`

> 对 relay 的启发：把 `run.permission_requested` / `run.awaiting_input` / `tool.call|tool.result` 统一为可操作卡片流，并从 `web/src/App.svelte` 中拆出模块化组件。

## 5. Relay 的优化路线图（里程碑 + Done 标准）

### M0（最高 ROI / 最小改动面）：Terminal Injection + 审计闭环

**目标**：在任意时刻向运行中 PTY 注入命令（区别于 `run.send_input` 的“等待输入”语义）。

- hostd：新增 `inject` 路径（复用幂等 `input_id`，但不依赖 awaiting_input）。
- server：新增 `rpc.terminal.inject` 转发到 hostd（或新增 HTTP endpoint，视现有路由风格）。
- web：最小入口（按钮/输入框即可）。
- 审计：必须落库（events 表），保证刷新/回放一致。

**Done**：
- 注入后输出可见；
- 刷新后 messages 回放仍能看到注入记录（含 actor/source）。

### M1（体验与鲁棒性）：run.ready + runner fallback 统一

- 引入 `run.ready`（或等价字段/事件），让 UI 知道什么时候可交互。
- 统一 structured/MCP/PTY 的启动探测与回落策略，并把选择结果写入 run 元信息。

**Done**：UI 能区分“启动中/可交互”；失败能自动回落且可观测。

### M2（移动端闭环）：可操作卡片一致化 + 输出治理 v1

- 组件化：将 `tool.call|tool.result`、`run.permission_requested`、`run.awaiting_input` 从 `App.svelte` 拆出。
- 输出 buffer 裁剪策略，避免长会话内存增长。

**Done**：卡片内可完成审批/输入/复制；长输出仍流畅。

### M3（终端智能化）：语义分段/快照/规则

- 语义 flush（分段、边界识别、结构化提取）
- 输出快照（便于回放/定位）
- 规则引擎（idle、timeout、健康检查等）

## 6. 与现有 relay 协议/存储的对齐点

relay 已具备落地这些优化的“地基”：

- 协议：`input_id` 幂等、`tool.call|tool.result`、`run.awaiting_input`、`run.permission_requested`。
- server SQLite：`runs.pending_*` + `events` 表（含 `input_id/text_redacted/text_sha256/data_json`）。

参考：
- `relay/docs/protocol.md`
- `relay/server/src/db.rs`

## 7. 建议下一步（落地顺序）

1) 先做 M0：Terminal Injection（最短路径、ROI 最大）
2) 再做 M1：run.ready + fallback（把“可交互性”补齐）
3) 同时做 M2 的最小拆分：把卡片从 `App.svelte` 拆出来（降低后续改动风险）
