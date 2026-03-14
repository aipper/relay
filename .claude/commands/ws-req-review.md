<!-- AIWS_MANAGED_BEGIN:claude:ws-req-review -->
# ws req review

用中文输出（命令/路径/代码标识符保持原样不翻译）。

目标：在不修改任何文件的前提下，对 `REQUIREMENTS.md` 做一次整体 QA，输出缺口/冲突/风险，减少实现漂移。

执行步骤（强制）：
1) 定位项目根目录：`git rev-parse --show-toplevel`（失败则停止并让用户确认根目录）。
2) 读取（存在则必须读取；缺失则明确列出，不要臆测）：
   - `AI_PROJECT.md`
   - `REQUIREMENTS.md`
   - `AI_WORKSPACE.md`
   - `requirements/CHANGELOG.md`（若存在）
   - `requirements/requirements-issues.csv`（若存在）
3) 输出固定结构的报告：

## 需求 QA（Requirements QA）
- 结论：是否可进入实现（是/否/有条件）
- 漂移风险：最容易导致不一致的点
- 可验收性缺口：缺少输入/输出/错误码/边界/示例的条目
- 完整性：非目标（Non-goals）/兼容性/鉴权/重试/并发/观测性/性能
- 一致性：与 `AI_PROJECT.md` 约束冲突点
- 可测试性与证据：最小验证命令 + 期望结果 + 证据路径
- 风险清单：3–8 条
- 需要澄清的问题：5–12 个（按优先级）

4) 最后询问用户：是否进入需求落盘流程 `/ws-req-change`？(Y/N)

安全：不打印 `secrets/test-accounts.json`。
<!-- AIWS_MANAGED_END:claude:ws-req-review -->

可在下方追加本项目对 Claude Code 的额外说明（托管块外内容会被保留）。
