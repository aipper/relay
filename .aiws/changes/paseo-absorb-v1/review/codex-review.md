# codex-review — paseo-absorb-v1（通用审计）

- Change: `paseo-absorb-v1`（分支 `change/paseo-absorb-v1`）
- Scope: 18 tracked files +486/-33（`git diff HEAD --stat`），另有 `server/src/coalescer.rs`（248 行）等 untracked 新文件
- Triage: dual-review: required
- Rationale: full-stack（protocol + hostd + server + web）+ 协议字段新增 + 数据一致性（seq/审批回放），且准备 finish
- Spec review scope: 归因绑定（Req H3/M2/M3/M5）与 Contract_Row TBD、proposal 非目标遵守
- Quality review scope: coalescer/审批回传/backfill 三条新路径的行为回归与边界

## 约束摘要（真值）

- `AI_PROJECT.md`：不写 secrets、不做破坏性操作；改动必须归因（需求 Req_ID 或问题 Problem_ID）；server/hostd 改动必须重编重启 + 清 `relay-run-*` + 健康检查；web 展示/事件解析改动至少 `bun run build`；每轮至少一个可追溯产物
- `REQUIREMENTS.md`：事件字段只允许兼容新增（本 change 遵守：全 `Option` + `skip_serializing_if`）；M2/M3/M5 与 H3 覆盖审批/messages；当前仅 `opencode` 为受支持 tool（e2e 的 codex 传参失败属预先存在）；`scripts/e2e.sh` 幂等断言为最低门禁
- `AI_WORKSPACE.md`：Rust `fmt/clippy/test`、web `build`、隔离栈 `dev-up.sh --port 8790` + chromium-1208 Playwright 为本 change 实际验证入口

## Findings

- [Warning][SPEC] `Contract_Row = TBD`，`requirements/requirements-issues.csv` 不存在，tasks 1.1–1.3 / 4.1–4.3 未勾。finish 前必须补最小合同或走 `ws-req-change` 豁免，否则归因门禁不闭环。
- [Critical][QUALITY] `server/src/coalescer.rs` 为 untracked 新文件，但 `server/src/main.rs` 已 `mod coalescer;`。若只 `git add -u`（不含 untracked）提交，新检出直接编译失败。commit 时必须显式 `git add server/src/coalescer.rs`（及 `evidence/*.spec.ts` 等意图内文件）。
- [Warning][REGRESSION] 暂存卫生：`M .aiws/journal/2026-09-17.jsonl`、`M .omo/run-continuation/*.json`、`D web/test-results/.last-run.json`、`?? web/.aiws-tmp-pw/`、`?? .omo/...` 不应进本 change 提交。建议只 staged 意图内 16 个源码/文档文件 + coalescer.rs + change 工件。
- [Warning][QUALITY] custom 行为语义分裂：PWA `sendActionDecision` 把未知/`custom` behavior 映射为 `approve + selected_action_id`，而 hostd `decide_permission_with_action` 在 `decision=approve/deny` 时忽略 `selected_action_id`（直接取 approve_text/deny_text），仅当 decision 非法时才按 action behavior 路由（custom 直接 Err）。结果：custom 按钮点击后静默执行 approve 文本，而非报错或按自定义语义处理，且 UI 无任何提示。当前发射点只发 approve/deny 故无生产影响，但协议注释与 UI 行为不一致。
- [Warning][QUALITY] coalescer 排序风险：`run.output` 缓冲 5ms 而 `permission_requested` 等直发；burst 尾部 output 可能排在审批卡之后到达。概率低、影响仅为显示顺序，但未被任何单测/Playwright 覆盖。
- [Warning][QUALITY] `backfillAfterReconnect` 单页 `limit=200` 无分页；gap > 200 时残留缺口无二次追赶。当前 burst 规模（40）下通过，属已知上限。
- [Warning][QUALITY] `after_seq` 过滤掉 NULL-seq 事件（SQL `seq > ?5` 对 NULL 为假）。`run.started` 等 NULL-seq 事件永远不会出现在 backfill 页。当前 backfill 只做增量合并（不替换全量），故可接受，但语义应在 `docs/protocol.md` 补一句。
- [Warning][REGRESSION] `#hydrateAwaitingFromMessages` 取“最新一条 `permission_requested`”重建 awaiting，未检查该 request 是否已有 `permission_decided`。若 messages 流不含 decided 事件，切会话/backfill 可能复活已决审批卡。C4 证据恰好记录了反向现象（stale run 因 limit=200 渲染不出卡），说明该路径对“旧请求”敏感，建议 finish 前确认 decided 事件是否在 messages 集合内。
- [Info][QUALITY] `/quit` 本地截断后无任何用户反馈（draft 清空而已）；命令表仅 `/quit /clear`，若命令提示浮层展示更多命令会造成“有些进 PTY 有些不进”的认知差。
- [Info][SPEC] 非目标遵守良好：无二进制帧、无 provider-adapter、无 runner 语义改动；线上 `:8787` 问题未碰（proposal 已声明另起 change）。

## 验证复核（已执行，主张可信）

- `cargo fmt --check` exit 0；`cargo clippy --all-targets --all-features` exit 0（12 pre-existing warnings）；`cargo test --workspace` exit 0（20 passed：coalescer 5 + after_seq 1 + 协议兼容 2 + 其余）
- `cd web && bun run build` exit 0
- 隔离栈 `--port 8790` + chromium-1208：A（`/quit` 零 input）、B（首包直发/burst 合并）、C（approve 一轮 + 旧形回退）、D（重连无 gap，DB 1..40 连续）各 1 passed；证据 `evidence/phase-{a,b,c,d}.md` 齐全
- `scripts/e2e.sh` 的 codex 失败为预先存在（脚本仍传 `--tool codex`，构建仅 opencode），与本 change 无关——但 finish 门禁若卡 e2e 全绿，需另起 change 修脚本，不应在本 change 内顺手修

## Workflow State Suffix 审计

- tasks.md/proposal.md 内未发现 `[workflow-state:*]` 后缀混用；plan 阶段标记按 skill 约定在对话层完成，无文件级异常。无修正项。

## Top risks（高→低）

1. 新文件未跟踪导致提交后构建断裂（coalescer.rs）——提交命令写错即中招
2. 暂存区混入 journal/omo/tmp 噪音——review/commit 误带无关文件
3. 合同行 TBD——finish 归因门禁不闭环
4. 已决审批卡被 hydrate 复活——待确认 decided 事件可见性
5. custom action 静默按 approve 执行——语义与注释不符
6. coalescer 与审批事件的跨类型排序——未覆盖
7. backfill 200 上限与 NULL-seq 语义——大 gap/旧事件场景

## Next（最小修复 + 最小验证）

- [ ] 提交前显式 add 意图内文件，排除噪音：
  `git add server/src/coalescer.rs protocol/src/lib.rs server/src/db.rs server/src/main.rs hostd/src/run_manager.rs hostd/src/main.rs hostd/src/local_api.rs docs/protocol.md web/src/App.svelte web/src/lib/components/ApprovalCard.svelte web/src/lib/components/ApprovalModal.svelte web/src/lib/components/ChatInput.svelte web/src/lib/components/SessionDetail.svelte web/src/lib/pages/SessionsPage.svelte web/src/lib/stores/relay-store.svelte.ts web/src/lib/stores/types.ts .aiws/changes/paseo-absorb-v1/proposal.md .aiws/changes/paseo-absorb-v1/tasks.md .aiws/changes/paseo-absorb-v1/evidence/phase-a.md .aiws/changes/paseo-absorb-v1/evidence/phase-b.md .aiws/changes/paseo-absorb-v1/evidence/phase-c.md .aiws/changes/paseo-absorb-v1/evidence/phase-d.md`
  并确认 `git status --short` 无 `.aiws/journal` / `.omo/` / `web/.aiws-tmp-pw/` / `test-results/`
- [ ] 二选一闭环归因：补最小 `requirements/requirements-issues.csv` 行（H3/M2/M3/M5）或在 proposal 记录 `ws-req-change` 豁免理由；勾 tasks 1.x/4.x 或明确推迟项
- [ ] 确认 messages 是否含 `permission_decided`（一 curl 即可）：`GET /runs/:id/messages?limit=200` 看已决 run 是否含 decided；若无，hydrate 加 decided 检查或记录为已知限制
- [ ] 最小验证：`cargo fmt --check && cargo clippy --all-targets --all-features && cargo test --workspace`（expect 全 0 / 20 passed）；`cd web && bun run build`（expect exit 0）

## Resolution 2026-09-17 (close-out triage)
- [Critical][QUALITY] coalescer.rs untracked → RESOLVED: commit 2ef2d06 显式加入 server/src/coalescer.rs；此后 cargo fmt/clippy/test 全绿，无新检出编译问题。
