# codex-review — pwa-e2e-green（2026-09-17）
Triage: dual-review: not-required
Rationale: 纯测试文件改动（web/tests/e2e），无产品代码、无 REQUIREMENTS 影响、无安全/数据一致性风险。

## Findings
- [Info][SPEC] 10 个失败均为测试漂移（旧 UI 文本/选择器、mock 缺 WebSocket 静态常量、strict-mode 多匹配），非产品回归；PWA-E2E-001 问题归因成立。
- [Info][QUALITY] 全量 99 passed (4.7m)，desktop/tablet/mobile 各 33；日志 /tmp/pw-green.log。
- [Warning][QUALITY] interaction.spec 用 first() 规避 modal+内联双命中：若未来只剩一种渲染，该断言仍通过但覆盖变弱，可接受。
- [Warning][REGRESSION] auth「记住登录状态」改走设置页：覆盖了 keepSignedIn→localStorage 链路，但不再覆盖登录页本身无残留文本；可接受（登录页改动由渲染用例覆盖）。
- [Info][SPEC] session-new.spec.ts 末尾仍无换行（沿用原文件状态，未引入）。

## 后缀审计
- 本 change 未写 workflow-state 后缀文件；plan 阶段由主 session 标记 DONE。无混用。

## Top risks
1. mock WebSocket 与真实浏览器语义仍有差距（静态常量刚补齐，未来产品若用更多 WS 特性会再漂移）。
2. placeholder 文本定位（ses_xxx/…）与产品文案耦合，改文案即坏测试。
3. 双搜索框（SessionsPage vs SessionList）产品冗余仍在，本次未动。

Next: commit 测试文件 + evidence，然后 verify-bc or 直接 finish（测试-only change）。
最小验证：cd web && ./node_modules/.bin/playwright test（已跑：99 passed）。
