# Change Proposal: {{CHANGE_ID}}

> Title: {{TITLE}}
>
> Created: {{CREATED_AT}}

## 目标与非目标

**目标：**
- <!-- WS:TODO 填写本次变更的目标（可验收） -->

**非目标：**
- <!-- WS:TODO 填写明确“不做什么”，防止 scope creep -->

## 变更归因（强制二选一）

- 需求交付：`Req_ID` = <!-- WS:TODO -->
- 问题修复：`Problem_ID` = <!-- WS:TODO -->

> 备注：若“问题阻塞需求”，两边都要在各自 CSV 的 `Notes` 字段互相引用对方 ID。

## 现状与问题

- <!-- WS:TODO 现状是什么、痛点是什么；必要时给证据（日志/截图/issue 链接） -->

## 方案概述（What changes）

- <!-- WS:TODO 逐条写清要改什么（行为/接口/数据/配置）；BREAKING 请标注 -->

## 影响范围（Scope）

- 影响的服务/模块/目录：
  - <!-- WS:TODO -->
- 可能影响的外部接口/使用方：
  - <!-- WS:TODO -->

## 风险与回滚

- 风险：
  - <!-- WS:TODO -->
- 回滚方案（必须可执行）：
  - <!-- WS:TODO -->

## 验证计划（必须可复现）

> 从 `AI_WORKSPACE.md` 选择最贴近本变更的验证入口，写成可直接复制执行的命令。

- 命令：
  - <!-- WS:TODO 例如：`uv run tools/server_test_runner.py --workspace .` -->
- 期望结果：
  - <!-- WS:TODO 例如：所有相关用例 DONE；无新增错误日志 -->

## 真值文件/合同更新清单

- `REQUIREMENTS.md`：<!-- WS:TODO 需要/不需要；如需要，写明新增/修改的验收条款 -->
- `requirements/CHANGELOG.md`：<!-- WS:TODO 需要/不需要 -->
- `requirements/requirements-issues.csv`：<!-- WS:TODO 需要/不需要 -->
- `issues/problem-issues.csv`：<!-- WS:TODO 需要/不需要 -->
- 证据落盘（`.agentdocs/tmp/...`）：<!-- WS:TODO 计划输出哪些报告/日志 -->
