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

```sh
cp docker/server.env.example docker/server.env
```

编辑 `docker/server.env`：
- `JWT_SECRET`：设置为长随机字符串（推荐用脚本生成）
- `ADMIN_USERNAME`：例如 `admin`
- 二选一：
  - `ADMIN_PASSWORD`（启动时自动生成 `ADMIN_PASSWORD_HASH`）
  - `ADMIN_PASSWORD_HASH`（生产推荐）

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

## 客户端机器（hostd + relay CLI）

### 方式 A：使用打包的客户端目录

在有 Rust 工具链的构建机上：

```sh
bash scripts/package-client.sh
```

将生成的 `dist/relay-client-*/` 目录拷贝到客户端机器。

前台启动 hostd（连接 VPS）：

```sh
./hostd-up.sh --server http://<你的VPS>:8787 --host-token <token>
```

启动一次会话（示例）：

```sh
./bin/relay codex --cwd /path/to/project
```

### 方式 B：Linux 常驻（systemd user）

如果客户端机器支持 `systemctl --user`，使用打包目录里的脚本安装：

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
