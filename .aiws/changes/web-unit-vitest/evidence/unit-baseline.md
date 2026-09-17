# unit-baseline — web-unit-vitest (2026-09-17)

- `cd web && bun run test:unit` → Test Files 1 passed, Tests 26 passed, Duration ~0.3s
- `cd web && bun run build` → exit 0（PWA precache 5 entries）
- 范围说明：首批用例覆盖 `src/lib/stores/utils.ts` 纯函数（truncateTail/truncateHead/sanitizeTerminalOutput/applyTerminalEdits/toWsBase/isProbablyInsecureUrl/dataString/dataBool/parseHostToolStatuses/compareTsDesc/basename/statusLabel/computeOutputMatches）；relay-store 类因 Svelte5 runes + 浏览器依赖（localStorage/window/WS/RAF）在 node 下无法实例化，本轮跳过直测，seq 语义仅由 truncateHead 等纯函数行为级覆盖。
