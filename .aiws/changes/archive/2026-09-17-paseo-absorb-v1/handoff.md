# Handoff: paseo-absorb-v1

> Archived: 2026-09-17T08:55:08Z

## 本次完成

- 按性价比顺序把 paseo（`getpaseo/paseo@6830c46`）四项设计落到 relay，全后向兼容（只加字段、不改名、不删字段）：

## 改动文件

- (see git log for details)

## 关键决策

- 跳过二进制帧：瓶颈在包数而非编码，合并器已解决主要成本；JSON envelope 保留可读性与可审计性。
- `selected_action_id` 全链路 Optional：protocol/hostd/server/web 逐层透传未知字段，未知 behavior fail-closed。
- 回填以 seq 为唯一真值：`maxSeqByRun` + 单 fetch 守卫 + seq 去重；NULL-seq 事件不参与回填（started 等元事件）。
- 证据策略：隔离栈（dev-up.sh --port 8790 + chromium-1208）做真实 E2E，标准 mock 套件只做回归参考。

## 协同记录

- analysis: 0 file(s)
- patches: 0 file(s)
- review: 3 file(s)
  - .aiws/changes/archive/2026-09-17-paseo-absorb-v1/review/codex-review.md
  - .aiws/changes/archive/2026-09-17-paseo-absorb-v1/review/quality-review.md
  - .aiws/changes/archive/2026-09-17-paseo-absorb-v1/review/spec-review.md
- evidence: 9 file(s)
  - .aiws/changes/archive/2026-09-17-paseo-absorb-v1/evidence/phase-a.md
  - .aiws/changes/archive/2026-09-17-paseo-absorb-v1/evidence/phase-b-assert.spec.ts
  - .aiws/changes/archive/2026-09-17-paseo-absorb-v1/evidence/phase-b.md
  - .aiws/changes/archive/2026-09-17-paseo-absorb-v1/evidence/phase-c-assert.spec.ts
  - .aiws/changes/archive/2026-09-17-paseo-absorb-v1/evidence/phase-c.md
  - .aiws/changes/archive/2026-09-17-paseo-absorb-v1/evidence/phase-d-assert.spec.ts
  - .aiws/changes/archive/2026-09-17-paseo-absorb-v1/evidence/phase-d.md
  - .aiws/changes/archive/2026-09-17-paseo-absorb-v1/evidence/verification.jsonl
  - ...(truncated)

## 下一步建议

- (no Blocks declared)

## 绑定

- Change_ID: paseo-absorb-v1
- Req_ID: PAS-000
