
# Change Proposal: pwa-interaction-v1

> Title: PWA 交互体验优化（对齐 hapi）v1
>
> Created: 2026-03-14T11:41:03Z

## Bindings

- Change_ID: pwa-interaction-v1
- Req_ID: WEB-070
- Additional_Req_IDs: WEB-071, WEB-072, HST-021
- Contract_Row: Req_ID=WEB-070
- Plan_File: plan/2026-03-14_19-37-46-pwa-interaction-v1.md
- Evidence_Path(s): changes/pwa-interaction-v1/evidence/verification-20260318.md

## 目标与非目标

**目标：**
- 提升 PWA（`web/`）交互顺畅度：减少卡顿/抖动/重复渲染
- 对齐 hapi 的交互模式：先规约再渲染（blocks reducer/reconcile）+ 卡片内闭环操作
- 输出视图性能治理：buffer 裁剪、自动滚动控制、搜索
- 降低维护成本：逐步拆分 `web/src/App.svelte`
- 修复“事件视图仍接收海量 `run.output`”与 PWA stale shell 导致的新前端不生效问题
- 打通“启动运行”页的 host-aware 工具探测、`opencode` 模型选择与 cwd 防错体验
- 提升 `opencode` structured 运行可靠性，避免 share 链接/挂起，并支持 per-run model override

**非目标：**
- 不切换传输架构到 Socket.IO/SSE（保持现有 WS fan-out + HTTP API）
- 不在 v1 引入“终端注入/语义 flush/规则引擎”等大能力（另立变更）
- 不做大范围视觉重设计（以交互与性能为主）
- 不做新的 runner 品类扩张；`gemini` 仍不作为与 `codex/claude/iflow/opencode` 同等级的一等 runner 交付

## 变更归因（强制二选一）

- 需求交付：`Req_ID` = WEB-070
- 需求交付（补充）：`Req_ID` = WEB-071, WEB-072, HST-021
- 问题修复：不适用（本次为需求交付）

> 备注：若“问题阻塞需求”，两边都要在各自 CSV 的 `Notes` 字段互相引用对方 ID。

## 现状与问题

- 现状：`web/src/App.svelte` 单文件承载连接/状态/渲染/交互等大量逻辑，长期迭代易引入交互抖动与性能回退。
- 痛点（用户感知）：会话切换、滚动、输入/审批交互不够顺滑；长输出易卡顿。
- 对齐目标：借鉴 hapi 的“规约层 + 可操作卡片 + ready gate”交互模式，在不改协议的前提下提升体验。
- 运行时排障还暴露出几类配套缺口：事件视图仍会接收完整 `run.output`、PWA 可能因 service worker/stale shell 不更新、`opencode` structured 会受到用户全局 config 里的 `share/plugin` 干扰、启动页缺少 host-aware 工具/模型/cwd 防错。

## 方案概述（What changes）

- 新增 `UiBlock[]` 规约层：把 messages/events 规约为稳定 blocks，并通过 reconcile 复用对象降低重渲染抖动
- 组件化卡片：`ToolCard` / `PermissionCard` / `AwaitingInputCard`（在卡片内完成 approve/deny/input）
- 输出性能治理：bufferLines 裁剪、自动滚动控制、搜索
- 结构拆分：逐步把 `App.svelte` 拆为 panes + store（保持行为兼容，渐进式替换）
- 增加消息 API / app WS 的 `include_output` gating，让事件视图与终端视图真正分离 output 回放/订阅策略
- 修复并增强启动体验：host-aware 工具下拉、`opencode` 模型下拉、最近成功 cwd 回填、cwd 错误人类化提示
- 补齐 `opencode` structured 可靠性：隔离全局 config、关闭 share、移除 plugin、stdin 置空、支持 per-run model override

## 影响范围（Scope）

- 影响的服务/模块/目录：
  - `web/src/App.svelte`
  - `web/src/lib/blocks/*`
  - `web/src/lib/components/cards/*`
  - `web/src/lib/stores/*`
  - `server/src/main.rs`
  - `server/src/db.rs`
  - `hostd/src/main.rs`
  - `hostd/src/local_api.rs`
  - `hostd/src/run_manager.rs`
  - `cli/src/index.ts`
  - `docs/protocol.md`
  - `docs/hostd-local-api.md`
- 可能影响的外部接口/使用方：
  - `GET /sessions/:id/messages` 增加 `include_output` 可选参数
  - `run.subscribe` 增加 `include_output` 订阅控制
  - `rpc.run.start` / hostd local API / CLI start 支持可选 `model`
  - `rpc.host.info` / `rpc.host.doctor` 对 `opencode` 返回模型元信息

## 风险与回滚

- 风险：
  - 规约层/拆分导致交互回归（approve/deny/input 状态错乱、滚动/渲染异常）
  - `opencode` 用户侧全局配置格式异常时，structured 调用可能在启动前就失败
  - PWA 已安装旧壳用户可能需要一次彻底重开/刷新，才能切到新 service worker 与 bundle
- 回滚方案（必须可执行）：
  - 回滚 `web/` 相关变更（git revert 或 checkout）
  - 回滚 `server/` / `hostd/` / `cli/` 中与 `include_output` / `opencode structured` / `model` 相关提交
  - 若引入开关（feature flag），可临时切回旧渲染路径或 host 默认模型路径

## 验证计划（必须可复现）

> 从 `AI_WORKSPACE.md` 选择最贴近本变更的验证入口，写成可直接复制执行的命令。

- 命令：
  - `cargo test -p relay-server`
  - `cargo test -p relay-hostd`
  - `cargo build -p relay-server -p relay-hostd`
  - `cd web && bun install`
  - `cd web && bun run build`
  - `bash scripts/e2e.sh`
- 期望结果：
  - web build 通过；PWA 交互手动验收通过（切换/滚动/卡片内交互明显顺滑）
  - 事件视图不再收到 TUI 工具的大量 `run.output`；终端视图仍可查看完整输出
  - `opencode` structured 不再返回 share 链接；显式模型覆盖能产出正常 assistant 回复
  - e2e 不回归（输入幂等等基础链路仍通过）

## 真值文件/合同更新清单

- `REQUIREMENTS.md`：需要（补充 include_output gating、opencode structured 可靠性/per-run model、host-aware start form 条款）
- `requirements/CHANGELOG.md`：需要
- `requirements/requirements-issues.csv`：需要（补齐 WEB-071 / WEB-072 / HST-021，并更新 WEB-070 状态）
- `issues/problem-issues.csv`：不需要
- 证据落盘（repo-local）：`changes/pwa-interaction-v1/evidence/verification-20260318.md`

## 计划文件

- `plan/2026-03-14_19-37-46-pwa-interaction-v1.md`
