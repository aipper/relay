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
docker compose run --rm --entrypoint /app/relay-server relay-server -- --hash-password 'your-password'
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
