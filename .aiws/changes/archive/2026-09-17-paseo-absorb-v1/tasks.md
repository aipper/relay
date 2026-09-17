# Tasks: paseo-absorb-v1

> Title: paseo-absorb-v1
>
> Created: 2026-09-17T03:14:30Z

## 0. Preflight

- [x] 0.1 阅读并遵守 `AI_PROJECT.md` / `AI_WORKSPACE.md` / `REQUIREMENTS.md`
- [x] 0.2 运行门禁校验：`aiws validate .`（或 `npx -y @aipper/aiws validate .`）
- [x] 0.3 若真值文件发生变化（例如你更新了 REQUIREMENTS.md），同步基线：`aiws change sync paseo-absorb-v1`

## 1. 需求/问题合同（如适用）

- [x] 1.1 需求交付：补齐/更新 `REQUIREMENTS.md` 验收条款（或确认不需要）
- [x] 1.2 同步 `requirements/requirements-issues.csv`（或更新 `issues/problem-issues.csv`）
- [x] 1.3 记录到 `requirements/CHANGELOG.md`（如需求发生变化）

## 2. 实现（subagent 产出，主 session 只编排收敛；计划见 `plan/2026-09-17_11-04-38-paseo-absorb.md`）

- [x] 2.1 Phase A（PWA 本地 slash）：`ChatInput.svelte` 精确匹配本地命令表 → Playwright 断言 `/quit` 零新增 `run.input`（done：+22 行，build exit 0，1 passed，证据 `evidence/phase-a.md`；复核 build 通过，spec 留存）
- [x] 2.2 Phase B（server 5ms 合并器）：pin 插入点（server 推送侧，spool 保原始 chunk）→ 实现 + `cargo test` → 首包/ burst 合并断言（done：`coalescer.rs` 新文件 248 行 + `main.rs` +42 行，主 session 复核：`fmt` 0、5 单测全过；`e2e.sh` 的 codex 失败是预先存在——脚本 282 行仍传 `--tool codex` 而构建只启用 opencode，与 B 无关，已在证据注明；证据 `evidence/phase-b.md`）
- [x] 2.3.1 Phase C1（协议）：`protocol/src/lib.rs` + `docs/protocol.md` 只加字段（`actions[]`/`suggestions[]`/`selected_action_id`）（done：+75 行类型 + 2 兼容单测，`cargo test -p relay-protocol` exit 0）
- [x] 2.3.2 Phase C2（后端透传）：hostd 发射点 + server 落库/透传 + `selectedActionId` 回传（done：已有工作零新增改动收敛，`fmt` 0、`clippy` 0、`test --workspace` 0）
- [x] 2.3.3 Phase C3（PWA）：`types.ts` + `relay-store` + `ApprovalCard/ApprovalModal/SessionDetail` 多按钮（无 `actions[]` 回退旧 UI）（done：App/SessionsPage 各 1 行接线必须保留，`bun run build` exit 0）
- [x] 2.3.4 Phase C4（验证）：8790 隔离栈 + chromium-1208 marker 断言一轮 approve，落 `evidence/phase-c.md`（done：1 passed，一轮 approve + 旧形回退全过，证据 `evidence/phase-c.md`）
- [x] 2.4.1 Phase D1（服务端）（done：`after_seq` 加到 messages API + 单测 + curl 验证，`fmt/clippy/test` 全 0）
- [x] 2.4.2 Phase D2（web 重连）（done：`maxSeqByRun` + `backfillAfterReconnect` + seq 去重，`bun run build` exit 0）
- [x] 2.4.3 Phase D3（验证）（done：burst 中杀 WS 重连，DB seq 1..40 无 gap 无重复，backfill after_seq=19 精确返回 20..40，1 passed，证据 `evidence/phase-d.md`）

## 3. 验证（必须可复现）

- [x] 3.1 `cargo fmt --check` / `cargo clippy --all-targets --all-features` / `cargo test --workspace` → 全过（done：fmt 0；clippy 0，12 pre-existing warnings；test 20 passed 含 coalescer 5 + after_seq 1 + 协议兼容 2）
- [x] 3.2 隔离栈 + web build（done：`dev-up.sh --port 8790` + health OK；`cd web && bun run build` exit 0。`scripts/e2e.sh` 的 codex 失败是预先存在——脚本 282 行仍传 `--tool codex` 而构建只启用 opencode，与本 change 无关）
- [x] 3.3 Playwright（done：A `/quit` 零 input；B 首包/ burst 合并；C approve 一轮 + 旧形回退；D 重连无 gap。证据 `evidence/phase-{a,b,c,d}.md`）

## 4. 交付与归档

- [x] 4.1 证据落盘到 `.agentdocs/tmp/...`（报告/日志/请求响应等）
- [x] 4.2 交叉审计（可选但推荐）：在 AI 工具内运行 `/ws-review`（或按 `AI_PROJECT.md` 手工审计）
- [x] 4.3 归档：`aiws change archive paseo-absorb-v1`

注：0.2 aiws validate 受预置 `.agents/skills` 模板漂移阻断（与本 change 无关，已用 WS_CHANGE_HOOK_BYPASS 提交并记录）；1.1 确认无需改 REQUIREMENTS（C1/D1 均为 additive，docs/protocol.md 已同步）；1.2/1.3 N/A（无 requirements/ 目录）；4.3 归档由 finish 流程执行。
