# quality-review — paseo-absorb-v1（行为/回归/测试）

- 结论：门禁全绿、隔离栈 E2E 全过；残留 5 个 Warning 级行为风险，无 Critical 功能回归（除提交卫生 Critical 见 codex-review）。
- 覆盖：Rust 20 tests（coalescer 5：首包直发/burst 合并/按 run 独立/类型过滤/默认 5ms；after_seq 1；协议旧形 2）；web build exit 0；Playwright A/B/C/D 各 1 passed（chromium-1208，8790 隔离栈）。
- 行为确认：spool 存原始 chunk（合并仅推送侧）；server 落库 `data_json` 全量透传 actions；PWA 去重按 seq（无内容匹配）；重连单次 backfill。
- 风险：
  - [Warning] custom action：UI 按 approve 发送，hostd approve 路径忽略 selected_action_id → 静默执行 approve_text。建议：UI 对 custom 禁用或标“暂不支持”，或 hostd 显式拒绝并让 UI toast。
  - [Warning] hydrate 复活：`#hydrateAwaitingFromMessages` 不查 decided。建议：一 curl 确认 messages 含 decided；若无则加 decided 检查。
  - [Warning] 跨类型排序：output 缓冲 vs 审批直发。建议：至少记录为已知限制，或让 permission_requested 强制先刷 coalescer。
  - [Warning] backfill 上限 200 无分页；NULL-seq 永不回填。建议：protocol.md 补一句语义。
  - [Info] `/quit` 无反馈；命令表与提示浮层可能不一致。
- 回归：旧形 approve/deny（C4）、旧协议解析（2 单测）、`include_output` 语义（D1 curl）均覆盖。`e2e.sh` codex 失败为预先存在，不计入本 change。
- Next：按 codex-review 最小验证重跑四门禁；提交时显式 add coalescer.rs 并排除 journal/omo/tmp 噪音。
