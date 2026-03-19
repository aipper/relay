<!-- AIWS_MANAGED_BEGIN:claude:aiws-change-start -->
# aiws change start

（私有原子入口；日常优先用 /ws-* 链路。）

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：
- 切到 change/<change-id> 并初始化变更工件

执行（在仓库根目录）：
```bash
change_id="<change-id>"
if [[ -x "./node_modules/.bin/aiws" ]]; then
  ./node_modules/.bin/aiws change start "${change_id}"
elif command -v aiws >/dev/null 2>&1; then
  aiws change start "${change_id}"
else
  npx @aipper/aiws change start "${change_id}"
fi
```
<!-- AIWS_MANAGED_END:claude:aiws-change-start -->

可在下方追加本项目对 Claude Code 的额外说明（托管块外内容会被保留）。
