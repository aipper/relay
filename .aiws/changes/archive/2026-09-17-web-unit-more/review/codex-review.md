# review: web-unit-more
Triage: dual-review: not-required
Rationale: 只新增一个测试文件，零产品改动。
- [Info][SPEC] 12 个目标纯函数均在 utils.ts 内可测范围；window 依赖的 inferDefaultApiBaseUrl 已显式跳过。
- [Info][QUALITY] 70/70 全绿（主 session 实跑复核），build exit 0。
- [Info][REGRESSION] 无回归面（无产品 diff）。
Top risks: 无。
Next: 用户确认后提交并 finish。
