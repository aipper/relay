# Tasks: pwa-interaction-v1

> Title: PWA 交互体验优化（对齐 hapi）v1
>
> Created: 2026-03-14T11:41:03Z

## 0. Preflight

- [x] 0.1 阅读并遵守 `AI_PROJECT.md` / `AI_WORKSPACE.md` / `REQUIREMENTS.md`
- [x] 0.2 运行门禁校验：`aiws validate .`（或 `npx -y @aipper/aiws validate .`）
- [x] 0.3 若真值文件发生变化（例如你更新了 REQUIREMENTS.md），同步基线：`aiws change sync pwa-interaction-v1`

## 1. 需求/问题合同（如适用）

- [x] 1.1 需求交付：补齐/更新 `REQUIREMENTS.md` 验收条款（或确认不需要）
- [x] 1.2 同步 `requirements/requirements-issues.csv`（或更新 `issues/problem-issues.csv`）
- [x] 1.3 记录到 `requirements/CHANGELOG.md`（如需求发生变化）

## 2. 实现

- [x] 2.1 Phase 1：新增 `web/src/lib/blocks/`，实现 `reduceToBlocks` + `reconcileBlocks`
- [x] 2.2 Phase 2：新增 `web/src/lib/components/cards/`（ToolCard/PermissionCard/AwaitingInputCard）并接入 blocks 渲染
- [x] 2.3 Phase 3：输出视图治理（buffer 裁剪、自动滚动、搜索）
- [x] 2.4 Phase 4：拆分 `App.svelte` 到 panes + store（渐进式替换）
- [x] 2.5 事件视图 output gating：`messages` API 与 app WS 支持 `include_output`，TUI 会话默认不向事件视图回放/订阅海量 `run.output`
- [x] 2.6 修复 PWA stale shell：恢复 PWA 产物生成、加强 `manifest/sw/registerSW/index.html` 缓存策略、提升 `uiVersion`
- [x] 2.7 启动页增强：host-aware 工具下拉、`opencode` 模型选择、最近成功 cwd 回填、cwd 错误人类化提示
- [x] 2.8 `opencode` structured 可靠性与模型透传：隔离全局 config、关闭 share、移除 plugin、stdin 置空、host info/local API/CLI/RPC 支持 `model`

## 3. 验证（必须可复现）

- [x] 3.1 `cd web && bun install && bun run build`
- [x] 3.2 手动验收：切换会话/滚动/卡片内 approve/deny/input 顺畅；输出不内存爆炸
- [x] 3.3 `bash scripts/e2e.sh`
- [x] 3.4 `cargo test -p relay-server`
- [x] 3.5 `cargo test -p relay-hostd`
- [x] 3.6 `cargo build -p relay-server -p relay-hostd`
- [x] 3.7 按项目规则重启 `relay-system`，并确认 `/health` 与 hostd unix socket LISTEN
- [x] 3.8 真实 run 验证：事件视图 output gating、生效的新 PWA bundle、`opencode` structured 回复、显式模型覆盖

## 4. 交付与归档

- [x] 4.1 证据落盘到 `changes/pwa-interaction-v1/evidence/...`（并引用 AIWS stamps）
- [ ] 4.2 交叉审计（可选但推荐）：在 AI 工具内运行 `/ws-review`（或按 `AI_PROJECT.md` 手工审计）
- [ ] 4.3 归档：`aiws change archive pwa-interaction-v1`
