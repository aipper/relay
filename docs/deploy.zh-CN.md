# 部署（VPS + 客户端）

本系统包含两个角色：

- **VPS**：运行 `relay-server`（HTTP API + WebSocket + 提供 PWA 静态资源）
- **客户端机器**：运行 `relay-hostd`（在本机启动 Codex/Claude/iFlow，并以出站 WebSocket 连接到 VPS）

## VPS（Docker，包含 PWA）

### 1) 配置 env

快速初始化（自动生成 `JWT_SECRET` + `ADMIN_PASSWORD_HASH`）：

```sh
bash scripts/docker-init.sh
```

如果你使用 Caddy（或其他反代）并希望和它在同一个 Docker 网络，可以在初始化时指定外部网络和固定的容器名：

```sh
bash scripts/docker-init.sh --network caddy --container-name relay-server --no-ports
```

```sh
cp docker/server.env.example docker/server.env
```

编辑 `docker/server.env`：
- `JWT_SECRET`：设置为长随机字符串（推荐用脚本生成）
- `ADMIN_USERNAME`：例如 `admin`
- 二选一：
  - `ADMIN_PASSWORD`（启动时自动生成 `ADMIN_PASSWORD_HASH`）
  - `ADMIN_PASSWORD_HASH`（生产推荐）
    - 注意：如果你手动填写 hash，请用单引号包起来（避免 `$...` 被某些 dotenv/compose 解析器误处理）：
      - `ADMIN_PASSWORD_HASH='$argon2id$v=19$...'`

随机生成 `JWT_SECRET`：

```sh
bash scripts/gen-jwt-secret.sh
```

可选：不装 Rust 也能生成 hash：

```sh
docker compose run --rm --entrypoint /app/relay-server relay-server --hash-password '你的密码'
```

### 2) 启动

```sh
docker compose up -d --build
```

### 3) 验证

```sh
curl -s http://127.0.0.1:8787/health
```

打开 PWA：
- `http://<你的VPS>:8787/`
- 如果你通过 Caddy/反代提供 HTTPS，请使用 `https://<你的域名>/`（推荐）。

PWA 登录说明：
- 默认连接“当前页面所在的服务”（同源），不需要填写 Server URL。
- 只有在 PWA 与 `relay-server` 不同源（例如本地开发）时，才需要启用“自定义 Server URL”并填写地址。
- 通过 `http://`（非本机）登录时密码为明文传输；建议在 HTTPS 下使用。

## 客户端机器（hostd + relay CLI）

### 方式 A：使用打包的客户端目录

在有 Rust 工具链的构建机上：

```sh
bash scripts/package-client.sh
```

将生成的 `dist/relay-client-*/` 目录拷贝到客户端机器。

前台启动 hostd（连接 VPS）：

```sh
./hostd-up.sh --server http://<你的VPS>:8787
```

启动一次会话（示例）：

```sh
./bin/relay codex --cwd /path/to/project
```

### 方式 B：Linux 常驻（systemd user）

如果客户端机器支持 `systemctl --user`，使用打包目录里的脚本安装：

```sh
./install-hostd-systemd-user.sh --server http://<你的VPS>:8787
systemctl --user status relay-hostd
```

### 方式 C：一键安装（Linux，推荐）

面向 Arch Linux / 其他基于 systemd 的发行版，使用交互式一键脚本：

```sh
./client-init.sh --server http://<你的VPS>:8787
```

该脚本会校验 `/health`，并默认安装为 user service；也可用 `--mode system` 安装为系统级服务。

Host 认证说明：
- 对同一个 `host_id`，server 第一次接入会记录 `sha256(host_token)`（TOFU）。
- host token 默认自动生成并存储在 hostd 配置文件里（打包目录默认：`~/.relay/hostd.json`）。

### 方式 D：npm 安装（macOS/Linux，需要 Bun）

如果你更希望“直接装一个 CLI”，可以使用 npm 包（而不是拷贝打包目录）。

安装：

```sh
npm i -g @aipper/relay-cli
```

说明：
- macOS/Linux 上 `postinstall` 会 best-effort 下载并安装 `relay-hostd` + `relay` 到 `~/.relay/bin/`。
- 若安装时禁用了 scripts 或网络/权限导致失败，可手动执行：`relay hostd install`。
- 默认下载地址会从 `cli/package.json#repository.url` 推导（可用 `RELAY_RELEASE_BASE_URL` 覆盖）；对应的文件命名规范见 `docs/release.md`。

一次性配置 server：

```sh
relay init --server http://<你的VPS>:8787
```

启动会话（如本地 unix socket 不存在会自动拉起 hostd）：

```sh
relay codex --cwd /path/to/project
```
