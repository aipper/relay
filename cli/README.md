# relay-cli

`relay` 是一个基于 **Bun** 的 CLI（入口为 `#!/usr/bin/env bun`），用于控制 `relay-hostd` 与 `relay-server`。

## 安装

前置：确保已安装 `bun` 且在 `$PATH` 中（该 CLI 运行时依赖 Bun API）。

全局安装：

```bash
npm i -g @aipper/relay-cli
```

安装说明（macOS/Linux）：
- `npm i -g` 会在 `postinstall` 阶段 **best-effort** 下载并安装 `relay-hostd` + `relay`（MCP bridge）到 `~/.relay/bin/`
- 如果你在安装时禁用了 scripts，或网络/权限导致失败，可手动执行：`relay hostd install`

首次使用初始化（推荐）：

```bash
relay init --server http://<your-vps>:8787
```

（可选）一条命令同时常驻启动 hostd：

```bash
relay init --server http://<your-vps>:8787 --start-daemon
```

（可选，Linux）安装并启用 systemd user service（开机/登录后自动常驻）：

```bash
relay daemon stop || true
relay init --server http://<your-vps>:8787 --install-systemd-user
```

这会写入：
- `~/.relay/settings.json`（保存 server 地址，供 `relay auth login --save` 等命令复用）
- `~/.config/abrelay/hostd.json`（hostd 连接配置；默认生成 `host_id/host_token`，只需要配置 server 地址即可）

手动安装 `relay-hostd`（当 postinstall 失败或被跳过时）：

```bash
relay hostd install
```

说明：
- 该命令会下载并安装 native 二进制到 `~/.relay/bin/`（`relay-hostd` + `relay`（用于 MCP bridge））
- 默认下载地址会从 `cli/package.json#repository.url` 推导（可用 `RELAY_RELEASE_BASE_URL` 覆盖）；如需使用自建发布源，也可传 `--base-url`

## 使用

```bash
relay doctor
relay auth status
relay daemon start
```

说明：
- `relay codex/claude/iflow/gemini`、`relay runs/fs/git/local ...` 会在本地 unix socket 不存在时 **自动拉起** `relay-hostd`（等价于 `relay daemon start`）
- 如需非交互安装/确认，可用 `--yes/-y` 或设置 `RELAY_YES=1`

更多命令：

```bash
relay --help
```
