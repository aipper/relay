# Design: pwa-e2e-green

> Title: pwa-e2e-green
>
> Created: 2026-09-17T09:20:41Z

## Context

- 背景：paseo-absorb-v1 收尾时全量 mock 套件 69/99，30 个失败经探针证实为测试侧漂移（基线 8fea4e4 的 LoginPage/LaunchPage 改版后 selector 过期），与产品逻辑无关。
- 现状：web/tests/e2e 共 99 用例（desktop/tablet/mobile × 33），fixtures/app.ts mock fetch+WS。
- 约束：只改测试文件，不碰 web/src；Svelte 5 runes 产物结构以当前构建为准。

## Goals / Non-Goals

**Goals:**
- 修复 10 组漂移断言（auth×2、搜索、审批卡、fork×2、new×1、switch×3），全量 99/99 全绿。

**Non-Goals:**
- 不改产品代码；不修 mock 覆盖不到的真机问题；不重构测试框架。

## Decisions

- 测试修测试：过期文本/role 改为当前 UI 的 locator（placeholder 精确定位）；fixtures/app.ts mock 补 WebSocket.OPEN/CONNECTING/CLOSING/CLOSED 静态常量（产品代码用 WebSocket.OPEN 发消息，缺失导致订阅/审批流全挂）；session-switch 用 exact:true 避 sidebar-toggle 多匹配。因为根因全在测试侧，产品零改动是最安全路径。

## Risks / Trade-offs

- mock 与产品 UI 持续漂移 → 缓解：产品 UI 改动时同步更新 e2e selector；本 change 已验证模式（placeholder 定位比文本断言耐改）。
- 全 mock 套件不覆盖真后端 → 缓解：8787/8790 隔离栈 E2E 仍按需手工跑，不在此 change 内。

## Migration / Rollback

- 无数据/接口迁移。回滚：git revert finish 合并提交即可，测试文件独立于产品代码。
