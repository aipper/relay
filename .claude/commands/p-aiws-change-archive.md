<!-- AIWS_MANAGED_BEGIN:claude:aiws-change-archive -->
# aiws change archive

用中文输出（命令/路径/代码标识符保持原样不翻译）。

（私有原子入口；日常优先用 /ws-* 链路。）

目标：
- 归档已完成 change 工件，并生成交接文档：`changes/archive/.../handoff.md`

执行（在仓库根目录）：
```bash
change_id="<change-id>"
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws change archive "${change_id}"
elif command -v aiws >/dev/null 2>&1; then
  aiws change archive "${change_id}"
else
  npx @aipper/aiws change archive "${change_id}"
fi
```
<!-- AIWS_MANAGED_END:claude:aiws-change-archive -->

可在下方追加本项目对 Claude Code 的额外说明（托管块外内容会被保留）。
