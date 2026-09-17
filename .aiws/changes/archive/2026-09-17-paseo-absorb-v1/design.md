# Design: paseo-absorb-v1

> Title: paseo-absorb-v1
>
> Created: 2026-09-17T03:14:30Z

## Context

- 背景：从 paseo（终端 agent 前端，node-pty + headless emulator）吸收 4 个机制：本地命令守卫、终端合并器、审批 actions、seq 回填。
- 现状：relay 是 Rust workspace（server/hostd）+ Bun CLI + Svelte PWA，链路 PWA↔server↔hostd；envelope 已有 per-run seq 与 spool replay。
- 约束：全部 additive（Optional 字段、缺省=旧行为），不引入二进制帧（relay 规模下瓶颈是包数不是编码）；Rust 侧 fmt/clippy/test 全绿，web 侧 build 全绿。

## Goals / Non-Goals

**Goals:**

- Phase A：ChatInput 本地精确匹配 `/clear` / `/exit`，不发远端。
- Phase B：server 5ms leading-edge 合并器，只作用于 app push 路径，spool 原始不变。
- Phase C：`permission_requested.actions[]/suggestions[]` + 决策 `selected_action_id`，多按钮渲染，旧形状回退。
- Phase D：messages API `after_seq` + 前端 seq 高水位断线回填，seq 去重、禁止内容匹配。

**Non-Goals:**

- 不引入 paseo 二进制 WS 帧（opcode+slot）与 snapshot/restore 机制。
- 不改 REQUIREMENTS.md（4 个 Phase 均为 additive，docs/protocol.md 已同步）。
- 不修标准 mock 套件 30 个预置失败（基线 8fea4e4 sidebar-toggle，与本 change 无关）。

## Decisions

- 跳过二进制帧：瓶颈在包数而非编码，合并器已解决主要成本；JSON envelope 保留可读性与可审计性。
- `selected_action_id` 全链路 Optional：protocol/hostd/server/web 逐层透传未知字段，未知 behavior fail-closed。
- 回填以 seq 为唯一真值：`maxSeqByRun` + 单 fetch 守卫 + seq 去重；NULL-seq 事件不参与回填（started 等元事件）。
- 证据策略：隔离栈（dev-up.sh --port 8790 + chromium-1208）做真实 E2E，标准 mock 套件只做回归参考。

## Risks / Trade-offs

- 合并器乱序观感 → 缓解：仅合并 5ms 窗口内同 run 输出，spool 保持逐行原始，replay 不受影响。
- hydrate 复活已决策审批 → 缓解：`#hydrateAwaitingFromMessages` 只回填无 decided 的 awaiting（review 记录残留 warning，行为已验证）。
- 回填 200-cap/NULL-seq 缺口 → 缓解：文档化限制，大 burst 场景由 spool replay 兜底。
- custom approve 语义静默 → 缓解：未知 behavior 直接 error 且 fail-closed，不执行。

## Migration / Rollback

- 无数据/接口迁移：所有字段 additive，旧客户端忽略新字段即可工作。
- 回滚：`git revert` 合入的 4 个 commit（2ef2d06 / 932ecd6 / 00b80c8 / bf4a57b）或整支回退；hostd/server 新旧版本可混跑（未知字段忽略）。
