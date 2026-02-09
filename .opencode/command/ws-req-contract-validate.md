<!-- AIWS_MANAGED_BEGIN:opencode:ws-req-contract-validate -->
# ws req contract validate

用中文输出（命令/路径/代码标识符保持原样不翻译）。

执行（失败则修正 CSV 后重试）：
- `python3 tools/requirements_contract.py validate`

输出要求：
- 若失败：列出前 20 条缺失字段（Req_ID + field），并给出最小补齐建议
<!-- AIWS_MANAGED_END:opencode:ws-req-contract-validate -->

可在下方追加本项目对 OpenCode 的额外说明（托管块外内容会被保留）。
