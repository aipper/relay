# Change Proposal: terminal-injection

> Title: 实现终端注入功能 (Terminal Injection)
> Created: 2026-03-14

- Change_ID: terminal-injection
- Req_ID: FEATURE-001
- Contract_Row: Req_ID=FEATURE-001
- Plan_File: plan/2026-03-14_18-00-00-terminal-injection.md
- Evidence_Path(s): .agentdocs/tmp/terminal-injection/

- Change_ID: terminal-injection
- Req_ID: FEATURE-001
- Problem_ID: N/A
- Contract_Row: Req_ID=FEATURE-001
- Plan_File: plan/2026-03-14_18-00-00-terminal-injection.md
- Evidence_Path(s): .agentdocs/tmp/terminal-injection/

## 目标与非目标

**目标：**
- 在 relay 中实现终端注入功能，允许在任意时刻向运行中的 PTY 注入命令
- 类似 golutra 的 `terminal_dispatch`，但适配 relay 的 C/S 架构

**非目标：**
- 不实现 DND (Do Not Disturb) 模式
- 不实现语义 flush
- 不实现缓冲策略 (v1 优先简单实现)

## 现状与问题

当前 relay 仅支持通过 `run.send_input` 发送用户输入，需要等待 agent 响应（awaiting_input）。无法在任意时刻强制注入命令到运行中的 PTY。

> Title: 实现终端注入功能 (Terminal Injection)
> Created: 2026-03-14

## 目标与非目标

**目标：**
- 在 relay 中实现终端注入功能，允许在任意时刻向运行中的 PTY 注入命令
- 类似 golutra 的 `terminal_dispatch`，但适配 relay 的 C/S 架构

**非目标：**
- 不实现 DND (Do Not Disturb) 模式
- 不实现语义 flush
- 不实现缓冲策略 (v1 优先简单实现)

## 变更归因（强制二选一）

- 需求交付：`Req_ID` = FEATURE-001
- 问题修复：`Problem_ID` = N/A

## 现状与问题

当前 relay 仅支持通过 `run.send_input` 发送用户输入，需要等待 agent 响应（awaiting_input）。无法在任意时刻强制注入命令到运行中的 PTY。

## 方案概述（What changes）

- hostd: 新增 `inject_input` 方法和 `/runs/:run_id/inject` API
- server: 新增 RPC 方法 `rpc.terminal.inject`，转发到 hostd
- web: 新增前端注入 UI 组件

## 影响范围（Scope）

- 影响的服务/模块/目录：
  - hostd/src/run_manager.rs
  - hostd/src/local_api.rs
  - server/src/api.rs
  - server/src/ws.rs
  - web/src/lib/api.ts
  - web/src/lib/components/RunPanel.svelte
- 可能影响的外部接口/使用方：
  - 无（向后兼容）

## 风险与回滚

- 风险：
  - 注入时机不当可能导致 PTY 缓冲区满阻塞
  - 并发写入可能需要加锁
- 回滚方案（必须可执行）：
  - `git checkout` 回退代码变更
  - 重新构建 relay-server 和 hostd

## 验证计划（必须可复现）

> 从 `AI_WORKSPACE.md` 选择最贴近本变更的验证入口，写成可直接复制执行的命令。

- 命令：
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features`
  - `cargo test`
- 期望结果：
  - 无格式化差异
  - 无 clippy warning
  - 所有测试通过
  - 手动测试：注入命令后，run 输出中能看到注入的命令内容

## 真值文件/合同更新清单

- `REQUIREMENTS.md`：不需要
- `requirements/CHANGELOG.md`：不需要
- `requirements/requirements-issues.csv`：已添加 FEATURE-001
- `issues/problem-issues.csv`：不需要
- 证据落盘（`.agentdocs/tmp/...`）：代码变更 + 测试结果
