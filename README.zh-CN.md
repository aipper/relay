# relay（Rust + PWA）

在“运行真实 CLI 的客户端机器”上运行 Codex / Claude / iFlow 等工具，并通过 VPS 上的中心服务 + PWA 在手机/浏览器里远程查看输出、发送输入、查看工具审计。

相关文档：
- 部署（VPS + 客户端）：`docs/deploy.zh-CN.md`

## 组件

- `server/`：中心服务（认证、路由、SQLite 存储、WebSocket、静态资源/PWA）
- `hostd/`：客户端守护进程（PTY runner，连接到 server，上报事件/接收指令）
- `relay-cli/`：本地 CLI（二进制 `relay`，通过 unix socket 访问 hostd）
- `cli/`：Bun CLI（开发/调试用，生产不要求安装 bun）
- `web/`：Svelte PWA

## VPS 部署（Docker，包含 PWA）

快速初始化（自动生成 `JWT_SECRET` + `ADMIN_PASSWORD_HASH`）：

```sh
bash scripts/docker-init.sh
```

1) 准备配置：

```sh
cp docker/server.env.example docker/server.env
```

编辑 `docker/server.env`：
- `JWT_SECRET`：设置为足够长的随机字符串（推荐用脚本生成）
- `ADMIN_USERNAME`：比如 `admin`
- 二选一：
  - `ADMIN_PASSWORD`（容器启动时会自动生成 `ADMIN_PASSWORD_HASH`）
  - `ADMIN_PASSWORD_HASH`（生产推荐直接用 hash）

随机生成 `JWT_SECRET`：

```sh
bash scripts/gen-jwt-secret.sh
```

可选：不装 Rust 也能生成 hash：

```sh
docker compose run --rm --entrypoint /app/relay-server relay-server --hash-password '你的密码'
```

2) 启动：

```sh
docker compose up -d --build
```

3) 验证：

```sh
curl -s http://127.0.0.1:8787/health
```

4) 打开 PWA：
- `http://<你的VPS>:8787/`

## 客户端（hostd + relay CLI，不依赖 bun）

`hostd` 必须跑在“真正运行 Codex/Claude/iFlow 的那台机器”上（需要访问本机项目目录与本机安装的 CLI 二进制）。

### 方式 A：打包客户端目录（推荐）

在有 Rust 工具链的构建机上：

```sh
bash scripts/package-client.sh
```

把生成的 `dist/relay-client-*/` 整个目录拷贝到客户端机器。

前台启动 hostd（连接到 VPS）：

```sh
./hostd-up.sh --server http://<你的VPS>:8787 --host-token <token>
```

启动一次会话（示例）：

```sh
./bin/relay codex --cwd /path/to/project
```

### 方式 B：Linux 常驻（systemd user）

如果客户端机器支持 `systemctl --user`：

```sh
./install-hostd-systemd-user.sh --server http://<你的VPS>:8787 --host-token <token>
systemctl --user status relay-hostd
```

### 方式 C：一键安装（Linux，推荐）

面向 Arch Linux / 其他基于 systemd 的发行版，使用交互式一键脚本：

```sh
./client-init.sh --server http://<你的VPS>:8787
```

该脚本会校验 `/health`，提示输入 host token，并默认安装为 user service；也可用 `--mode system` 安装为系统级服务。

## 可选：安装 codex/claude/iflow shims（在任意项目目录直接敲命令）

如果你希望在任意项目目录里直接运行 `codex`/`claude`/`iflow` 时自动被 relay 接管：

```sh
bash scripts/install-shims.sh --auto-path
```

卸载：

```sh
bash scripts/install-shims.sh --uninstall
```
