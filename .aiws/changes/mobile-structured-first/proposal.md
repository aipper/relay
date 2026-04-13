# Change Proposal: mobile-structured-first

> Title: 移动端默认会话卡片流优先（structured-first）
>
> Created: 2026-02-08

## 目标与非目标

**目标：**
- 在移动端（<=640）打开/恢复/切换会话时，默认进入事件卡片流，而不是终端视图。
- 终端保留为次级入口，仅在用户主动点击后进入。

**非目标：**
- 不改后端协议与鉴权。
- 不新增 API。

## 变更归因（强制二选一）

- 需求交付：`Req_ID` = `WEB-061`
- 本次不涉及问题修复（无 `Problem_ID`）

## 现状与问题

- 当前移动端仍存在进入终端输出上下文的路径，影响审批/待输入等结构化操作效率。

## 方案概述（What changes）

- 调整 `web/src/App.svelte` 的移动端默认视图选择逻辑：统一 messages/card stream 优先。
- 在会话切换/恢复路径上重置为事件视图，避免继承终端视图。

## 影响范围（Scope）

- 影响的服务/模块/目录：
  - `web/src/App.svelte`
- 可能影响的外部接口/使用方：
  - Web 移动端交互行为（无 API 变化）

## 风险与回滚

- 风险：
  - 桌面端视图偏好被误影响。
- 回滚方案（必须可执行）：
  - 回退 `web/src/App.svelte` 本次变更片段并重新构建验证。

## 验证计划（必须可复现）

- 命令：
  - `cd web && bun run build`
  - `aiws validate .`
- 期望结果：
  - 构建成功；AIWS 校验通过。

## 参考真值文件

- `AI_WORKSPACE.md`

## 真值文件/合同更新清单

- `REQUIREMENTS.md`：已更新（本次需求条款已落盘）
- `requirements/CHANGELOG.md`：已更新
- `requirements/requirements-issues.csv`：已更新（`WEB-061`）
- `issues/problem-issues.csv`：不需要
- 证据落盘（`.agentdocs/tmp/...`）：验证命令输出与变更工件
