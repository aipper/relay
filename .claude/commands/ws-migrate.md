<!-- AIWS_MANAGED_BEGIN:claude:ws-migrate -->
# ws migrate

用中文输出；命令与路径保持原样不翻译。

目标：把当前仓库补齐为 aiws workspace 模板，并启用可验证门禁。

执行（在项目根目录）：
1) 若已存在 `.aiws/manifest.json`：`npx @aipper/aiws update .`
2) 否则：`npx @aipper/aiws init .`
3) 门禁校验：`npx @aipper/aiws validate .`
4) 启用本机 git hooks（推荐，本地生效）：`git config core.hooksPath .githooks`

回滚：
- 恢复最近一次快照：`npx @aipper/aiws rollback . latest`

约束：
- 不写入任何 secrets。
- 不执行破坏性命令。
<!-- AIWS_MANAGED_END:claude:ws-migrate -->

可在下方追加本项目对 Claude Code 的额外说明（托管块外内容会被保留）。
