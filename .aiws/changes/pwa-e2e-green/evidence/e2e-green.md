# pwa-e2e-green — 全绿证据（2026-09-17）
- 全量：`cd web && ./node_modules/.bin/playwright test` → **99 passed (4.7m)**，0 failed（desktop/tablet/mobile 各 33）
- 改动：仅 web/tests/e2e（6 spec + fixtures/app.ts），产品代码零改动
- 根因：测试漂移（旧 UI 文本/选择器）+ mock WebSocket 缺静态常量（fixtures/app.ts 补 OPEN/CONNECTING/CLOSING/CLOSED）+ strict-mode 多匹配（exact:true / first()）
- 日志：/tmp/pw-green.log；test-results/.last-run.json status=passed
