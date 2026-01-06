# Deploy (VPS + Clients)

This system has two roles:

- **VPS**: runs `relay-server` (HTTP API + WebSocket + serves the PWA).
- **Client machine(s)**: run `relay-hostd` (runs Codex/Claude/iFlow locally and connects outbound to the VPS).

## VPS (Docker)

### 1) Configure env

Quick init (generates JWT_SECRET + ADMIN_PASSWORD_HASH):

```sh
bash scripts/docker-init.sh
```

If you use Caddy (or another reverse proxy) in the same Docker network, you can also configure an external network and a stable container name:

```sh
bash scripts/docker-init.sh --network caddy --container-name relay-server --no-ports
```

```sh
cp docker/server.env.example docker/server.env
```

Edit `docker/server.env`:
- `JWT_SECRET`: set to a long random string (recommended: generate)
- `ADMIN_USERNAME`: e.g. `admin`
- set **one** of:
  - `ADMIN_PASSWORD` (entrypoint will generate `ADMIN_PASSWORD_HASH` on boot)
  - `ADMIN_PASSWORD_HASH` (recommended for production)

Generate a random JWT secret:

```sh
bash scripts/gen-jwt-secret.sh
```

Optional: generate the password hash without installing Rust:

```sh
docker compose run --rm --entrypoint /app/relay-server relay-server --hash-password 'your-password'
```

### 2) Start

```sh
docker compose up -d --build
```

### 3) Verify

```sh
curl -s http://127.0.0.1:8787/health
```

Open the PWA:
- `http://<your-vps>:8787/`
- If you serve it via a reverse proxy with HTTPS, prefer `https://<your-domain>/`.

PWA login notes:
- By default, the PWA connects to the same origin (the current page), so you don't need to enter a Server URL.
- Only use a custom Server URL when the PWA and `relay-server` are on different origins (e.g. local dev).
- If you access the PWA over plain HTTP (non-localhost), the password is sent without transport encryption; prefer HTTPS.

## Client machine (hostd + relay CLI)

### Option A: use a packaged client bundle

On a machine with Rust toolchain (build box):

```sh
bash scripts/package-client.sh
```

Copy the generated directory `dist/relay-client-*/` to the client machine.

Start hostd (foreground):

```sh
./hostd-up.sh --server http://<your-vps>:8787 --host-token <token>
```

Start a run (example):

```sh
./bin/relay codex --cwd /path/to/project
```

### Option B: run hostd as a user service (Linux systemd)

If the client machine has `systemctl --user`, use the helper in the packaged bundle:

```sh
./install-hostd-systemd-user.sh --server http://<your-vps>:8787 --host-token <token>
```

Then check:

```sh
systemctl --user status relay-hostd
```

### Option C: one-shot client installer (Linux, recommended)

For Arch Linux / systemd-based distros, use the interactive installer:

```sh
./client-init.sh --server http://<your-vps>:8787
```

It validates `/health`, prompts for the host token, and installs either a user service (default) or a system service (`--mode system`).
