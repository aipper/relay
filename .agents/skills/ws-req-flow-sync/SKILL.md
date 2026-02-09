---
name: ws-req-flow-sync
description: Flow 同步（生成 api-flow 与场景执行合同）
---

用中文输出；命令与路径保持原样不翻译。

目标：基于 `REQUIREMENTS.md` 的 FlowSpec 生成：
- `docs/api-flow.mmd`（简短逻辑图，Mermaid）
- `issues/server-scenario-issues.csv`（场景执行合同：TODO/DONE/BLOCKED）

执行（在 workspace 根目录）：
`python3 tools/requirements_flow_gen.py --workspace .`

若缺少工具 `tools/requirements_flow_gen.py`：提示用户先运行 `$aiws-init`（默认会安装 optional tools），或执行：
```bash
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws init .
elif command -v aiws >/dev/null 2>&1; then
  aiws init .
else
  npx @aipper/aiws init .
fi
```

输出要求：只打印生成的文件路径与下一步命令：
- 查看逻辑图：`cat docs/api-flow.mmd`
- 查看场景合同：`cat issues/server-scenario-issues.csv`
