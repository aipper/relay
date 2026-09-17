# evidence: blocks 单测覆盖

命令：bash -c "cd web && bun run test:unit"
结果：Test Files 3 passed (3) / Tests 42 passed (42) / Duration ~0.3s
门禁：bash -c "cd web && bun run build" exit 0
新增：web/src/lib/blocks/reduce.test.ts、web/src/lib/blocks/reconcile.test.ts（16 新用例）
产品代码：零改动（git status 仅测试文件新增 + CSV 合同行）
