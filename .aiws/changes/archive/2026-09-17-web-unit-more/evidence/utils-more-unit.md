# evidence: utils 剩余纯函数单测（web-unit-more）
- 新增 `web/src/lib/stores/utils-extra.test.ts`（184 行），12 函数 × ≥2 用例。
- `bash -c "cd web && bun run test:unit"`：4 files / 70 tests passed（~0.36s，含既有 42）。
- `bash -c "cd web && bun run build"`：exit 0。
- 零产品改动；`inferDefaultApiBaseUrl` 跳过（读 window.location）。
