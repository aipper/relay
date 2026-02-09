<!-- AIWS_MANAGED_BEGIN:claude:ws-req-flow-sync -->
# ws req flow sync

用中文输出；命令与路径保持原样不翻译。

目标：基于 `REQUIREMENTS.md` 的 FlowSpec 生成：
- `docs/api-flow.mmd`（简短逻辑图，Mermaid）
- `issues/server-scenario-issues.csv`（场景执行合同：TODO/DONE/BLOCKED）

执行（在 workspace 根目录）：
`python3 tools/requirements_flow_gen.py --workspace .`

若缺少工具 `tools/requirements_flow_gen.py`：提示用户先运行 `npx @aipper/aiws init .`（默认会安装 optional tools）。

输出要求：只打印生成的文件路径与下一步命令：
- 查看逻辑图：`cat docs/api-flow.mmd`
- 查看场景合同：`cat issues/server-scenario-issues.csv`
<!-- AIWS_MANAGED_END:claude:ws-req-flow-sync -->

可在下方追加本项目对 Claude Code 的额外说明（托管块外内容会被保留）。
