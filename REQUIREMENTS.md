# relay — REQUIREMENTS

本文件用于把当前项目在文档中已经明确写出的“目标/范围/约束/验收方式”固化为可执行的需求清单，便于后续迭代与回滚。

## 需求来源（当前唯一来源）

- `README.md`
- `AGENTS.md`

## 术语

- **server**：中心服务（认证、路由、存储、WebSocket），供 PWA 与 hostd 连接。
- **hostd**：宿主机守护进程（PTY 进程运行器、事件 spooling，出站 WS 连接 server）。
- **cli**：本地命令行封装（通过 hostd 启动 runs，不直接拥有/管理工具进程）。
- **web**：Svelte PWA（runs 列表、实时日志、approve/deny/input）。
- **spool**：hostd 本地 SQLite 事件队列/重放机制，用于离线与断线重连。

## 目标（Goals）

- 在宿主机上运行 AI coding CLIs（如 Codex / Claude / iFlow 等），并通过移动端友好的 PWA 远程监控与控制。
- 以多组件方式交付：Rust workspace（server/hostd）+ Bun CLI（cli）+ Svelte PWA（web）。
- 基于事件模型通过 WebSocket 串联：PWA ↔ server ↔ hostd，并支持远程输入（remote input）。
- hostd 支持离线/断线重连后的事件重放（spool replay）。

## 非目标（Non-goals）

- 该仓库处于 active development；文档中标注为 “skeleton” 的部分不承诺完整功能，只要求能按 README 的方式启动/演示。
- cli 不直接拥有/管理工具进程（由 hostd 负责 PTY/进程相关能力）。
- 事件类型作为稳定 API：不允许在无版本升级的情况下删除/重命名既有事件字段（仅允许兼容性新增字段）。

## 功能需求（Functional Requirements）

### server（Rust）

- 提供认证（auth）。
- 提供路由（routing）。
- 使用 SQLite 作为存储（storage）。
- 提供 WebSocket：
  - 支持 PWA 连接。
  - 支持 hostd 连接。
- 提供密码哈希生成能力（Argon2），用于创建可登录的凭据（见 README 的 `--hash-password` 用法）。

### hostd（Rust）

- 作为宿主机守护进程：
  - 运行 PTY 交互进程（PTY runner）。
  - 以出站方式通过 WebSocket 连接 server（outbound WS）。
- 事件 spooling 与重放：
  - 将待发送事件持久化到本地 SQLite spool DB。
  - 断线后重连时重放未送达事件。
  - 可通过 `SPOOL_DB_PATH` 配置 spool DB 路径（默认：`data/hostd-spool.db`）。

### cli（Bun）

- 作为本地命令包装层，通过 hostd 启动 runs。
- 提供登录能力（示例：`bun run dev login ...`）。
- 提供通过 WebSocket 发送远程输入能力（示例：`bun run dev ws-send-input ...`）。

### web（Svelte PWA）

- 提供移动端友好的 UI。
- 支持：
  - runs 列表与状态查看；
  - 实时日志（live logs）；
  - 审批与输入流转：approve/deny/input。

## 配置与安全（Configuration & Security Requirements）

- 禁止提交 secrets；运行时配置放在 `conf/env`（或 `.env`）并确保被 `.gitignore` 忽略。
- 日志默认短保留（3 days）。
- 输入（inputs）必须以 **redacted** 形式存储；默认关闭 raw input 存储。

## 构建/开发/测试（Build & Test Requirements）

- Rust workspace 支持：
  - `cargo fmt` / `cargo fmt --check`
  - `cargo clippy --all-targets --all-features`
  - `cargo test`
- CLI（Bun）支持：
  - `cd cli && bun install`
  - `cd cli && bun run dev`（若提供）
- Web（Svelte PWA）支持：
  - `cd web && bun install`
  - `cd web && bun run dev`
  - `cd web && bun run build`

## 验收标准（Acceptance Criteria）

以下验收项以“可运行、可验证”为最低标准（不额外扩展未在来源文档中声明的行为）。

- 能按 `README.md` 启动 `relay-server`（在 `conf/env` 配置完成后）。
- 能按 `README.md` 启动 `relay-hostd` skeleton，并连接到 server（通过 `SERVER_BASE_URL` 等环境变量配置）。
- 能按 `README.md` 启动 web skeleton（`cd web && bun run dev`）。
- 能按 `README.md` 方式使用 cli 完成：
  - `login` 获取 token；
  - `ws-send-input` 发送远程输入到指定 `run_id`。
- `hostd` spool replay 行为满足 README 的 E2E smoke test：
  - `scripts/e2e.sh` 能运行完成；
  - 对同一 `input_id` 发送两次输入时，在 SQLite 中满足幂等性断言（idempotency）。

