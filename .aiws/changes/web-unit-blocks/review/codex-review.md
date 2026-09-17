# codex-review: web-unit-blocks

Triage: dual-review: not-required
Rationale: 测试-only，新增 2 个 spec 文件（227 行），零产品代码改动。

Findings:
- [Info][SPEC] 覆盖与计划一致：配对/ok三态/过滤合并/pinned跳过/引用保持/变更替换。
- [Info][QUALITY] 全量 42/42 全绿（~0.3s），build exit 0。
- [Info][REGRESSION] web/ diff 仅 2 新增文件，无回归面。

主要风险（Top risks）:
1. 用例锁定当前语义，未来改分组逻辑需同步改用例（低）。

下一步（Next）: 提交并 finish。
验证：bash -c "cd web && bun run test:unit" 全绿已执行。
