# Plan: 终端注入 (Terminal Injection) 功能实现

## Bindings

- Change_ID: terminal-injection
- Req_ID: FEATURE-001
- Problem_ID: N/A
- Contract_Row: Req_ID=FEATURE-001, Problem_ID=N/A
- Plan_File: plan/2026-03-14_18-00-00-terminal-injection.md
- Evidence_Path(s): .agentdocs/tmp/terminal-injection/

## Goal

在 relay 中实现终端注入功能，允许在任意时刻向运行中的 PTY 注入命令，类似 golutra 的 `terminal_dispatch`。

## Non-goals

- 不实现 DND (Do Not Disturb) 模式（当前无需求）
- 不实现语义 flush（后续单独规划）
- 不实现缓冲策略（v1 优先简单实现）

---

## Scope

### 涉及模块

| 模块 | 改动 |
|------|------|
| `hostd/src/run_manager.rs` | 新增 `inject_input` 方法 |
| `hostd/src/local_api.rs` | 新增 `/runs/:run_id/inject` API |
| `server/src/ws.rs` | 转发 `rpc.terminal.inject` 事件 |
| `server/src/api.rs` | 新增 `/rpc/terminal.inject` 接口 |
| `web/src/lib/api.ts` | 前端调用封装 |
| `web/src/lib/components/RunPanel.svelte` | 注入 UI |

### 事件协议

```json
{
  "method": "rpc.terminal.inject",
  "params": {
    "run_id": "run_xxx",
    "text": "ls -la\n",
    "input_id": "inject_001",
    "context": { "source": "manual" }
  }
}
```

---

## Plan

### Phase 1: hostd 层实现

1. **扩展 RunManager**: 新增 `inject_input` 方法，强制写入 stdin（绕过 awaiting_input 检查）
2. **新增 Local API**: `/runs/:run_id/inject` 接口

### Phase 2: server 层实现

1. **新增 RPC 方法**: `rpc.terminal.inject` 处理
2. **WebSocket 转发**: 转发到 hostd

### Phase 3: 前端实现

1. **API 封装**: `injectInput()` 函数
2. **注入 UI**: RunPanel 输入框

---

## Verify

**期望结果**：
- `cargo fmt --check`: 无格式化差异
- `cargo clippy`: 无 warning
- `cargo test`: 全部通过
- 手动测试: 注入命令后输出可见

- [ ] `cargo fmt --check` 通过
- [ ] `cargo clippy --all-targets --all-features` 无警告
- [ ] `cargo test` 全部通过
- [ ] 手动测试：启动 run 后注入命令验证输出

---

## Risks & Rollback

### 风险

1. **写入时机**: PTY 缓冲区满可能阻塞
2. **并发写入**: send_input 和 inject_input 需加锁
3. **安全**: 注入功能可能被滥用

### 回滚

```bash
git checkout <commit>
cargo build --package relay-server --package hostd
```

---

## Evidence

- 本计划文件
- 依赖: golutra terminal_dispatch, relay send_input
