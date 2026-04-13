# Tasks: terminal-injection

> Title: 实现终端注入功能 (Terminal Injection)
> Created: 2026-03-14

## 0. Preflight

- [ ] 0.1 阅读并遵守 `AI_PROJECT.md` / `AI_WORKSPACE.md` / `REQUIREMENTS.md`
- [ ] 0.2 运行门禁校验：`aiws validate .`（或 `npx -y @aipper/aiws validate .`）

## 1. 需求/问题合同（如适用）

- [ ] 1.1 新功能开发（无需需求合同）

## 2. 实现

- [ ] 2.1 hostd: 扩展 `RunManager.inject_input` 方法
- [ ] 2.2 hostd: 新增 `/runs/:run_id/inject` API (local_api.rs)
- [ ] 2.3 server: 新增 `rpc.terminal.inject` RPC 方法
- [ ] 2.4 web: 前端 API 封装
- [ ] 2.5 web: RunPanel 注入 UI

## 3. 验证（必须可复现）

- [ ] 3.1 `cargo fmt --check` 通过
- [ ] 3.2 `cargo clippy --all-targets --all-features` 无警告
- [ ] 3.3 `cargo test` 全部通过
- [ ] 3.4 手动测试：启动 run，注入命令，验证输出

## 4. 交付与归档

- [ ] 4.1 证据落盘（代码变更、测试结果）
- [ ] 4.2 归档：`aiws change archive terminal-injection`
