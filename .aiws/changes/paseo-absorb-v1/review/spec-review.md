# spec-review — paseo-absorb-v1（流程/真值归因）

- 结论：实现与 REQUIREMENTS 验收方向一致，但归因合同未闭环，finish 被阻断直到合同补齐或豁免。
- 归因：proposal 声明 Req H3（tool/permission）/M2（web 实时）/M3（审批）/M5（sessions/messages）+ spool replay。代码改动可映射：协议 actions→H3/M3；coalescer→M2；after_seq/backfill→M5；slash→体验优化（无直接 Req，可接受为 H2 可读性附带）。
- 兼容性：全新增 `Option` + `skip_serializing_if` + 旧形单测 2 个，符合“事件字段只增不改”真值。PWA 无 actions 回退旧按钮、hostd 缺 selected_action_id 回退 envelope type，均已验证。
- 非目标：无二进制帧/adapter/runner 改动；线上 `:8787` 未碰。合规。
- 缺口（阻断 finish）：`Contract_Row=TBD`，`requirements/requirements-issues.csv` 缺失，tasks 1.1–1.3、4.1–4.3 未完成。`aiws change validate` 当前仅警告，通过不代表归因闭环。
- 证据：proposal.md、tasks.md（2.x/3.x 全勾）、`evidence/phase-{a,b,c,d}.md`、plan 文件 `plan/2026-09-17_11-04-38-paseo-absorb.md`（plan-verify 已过）。
- Next：补一行最小合同（H3/M2/M3/M5）或书面豁免；勾/推迟 tasks 1.x/4.x；再跑 `$ws-verify-before-complete`。
