<!-- AIWS_MANAGED_BEGIN:claude:aiws-change-templates-init -->
# aiws change templates init

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 初始化 changes/templates 覆盖模板

执行（在仓库根目录）：
```bash
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws change templates init
elif command -v aiws >/dev/null 2>&1; then
  aiws change templates init
else
  npx @aipper/aiws change templates init
fi
```
<!-- AIWS_MANAGED_END:claude:aiws-change-templates-init -->

可在下方追加本项目对 Claude Code 的额外说明（托管块外内容会被保留）。
