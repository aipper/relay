# Plan: OpenCode-first PWA focus

## Bindings

- Change_ID: opencode-pwa-focus
- Req_ID: WEB-073
- Additional_Req_IDs: WEB-074, WEB-075, HST-022
- Contract_Row: Req_ID=WEB-073, Req_ID=WEB-074, Req_ID=WEB-075, Req_ID=HST-022
- Plan_File: plan/2026-03-19_11-02-10-opencode-pwa-focus.md
- Evidence_Path(s): .agentdocs/tmp/opencode-pwa-focus-feasibility-20260319.md, changes/opencode-pwa-focus/evidence/change-status-20260319-070517Z.json, changes/opencode-pwa-focus/evidence/change-validate-strict-20260319-070517Z.json, changes/opencode-pwa-focus/evidence/aiws-validate-stamp-20260319-070517Z.json, changes/opencode-pwa-focus/evidence/change-sync-stamp-20260319-070517Z.json, changes/opencode-pwa-focus/evidence/delivery-summary-20260319-070517Z.md

## Goal

在保持现有 `server/hostd/web/cli` 架构的前提下，把产品重心调整为 **OpenCode-first**：

1. PWA 能按 OpenCode 语义列出并切换 session
2. PWA 能新开 session / fork session / 查看对话历史
3. PWA 能按 host 展示并切换 OpenCode 模型
4. PWA 能展示 session 级 todo / 计划
5. 当任务完成后，PWA 上的计划状态能自动更新，而不是只靠手动勾选

## Non-goals

- 不替换为 OpenCode 官方 web UI，也不要求 relay 直接内嵌官方前端
- 不在本轮引入多用户/租户/端到端加密同步
- 不把所有现有 runner 都按 OpenCode 方式重写；本轮只聚焦 `opencode`
- 不依赖原始终端文本解析来长期维持计划状态；文本解析只允许作为过渡兜底

## Scope

- 本次计划涉及 `REQUIREMENTS.md`、`requirements/requirements-issues.csv`、`docs/protocol.md`、`hostd/src/*`、`server/src/*`、`web/src/*`、`cli/src/index.ts`，聚焦 OpenCode-first 的 session/model/todo 语义，不扩展到其他 runner 的同等级改造。

### In scope

- `REQUIREMENTS.md`
- `requirements/requirements-issues.csv`
- `docs/protocol.md`
- `hostd/src/run_manager.rs`
- `hostd/src/main.rs`
- `hostd/src/local_api.rs`
- `server/src/db.rs`
- `server/src/main.rs`
- `web/src/App.svelte`
- `web/src/lib/blocks/*`
- `cli/src/index.ts`

### Out of scope for the first implementation wave

- `codex` / `claude` / `iflow` 的同等级 OpenCode 化改造
- 多端同步冲突处理
- 历史 transcript 全量迁移/回填
- 大范围视觉重设计

## Submodules

- 本仓库当前无 `.gitmodules` 条目；本计划无 submodule target truth 要求。

## Feasibility assessment

### Confirmed feasible now

- OpenCode 原生已支持 session create/list/get/messages/fork/todo/model override。
- 当前仓库已具备：
  - `hostd` structured OpenCode 执行与 per-run model override
  - host model discovery -> PWA model picker
  - relay generic `/sessions` + `/sessions/:id/messages`
  - PWA 的 session/messages/todo 雏形

### Missing for product-complete delivery

- OpenCode `sessionID` 未成为 relay 的一等字段
- server 无 session todo 持久化/代理层
- PWA todo 仅 `localStorage`，无自动完成
- CLI 无 OpenCode-first session/todo/history 面
- 当前需求真值尚未把“OpenCode-first”定义为正式交付目标

### Verdict

- **可实现，且应作为新一轮需求/协议/数据模型调整来做。**
- 推荐路线：保留 relay 的远程控制架构，把 OpenCode 的 session/todo/model 能力映射为 relay 的一等领域对象。

## Plan

- 执行顺序：先补需求真值与合同，再提升 `opencode_session_id` 为一等元数据，然后补 server 代理层、PWA OpenCode-first 信息架构、todo 自动完成与 CLI 对齐能力。

### Phase 0 — 先修真值，不直接写功能

1. 运行 `/ws-req-review`，把“项目转向 OpenCode-first”定为正式需求调整。
2. 用 `/ws-req-change` 更新 `REQUIREMENTS.md` 与 `requirements/requirements-issues.csv`，新增至少以下需求条目：
   - OpenCode native session identity persistence
   - PWA session list/switch/new/fork/history
   - PWA OpenCode todo list and auto-complete
   - CLI OpenCode-first session/todo commands
3. 在 `docs/protocol.md` 写清新增字段/接口的兼容策略，确保 additive 扩展。

### Phase 1 — 提升 OpenCode session identity 为一等模型

目标：relay 不再只知道 `run_id`，而是明确知道该 run 绑定的 OpenCode `sessionID`。

1. `hostd/src/run_manager.rs`
   - 把 structured 模式拿到的 `opencode_session_id` 暴露为可持久化元信息。
   - 在 `run.started` / 后续事件中带上 OpenCode session metadata（新增字段，保持向后兼容）。
2. `server/src/db.rs`
   - 为 `runs` 增加 OpenCode session identity 字段，或增加独立映射表。
3. `server/src/main.rs`
   - `GET /sessions` / `GET /sessions/:id` 补充 OpenCode session metadata。

### Phase 2 — 让 server 成为 OpenCode-first proxy layer

目标：PWA/CLI 不只消费 relay generic messages，还能拿到 OpenCode-native session/todo 能力。

1. 设计并实现最小 API 扩展（任选其一，但要统一）:
   - A. relay server 直接代理 OpenCode-native session/todo endpoints
   - B. relay server 以自己的资源名暴露，但字段语义对齐 OpenCode
2. 至少补齐：
   - session list
   - session create
   - session fork
   - session messages/history
   - session todo list
3. 继续保持现有 `run`/event store 用于审计与远程控制，不做破坏性替换。

### Phase 3 — PWA 改为 OpenCode-first 信息架构

目标：用户在 PWA 里看到的是“OpenCode 会话工作台”，不是泛化 run 控制台。

1. `web/src/App.svelte` / `web/src/lib/blocks/*`
   - 列表优先展示 OpenCode session 语义（新开、切换、fork、历史）。
   - 详情页增加清晰的 OpenCode session 标识与切换入口。
2. 启动页
   - 保留 host-aware tool 能力，但当 tool=`opencode` 时展示完整模型/会话入口。
3. todo 区
   - 把当前 `localStorage` todo 雏形替换或升级为 server-backed session todo。
   - 使用结构化 task/todo 数据驱动 UI，而不是只解析 `TODO:` 文本。

### Phase 4 — 自动完成计划状态

目标：当 OpenCode 内部任务完成，PWA 的计划状态自动更新。

1. 优先方案：消费 OpenCode 的结构化 todo/task 状态并映射到 UI。
2. 次优方案：若 OpenCode 当前接口只暴露 todo snapshot，则以 server 定时/事件刷新为准更新 completed 状态。
3. 过渡兜底：仅在结构化信息缺失时，允许保留 `- [x]` / `TODO:` 的文本解析补充，但不得作为长期唯一来源。

### Phase 5 — CLI 补齐 OpenCode-first 操作面

目标：CLI 与 PWA 能力对齐，便于调试与回归。

1. `cli/src/index.ts` 增加只读/控制命令（命名可后定）：
   - session list/get/messages
   - session create/fork
   - todo list
2. CLI 命令优先复用 server API，不直接绕过 relay 访问本地实现。

## Verify

- 先做 planning/contract 校验，再跑 Rust 与 web 构建验证，最后做 OpenCode-first 的手动验收路径（新建/切换/fork/history/todo 自动更新）。
- 命令：`python3 tools/requirements_contract.py validate`、`aiws validate .`、`cargo fmt --check`、`cargo clippy --all-targets --all-features`、`cargo test --workspace`、`cd web && bun install && bun run build`
- 期望结果：合同校验通过；AIWS 校验通过；Rust 格式化/静态检查/测试退出码均为 0；web build 退出码 0；PWA 手动验收可完成 OpenCode session 新建/切换/fork/history/todo 自动更新。

### Planning / truth alignment

- `python3 tools/requirements_contract.py validate`
  - 期望：新增或更新后的 requirement rows 可通过合同校验。
- `aiws validate .`
  - 期望：真值、contract、change artifacts 无阻断错误。

### Rust backend

- `cargo fmt --check`
  - 期望：退出码 0
- `cargo clippy --all-targets --all-features`
  - 期望：退出码 0
- `cargo test --workspace`
  - 期望：退出码 0

### Web / PWA

- `cd web && bun install && bun run build`
  - 期望：退出码 0

### Manual acceptance for the OpenCode-first slice

1. 在 PWA 选择某 host，tool=`opencode` 时能看到可用模型与默认模型。
2. 能新建 OpenCode session。
3. 能切换已有 OpenCode session，并看到历史消息。
4. 能 fork 当前 session。
5. 能列出 session todo。
6. 当 OpenCode todo 状态变化后，PWA 上对应计划项自动完成或刷新完成状态。

## Risks & Rollback

- 主要风险是把 OpenCode 会话/待办语义错误地退化成 generic run/output 文本；回滚策略是保持所有新增字段/API 为 additive，并保留现有 generic `/sessions` 与 localStorage todo 作为兼容 fallback。

### Risks

- OpenCode CLI JSON 事件与 server API 语义并不完全等价，若只靠 `run --format json` 可能出现状态缺口。
- 若把 OpenCode session identity 加得过深，旧的 generic run consumers 可能出现兼容问题。
- 若直接把当前 local todo 替换掉，可能导致已有浏览器本地待办丢失。

### Rollback

- 协议层：新增字段必须 additive；回滚时仅关闭新字段消费，不移除旧字段。
- server/web：保留现有 generic `/sessions` + localStorage todo 路径作为临时兼容 fallback。
- hostd：保留当前 structured mode 行为，不在第一次迭代里破坏已验证的 model override / output mapping。

## Evidence

- 评审证据：`.agentdocs/tmp/opencode-pwa-focus-feasibility-20260319.md`
- 本计划：`plan/2026-03-19_11-02-10-opencode-pwa-focus.md`

## Next

1. 先执行 `/ws-req-review`
2. 再执行 `/ws-req-change`
3. 真值更新通过后再进入 `/ws-plan-verify`
4. 通过后进入 `/ws-dev`
