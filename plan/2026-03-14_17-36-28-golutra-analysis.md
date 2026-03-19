# Plan: Golutra 优点分析与 Relay 优化借鉴

## Bindings

| 字段 | 值 |
|------|-----|
| **Change ID** | golutra-analysis |
| **Plan File** | `plan/2026-03-14_17-36-28-golutra-analysis.md` |
| **Contract Row** | N/A (分析类工作，非需求交付) |
| **Evidence Path** | 本文档 + 对比分析 |

## Goal

整理 golutra 项目的核心优点，分析 relay 可以借鉴/优化的方向，形成可执行的分析报告。

## Non-goals

- 不进行代码实现（仅分析）
- 不对比所有细节功能
- 不评估 golutra 的缺陷

---

## Scope

### 分析对象

- **golutra**: `/home/ab/code/golutra` - Tauri 桌面应用，多 AI CLI 编排工具
- **relay**: `/home/ab/code/relay` - Rust C/S 架构，远程 AI CLI 控制

### 分析维度

1. 架构设计
2. 终端/进程管理
3. 消息/事件流
4. UI/UX 能力
5. 独特功能

---

## Plan

### 1. 架构设计对比

#### golutra 架构

```
┌─────────────────────────────────────────────────┐
│                  Tauri Desktop App              │
│  ┌─────────────┐  ┌──────────────────────────┐ │
│  │  Vue 3 UI  │  │    Rust Backend         │ │
│  │             │◄─►│  - TerminalManager     │ │
│  │             │  │  - MessagePipeline      │ │
│  │             │  │  - Platform APIs       │ │
│  └─────────────┘  └──────────────────────────┘ │
└─────────────────────────────────────────────────┘
```

**特点**:
- 单体桌面应用，所有组件运行在同一进程
- 通过 Tauri IPC 通信
- 本地存储 (ChatDb)
- 多 workspace + 多 member 管理

#### relay 架构

```
┌──────────┐    WebSocket    ┌──────────┐
│   Web    │◄──────────────►│  Server  │
└──────────┘                 └────┬─────┘
                                  │ WebSocket
                            ┌─────▼─────┐
                            │   hostd   │
                            │ (PTY run) │
                            └───────────┘
```

**特点**:
- 分布式 C/S 架构
- 远程控制能力
- SQLite 持久化 + spool 重放
- 事件驱动模型

### 2. 终端/进程管理

#### golutra 亮点

**launch.rs** - 多级兜底启动策略:

```rust
// 优先级: 指定终端 → 默认终端 → 候选列表 → 系统默认
pub fn launch_terminal_with_fallback(
    terminal_type,  // Shell/Claude/Codex/OpenCode/Qwen/Gemini
    terminal_path,  // 自定义路径
    terminal_command,
    strict_shell,
) -> TerminalLaunchResult { fallback_used }
```

**poller.rs** - 触发式调度:

```rust
// 事件驱动规则评估，状态回落保持定时触发
pub fn spawn_status_poller(app: AppHandle, manager: &TerminalManager) {
    // TriggerScheduler: 只在触发事件时评估对应规则
    // 支持: SemanticFlush, StatusFallback, PostReady 等规则
}
```

**支持的终端类型**:
- Shell (bash/zsh/powershell)
- Claude Code
- Gemini CLI
- Codex CLI
- OpenCode
- Qwen Code
- OpenClaw

#### relay 亮点

**run_manager.rs** - Tool Mode 自动探测:

```rust
// Codex: TUI / MCP Structured / Auto (自动探测最优模式)
codex_mode_setting() -> CodexModeSetting::Tui | Structured | Auto

// OpenCode: Structured / TUI
opencode_mode_setting() -> OpencodeModeSetting::Structured | Tui

// Tool Mode Cache: 探测结果持久化，避免重复探测
```

**PTY 管理**:
- PTY + tmux 包装 (可选)
- stdout/stderr 分离
- 输出缓冲 + flush 策略

### 3. 消息/事件流

#### golutra - Transport + Repository 模式

```rust
// message_pipeline.rs
pub struct UiMessageTransport { app: AppHandle }
impl TerminalMessageTransport for UiMessageTransport {
    fn emit_terminal_stream(&self, payload) {
        self.app.emit("terminal-message-stream", payload);
    }
}

pub struct UiMessageRepository { app: AppHandle }
impl TerminalMessageRepository for UiMessageRepository {
    fn append_terminal_message(&self, ...) {
        chat_append_terminal_message(&self.app, state.inner(), ...)
    }
}
```

**特点**:
- Transport: 实时推送 (Tauri emit)
- Repository: 持久化存储
- Pipeline: 分离关注点

#### relay - 事件模型

```json
// 事件类型
run.started, run.output, run.exited
run.awaiting_input, run.permission_requested
tool.call, tool.result
rpc.*, rpc.response

// 离线回放
run.ack: 服务器确认收到事件，用于 spool 清理
```

### 4. UI/UX 能力 (golutra)

#### 终端功能

| 功能 | golutra | relay |
|------|---------|-------|
| 终端输出流 | ✅ 实时流式 | ✅ |
| 快照/回放 | ✅ Snapshot Service | ❌ |
| 语义分析 | ✅ Semantic Worker | ❌ |
| 触发规则 | ✅ Polling Rules | ❌ |
| 上下文感知 | ✅ Project Context | ❌ |
| 终端注入 | ✅ Direct Injection | ❌ |

#### 多 Agent 管理

| 功能 | golutra | relay |
|------|---------|-------|
| 多终端并行 | ✅ 无限制 | ✅ (多 run) |
| Workspace 隔离 | ✅ | ❌ |
| Member 管理 | ✅ | ❌ |
| 状态追踪 | ✅ TerminalStatusPayload | ✅ (hosts/runs) |

### 5. 独特功能对比

| 功能 | golutra | relay | 可借鉴性 |
|------|---------|-------|----------|
| 终端注入 | ✅ `terminal_dispatch` | ❌ | 高 |
| 语义 flush | ✅ 智能输出分段 | ❌ | 中 |
| Post-ready 状态 | ✅ 会话就绪检测 | ❌ | 中 |
| MCP 桥接 | ❌ | ✅ Codex MCP | N/A |
| 离线 spool | ❌ | ✅ | N/A |
| 远程控制 | ❌ | ✅ | N/A |

---

## 优化建议 (Relay 可借鉴方向)

### 高优先级

#### 1. 终端注入能力 (Direct Injection)

**现状**: relay 仅支持远程输入 (`run.send_input`)，无法主动注入命令到运行中的终端

**golang 方案**:
```rust
// golutra: terminal_dispatch
pub fn terminal_dispatch(
    terminal_id: String,
    data: String,
    context: TerminalDispatchContext,
) -> Result<(), String>
```

**relay 可借鉴**:
- 在 hostd 添加 `rpc.terminal.inject` 接口
- 支持在任意时刻注入命令到 PTY
- 可用于: 自动触发 agent 响应、调试、会话恢复

#### 2. 语义输出处理 (Semantic Worker)

**现状**: relay 直接转发原始输出，无智能分段

**golang 方案**:
```rust
// terminal_engine/semantic.rs
// - 检测 AI 输出边界
// - 提取结构化信息 (tool call, result)
// - 智能 flush 策略
```

**relay 可借鉴**:
- 在 `run.output` 基础上增加语义层
- 识别 tool call / tool result 边界
- 改善 UI 展示 (卡片式 vs 原始流)

#### 3. 会话就绪检测 (Post-Ready State)

**现状**: relay 依赖 `run.started` 事件，无进一步状态检测

**golang 方案**:
```rust
// PostReadyState: Starting -> Ready -> ...
// poller.rs 跟踪会话是否真正就绪
pub const POST_READY_TICK_MS: u64 = 500;
```

**relay 可借鉴**:
- 添加 `run.ready` 事件 (agent 初始化完成)
- UI 可据此展示"正在准备环境" vs "真正运行中"

### 中优先级

#### 4. 多级兜底启动策略

**现状**: relay 直接启动，失败即失败

**golang 方案**: `launch_terminal_with_fallback`

**relay 可借鉴**:
- 在 `start_run` 失败时尝试 fallback
- 例如: `codex mcp` 失败 → 降级到 PTY 模式

#### 5. 触发式规则评估

**现状**: relay 纯事件驱动，无定时/状态规则

**golang 方案**: `TriggerScheduler` + `RuleMask`

**relay 可借鉴**:
- 心跳检测 (host 在线状态)
- 超时自动结束
- 状态回退策略

#### 6. 输出快照服务

**golang 方案**:
```rust
// snapshot_service.rs
// - 终端内容快照
// - 历史回看
// - 调试辅助
```

**relay 可借鉴**:
- 在 web 端提供"输出快照"功能
- 支持查看历史输出

### 低优先级 (架构差异大)

- Workspace/Member 管理 (golutra 为单体设计)
- 本地 ChatDb (relay 为分布式)
- 上下文感知 (需要 IDE 集成)

---

## 总结

| 类别 | golutra 优势 | relay 当前状态 | 借鉴价值 |
|------|-------------|---------------|----------|
| 终端智能 | 语义分析、触发器、注入 | 基础 PTY | ⭐⭐⭐ |
| 启动策略 | 多级兜底 | 单一路径 | ⭐⭐ |
| 状态管理 | Post-Ready 检测 | 简单状态机 | ⭐⭐ |
| 离线能力 | 本地存储 | Spool 重放 | ⭐ (relay 已实现) |
| 远程控制 | 无 | WebSocket 远程 | ⭐ (relay 已实现) |
| MCP 桥接 | 无 | Codex MCP | ⭐ (relay 已实现) |

**核心差距**: golutra 作为本地桌面应用，在"终端智能化"方面远强于 relay；relay 作为远程控制工具，在"分布式能力"方面是优势。

**建议优先级**: 
1. 终端注入能力 (高价值，可快速实现)
2. 语义输出处理 (提升 UI 体验)
3. 会话就绪检测 (改善状态反馈)
4. 启动兜底策略 (提升鲁棒性)

---

## Verify

- [x] 对比两个项目的核心模块
- [x] 分析 golutra 的技术亮点
- [x] 识别 relay 可借鉴方向
- [x] 输出可落盘分析报告

## Risks & Rollback

- **风险**: 本分析仅基于代码阅读，未实际运行 golutra
- **回滚**: N/A (分析类工作)

## Evidence

- `plan/2026-03-14_17-36-28-golutra-analysis.md` (本文档)
- 依赖文件:
  - `/home/ab/code/golutra/src-tauri/src/ui_gateway/terminal.rs`
  - `/home/ab/code/golutra/src-tauri/src/ui_gateway/message_pipeline.rs`
  - `/home/ab/code/golutra/src-tauri/src/terminal_engine/session/launch.rs`
  - `/home/ab/code/golutra/src-tauri/src/terminal_engine/session/poller.rs`
  - `/home/ab/code/relay/hostd/src/run_manager.rs`
  - `/home/ab/code/relay/docs/protocol.md`
