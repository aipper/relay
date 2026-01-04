#!/usr/bin/env bun

type JsonValue = unknown;

function usage(): never {
  console.log(`relay (skeleton)

Usage:
  relay auth status
  relay auth login --server http://127.0.0.1:8787 --username admin --password '...' [--save]
  relay auth logout

  relay codex  --sock /tmp/relay-hostd.sock [--cmd "codex ..."] [--cwd .]    (default: codex)
  relay claude --sock /tmp/relay-hostd.sock [--cmd "claude ..."] [--cwd .]   (default: claude)
  relay iflow  --sock /tmp/relay-hostd.sock [--cmd "iflow ..."] [--cwd .]    (default: iflow)
  relay gemini --sock /tmp/relay-hostd.sock [--cmd "gemini ..."] [--cwd .]   (default: gemini)

  relay daemon start [--server http://127.0.0.1:8787] [--host-id host-dev] [--host-token devtoken]
                    [--sock /tmp/relay-hostd.sock] [--spool ~/.relay/hostd-spool.db] [--log ~/.relay/logs/hostd.log]
  relay daemon status
  relay daemon stop
  relay daemon logs

  relay doctor

  relay login --server http://127.0.0.1:8787 --username admin --password '...'   (compat)
  relay local start --sock /tmp/relay-hostd.sock --tool codex --cmd "..." [--cwd .]
  relay local input --sock /tmp/relay-hostd.sock --run <run_id> --text "y\\n" [--input-id <uuid>]
  relay fs read   --sock /tmp/relay-hostd.sock --run <run_id> --path relative/file.txt
  relay fs search --sock /tmp/relay-hostd.sock --run <run_id> --q "needle"
  relay git status --sock /tmp/relay-hostd.sock --run <run_id>
  relay git diff   --sock /tmp/relay-hostd.sock --run <run_id> [--path relative/file.txt]
  relay runs list  --sock /tmp/relay-hostd.sock
  relay runs stop  --sock /tmp/relay-hostd.sock --run <run_id> [--signal term|kill]
  relay ws-send-input --server http://127.0.0.1:8787 --token <jwt> --run <run_id> --text "y\\n" [--input-id <uuid>]
  relay ws-stop --server http://127.0.0.1:8787 --token <jwt> --run <run_id> [--signal term|kill]
  relay ws-approve --server http://127.0.0.1:8787 --token <jwt> --run <run_id> --request-id <uuid>
  relay ws-deny    --server http://127.0.0.1:8787 --token <jwt> --run <run_id> --request-id <uuid>
  relay ws-start-run --server http://127.0.0.1:8787 --token <jwt> --host-id <host_id> --tool codex --cmd "echo hi; cat" [--cwd .]
  relay ws-start-run --server http://127.0.0.1:8787 --token <jwt> --host-id <host_id> --tool codex [--cmd "codex ..."] [--cwd .]
  relay ws-rpc-fs-read   --server http://127.0.0.1:8787 --token <jwt> --run <run_id> --path relative/file.txt
  relay ws-rpc-fs-search --server http://127.0.0.1:8787 --token <jwt> --run <run_id> --q "needle"
  relay ws-rpc-fs-list   --server http://127.0.0.1:8787 --token <jwt> --run <run_id> [--path .]
  relay ws-rpc-git-status --server http://127.0.0.1:8787 --token <jwt> --run <run_id>
  relay ws-rpc-git-diff   --server http://127.0.0.1:8787 --token <jwt> --run <run_id> [--path relative/file.txt]
  relay ws-rpc-run-stop   --server http://127.0.0.1:8787 --token <jwt> --run <run_id> [--signal term|kill]
  relay ws-rpc-runs-list  --server http://127.0.0.1:8787 --token <jwt> --run <run_id>
  relay ws-rpc-host-info   --server http://127.0.0.1:8787 --token <jwt> --host-id <host_id>
  relay ws-rpc-host-doctor --server http://127.0.0.1:8787 --token <jwt> --host-id <host_id>
  relay ws-rpc-host-capabilities --server http://127.0.0.1:8787 --token <jwt> --host-id <host_id>
  relay ws-rpc-host-logs-tail --server http://127.0.0.1:8787 --token <jwt> --host-id <host_id> [--lines 200] [--max-bytes 200000]

Notes:
  - This CLI is a thin control layer. The long-running tool processes should be owned by hostd.
  - local commands use curl --unix-socket (requires curl in PATH).
  - auth token source priority: RELAY_TOKEN env > ~/.relay/settings.json
  - daemon is a local helper for starting relay-hostd in background (dev-oriented).
`);
  process.exit(1);
}

function getArg(flag: string): string | undefined {
  const idx = process.argv.indexOf(flag);
  if (idx === -1) return undefined;
  return process.argv[idx + 1];
}

function hasFlag(flag: string): boolean {
  return process.argv.includes(flag);
}

function requireCmd(cmd: string | undefined): string {
  if (!cmd) usage();
  return cmd;
}

function cmdOrDefault(cmd: string | undefined, def: string): string {
  return cmd ?? def;
}

function envOrUndefined(name: string): string | undefined {
  const v = process.env[name];
  if (!v) return undefined;
  return v;
}

function settingsPath(): string {
  const home = envOrUndefined("HOME");
  if (!home) throw new Error("HOME is not set; cannot read ~/.relay/settings.json");
  return `${home.replace(/\/$/, "")}/.relay/settings.json`;
}

function relayHomeDir(): string {
  const home = envOrUndefined("HOME");
  if (!home) throw new Error("HOME is not set; cannot use ~/.relay");
  return `${home.replace(/\/$/, "")}/.relay`;
}

function daemonStatePath(): string {
  return `${relayHomeDir()}/daemon.state.json`;
}

type Settings = { server?: string; token?: string };

async function readSettings(): Promise<Settings> {
  const path = settingsPath();
  const file = Bun.file(path);
  if (!(await file.exists())) return {};
  const raw = await file.text();
  const parsed = JSON.parse(raw) as unknown;
  if (!parsed || typeof parsed !== "object") return {};
  const obj = parsed as Record<string, unknown>;
  const server = typeof obj.server === "string" ? obj.server : undefined;
  const token = typeof obj.token === "string" ? obj.token : undefined;
  return { server, token };
}

async function writeSettings(next: Settings): Promise<void> {
  const fs = await import("node:fs/promises");
  const path = settingsPath();
  const dir = path.replace(/\/settings\.json$/, "");
  await fs.mkdir(dir, { recursive: true });
  await Bun.write(path, JSON.stringify(next, null, 2) + "\n");
}

type DaemonState = {
  pid: number;
  started_at: string;
  server: string;
  server_ws: string;
  host_id: string;
  host_token: string;
  sock: string;
  spool: string;
  log: string;
  hostd_bin: string;
};

async function readDaemonState(): Promise<DaemonState | null> {
  const path = daemonStatePath();
  const file = Bun.file(path);
  if (!(await file.exists())) return null;
  const raw = await file.text();
  return JSON.parse(raw) as DaemonState;
}

async function writeDaemonState(state: DaemonState): Promise<void> {
  const fs = await import("node:fs/promises");
  await fs.mkdir(relayHomeDir(), { recursive: true });
  await Bun.write(daemonStatePath(), JSON.stringify(state, null, 2) + "\n");
}

async function clearDaemonState(): Promise<void> {
  const fs = await import("node:fs/promises");
  try {
    await fs.unlink(daemonStatePath());
  } catch {
    // ignore
  }
}

function stripTrailingSlash(url: string): string {
  return url.replace(/\/$/, "");
}

function toWsBaseFromHttp(httpBase: string): string {
  return stripTrailingSlash(httpBase).replace(/^http:/, "ws:").replace(/^https:/, "wss:");
}

function isProcessRunning(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

async function resolveServer(): Promise<{ server: string; source: "arg" | "env" | "file" }> {
  const arg = getArg("--server");
  if (arg) return { server: stripTrailingSlash(arg), source: "arg" };
  const env = envOrUndefined("RELAY_SERVER");
  if (env) return { server: stripTrailingSlash(env), source: "env" };
  const s = await readSettings();
  if (s.server) return { server: stripTrailingSlash(s.server), source: "file" };
  throw new Error("missing server; set --server or RELAY_SERVER or ~/.relay/settings.json");
}

async function resolveToken(): Promise<{ token: string; source: "env" | "file" }> {
  const env = envOrUndefined("RELAY_TOKEN");
  if (env) return { token: env, source: "env" };
  const s = await readSettings();
  if (s.token) return { token: s.token, source: "file" };
  throw new Error("missing token; set RELAY_TOKEN or run `relay auth login --save`");
}

function requireBinaryInPath(bin: string): void {
  const r = Bun.spawnSync(["bash", "-lc", `command -v ${JSON.stringify(bin)} >/dev/null 2>&1`], {
    stdout: "ignore",
    stderr: "ignore",
  });
  if (r.exitCode !== 0) throw new Error(`missing dependency: ${bin}`);
}

function resolveHostdBin(): string {
  const env = envOrUndefined("RELAY_HOSTD_BIN");
  if (env) return env;

  // Prefer installed binary in PATH (production-style).
  const which = Bun.spawnSync(["bash", "-lc", "command -v relay-hostd 2>/dev/null || true"], {
    stdout: "pipe",
    stderr: "ignore",
  });
  const found = (which.stdout ? new TextDecoder().decode(which.stdout).trim() : "").trim();
  if (found) return found;

  // Dev fallback: workspace binary path if present.
  const dev = `${process.cwd()}/target/debug/relay-hostd`;
  return dev;
}

async function postJson(url: string, body: Record<string, JsonValue>) {
  const res = await fetch(url, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });
  const text = await res.text();
  if (!res.ok) throw new Error(`${res.status} ${res.statusText}: ${text}`);
  return JSON.parse(text) as Record<string, JsonValue>;
}

async function localStartRun(sock: string, tool: string, runCmd: string, cwd?: string) {
  requireBinaryInPath("curl");
  const body = { tool, cmd: runCmd, cwd: cwd ?? null };
  const curlArgs = [
    "--silent",
    "--show-error",
    "--unix-socket",
    sock,
    "-X",
    "POST",
    "http://localhost/runs",
    "-H",
    "content-type: application/json",
    "--data-binary",
    JSON.stringify(body),
  ];
  const p = Bun.spawn(["curl", ...curlArgs], { stdout: "pipe", stderr: "pipe" });
  const out = await new Response(p.stdout).text();
  const err = await new Response(p.stderr).text();
  const code = await p.exited;
  if (code !== 0) throw new Error(err || out);
  const parsed = JSON.parse(out) as Record<string, JsonValue>;
  const runId = typeof parsed.run_id === "string" ? parsed.run_id : undefined;
  if (!runId) throw new Error(`unexpected response from hostd: ${out}`);
  return { out, runId };
}

async function localGetJson(sock: string, url: string): Promise<Record<string, JsonValue>> {
  requireBinaryInPath("curl");
  const curlArgs = ["--silent", "--show-error", "--unix-socket", sock, "-X", "GET", url];
  const p = Bun.spawn(["curl", ...curlArgs], { stdout: "pipe", stderr: "pipe" });
  const out = await new Response(p.stdout).text();
  const err = await new Response(p.stderr).text();
  const code = await p.exited;
  if (code !== 0) throw new Error(err || out);
  return JSON.parse(out) as Record<string, JsonValue>;
}

async function localPostJson(sock: string, url: string, body: Record<string, JsonValue>): Promise<Record<string, JsonValue> | null> {
  requireBinaryInPath("curl");
  const curlArgs = [
    "--silent",
    "--show-error",
    "--unix-socket",
    sock,
    "-X",
    "POST",
    url,
    "-H",
    "content-type: application/json",
    "--data-binary",
    JSON.stringify(body),
  ];
  const p = Bun.spawn(["curl", ...curlArgs], { stdout: "pipe", stderr: "pipe" });
  const out = await new Response(p.stdout).text();
  const err = await new Response(p.stderr).text();
  const code = await p.exited;
  if (code !== 0) throw new Error(err || out);
  if (!out.trim()) return null;
  return JSON.parse(out) as Record<string, JsonValue>;
}

async function wsSend(server: string, token: string, env: Record<string, JsonValue>) {
  const wsUrl = server.replace(/^http:/, "ws:").replace(/^https:/, "wss:").replace(/\/$/, "");
  await new Promise<void>((resolve, reject) => {
    let done = false;
    let timeout: ReturnType<typeof setTimeout> | undefined;
    const finish = (err?: Error) => {
      if (done) return;
      done = true;
      if (timeout) clearTimeout(timeout);
      if (err) reject(err);
      else resolve();
    };
    const ws = new WebSocket(`${wsUrl}/ws/app?token=${encodeURIComponent(token)}`);
    timeout = setTimeout(() => {
      try {
        ws.close();
      } catch {
        // ignore
      }
      finish(new Error("websocket timeout"));
    }, 10_000);
    ws.onopen = () => {
      ws.send(JSON.stringify(env));
      // Give the runtime a brief moment to flush the outgoing frame before closing.
      setTimeout(() => {
        try {
          ws.close();
        } catch {
          // ignore
        }
      }, 10);
    };
    ws.onerror = () => finish(new Error("websocket error"));
    ws.onclose = (ev) => {
      // If the handshake was rejected (e.g. 401), Bun surfaces it as a close with a non-1000 code.
      const code = typeof ev?.code === "number" ? ev.code : 1000;
      if (code !== 1000) {
        const reason = typeof ev?.reason === "string" ? ev.reason : "";
        finish(new Error(`websocket closed: ${code}${reason ? ` ${reason}` : ""}`));
        return;
      }
      finish();
    };
  });
}

async function wsRpc(
  server: string,
  token: string,
  requestEnv: Record<string, JsonValue>,
  matchResponse: (env: Record<string, JsonValue>) => boolean,
  timeoutMs: number,
): Promise<Record<string, JsonValue>> {
  const wsUrl = server.replace(/^http:/, "ws:").replace(/^https:/, "wss:").replace(/\/$/, "");
  return await new Promise<Record<string, JsonValue>>((resolve, reject) => {
    const ws = new WebSocket(`${wsUrl}/ws/app?token=${encodeURIComponent(token)}`);
    let done = false;
    const timeout = setTimeout(() => {
      try {
        ws.close();
      } catch {
        // ignore
      }
      done = true;
      reject(new Error("rpc timeout"));
    }, timeoutMs);

    ws.onopen = () => {
      ws.send(JSON.stringify(requestEnv));
    };
    ws.onerror = () => {
      clearTimeout(timeout);
      if (done) return;
      reject(new Error("websocket error"));
    };
    ws.onmessage = (ev) => {
      try {
        const msg = JSON.parse(ev.data) as Record<string, JsonValue>;
        if (matchResponse(msg)) {
          clearTimeout(timeout);
          done = true;
          try {
            ws.close();
          } catch {
            // ignore
          }
          resolve(msg);
        }
      } catch {
        // ignore
      }
    };
    ws.onclose = (ev) => {
      clearTimeout(timeout);
      if (done) return;
      const code = typeof ev?.code === "number" ? ev.code : 1000;
      if (code !== 1000) {
        const reason = typeof ev?.reason === "string" ? ev.reason : "";
        reject(new Error(`websocket closed: ${code}${reason ? ` ${reason}` : ""}`));
      } else {
        reject(new Error("websocket closed"));
      }
    };
  });
}

async function main() {
  const cmd = requireCmd(process.argv[2]);

  if (cmd === "runs") {
    const sub = requireCmd(process.argv[3]);
    const sock = getArg("--sock") ?? envOrUndefined("RELAY_HOSTD_SOCK");
    if (!sock) usage();

    if (sub === "list") {
      const data = await localGetJson(sock, "http://localhost/runs");
      console.log(JSON.stringify(data, null, 2));
      return;
    }

    if (sub === "stop") {
      const runId = getArg("--run");
      const signal = getArg("--signal") ?? "term";
      if (!runId) usage();
      if (signal !== "term" && signal !== "kill") throw new Error("invalid --signal (expected term|kill)");
      await localPostJson(sock, `http://localhost/runs/${encodeURIComponent(runId)}/stop`, { signal });
      console.log("ok");
      return;
    }

    usage();
  }

  if (cmd === "fs") {
    const sub = requireCmd(process.argv[3]);
    const sock = getArg("--sock") ?? envOrUndefined("RELAY_HOSTD_SOCK");
    const runId = getArg("--run");
    if (!sock || !runId) usage();

    if (sub === "read") {
      const path = getArg("--path");
      if (!path) usage();
      const url = `http://localhost/runs/${encodeURIComponent(runId)}/fs/read?path=${encodeURIComponent(path)}`;
      const data = await localGetJson(sock, url);
      console.log(JSON.stringify(data, null, 2));
      return;
    }

    if (sub === "search") {
      const q = getArg("--q");
      if (!q) usage();
      const url = `http://localhost/runs/${encodeURIComponent(runId)}/fs/search?q=${encodeURIComponent(q)}`;
      const data = await localGetJson(sock, url);
      console.log(JSON.stringify(data, null, 2));
      return;
    }

    usage();
  }

  if (cmd === "git") {
    const sub = requireCmd(process.argv[3]);
    const sock = getArg("--sock") ?? envOrUndefined("RELAY_HOSTD_SOCK");
    const runId = getArg("--run");
    if (!sock || !runId) usage();

    if (sub === "status") {
      const url = `http://localhost/runs/${encodeURIComponent(runId)}/git/status`;
      const data = await localGetJson(sock, url);
      console.log(JSON.stringify(data, null, 2));
      return;
    }

    if (sub === "diff") {
      const path = getArg("--path");
      const url =
        path !== undefined
          ? `http://localhost/runs/${encodeURIComponent(runId)}/git/diff?path=${encodeURIComponent(path)}`
          : `http://localhost/runs/${encodeURIComponent(runId)}/git/diff`;
      const data = await localGetJson(sock, url);
      console.log(JSON.stringify(data, null, 2));
      return;
    }

    usage();
  }

  if (cmd === "daemon") {
    const sub = requireCmd(process.argv[3]);

    if (sub === "status") {
      const state = await readDaemonState();
      if (!state) {
        console.log(JSON.stringify({ running: false, state: null }, null, 2));
        return;
      }
      console.log(JSON.stringify({ running: isProcessRunning(state.pid), state }, null, 2));
      return;
    }

    if (sub === "logs") {
      const state = await readDaemonState();
      if (!state) throw new Error("daemon not configured; run `relay daemon start` first");
      console.log(state.log);
      return;
    }

    if (sub === "stop") {
      const state = await readDaemonState();
      if (!state) {
        console.log(JSON.stringify({ stopped: false, reason: "no_state" }, null, 2));
        return;
      }
      if (!isProcessRunning(state.pid)) {
        await clearDaemonState();
        console.log(JSON.stringify({ stopped: true, already: true }, null, 2));
        return;
      }
      process.kill(state.pid, "SIGTERM");
      for (let i = 0; i < 50; i++) {
        if (!isProcessRunning(state.pid)) break;
        await new Promise((r) => setTimeout(r, 100));
      }
      const still = isProcessRunning(state.pid);
      if (still) process.kill(state.pid, "SIGKILL");
      await clearDaemonState();
      console.log(JSON.stringify({ stopped: true, killed: still }, null, 2));
      return;
    }

    if (sub === "start") {
      const serverHttp = getArg("--server") ?? (await resolveServer()).server;
      const serverWs = toWsBaseFromHttp(serverHttp);
      const hostId = getArg("--host-id") ?? envOrUndefined("RELAY_HOST_ID") ?? "host-dev";
      const hostToken = getArg("--host-token") ?? envOrUndefined("RELAY_HOST_TOKEN") ?? "devtoken";

      const sock =
        getArg("--sock") ?? envOrUndefined("RELAY_HOSTD_SOCK") ?? `${relayHomeDir()}/relay-hostd.sock`;
      const spool =
        getArg("--spool") ?? envOrUndefined("RELAY_SPOOL_DB") ?? `${relayHomeDir()}/hostd-spool.db`;
      const log = getArg("--log") ?? envOrUndefined("RELAY_HOSTD_LOG") ?? `${relayHomeDir()}/logs/hostd.log`;

      const hostdBin = resolveHostdBin();
      const fs = await import("node:fs/promises");
      await fs.mkdir(`${relayHomeDir()}/logs`, { recursive: true });

      const existing = await readDaemonState();
      if (existing && isProcessRunning(existing.pid)) {
        console.log(JSON.stringify({ started: false, reason: "already_running", pid: existing.pid }, null, 2));
        return;
      }

      // Best-effort: clear stale socket file.
      try {
        await fs.unlink(sock);
      } catch {
        // ignore
      }

      // Spawn detached hostd.
      const out = await fs.open(log, "a");
      const child = Bun.spawn([hostdBin], {
        env: {
          ...process.env,
          SERVER_BASE_URL: serverWs,
          HOST_ID: hostId,
          HOST_TOKEN: hostToken,
          LOCAL_UNIX_SOCKET: sock,
          SPOOL_DB_PATH: spool,
        },
        stdout: out,
        stderr: out,
        detached: true,
      });
      child.unref();

      const pid = child.pid;
      if (!pid) throw new Error("failed to spawn hostd (missing pid)");

      const state: DaemonState = {
        pid,
        started_at: new Date().toISOString(),
        server: serverHttp,
        server_ws: serverWs,
        host_id: hostId,
        host_token: hostToken,
        sock,
        spool,
        log,
        hostd_bin: hostdBin,
      };
      await writeDaemonState(state);

      console.log(JSON.stringify({ started: true, pid, sock, log }, null, 2));
      return;
    }

    usage();
  }

  if (cmd === "doctor") {
    const checks: Array<{ name: string; ok: boolean; detail?: string }> = [];

    const have = (bin: string) => {
      try {
        requireBinaryInPath(bin);
        return true;
      } catch {
        return false;
      }
    };

    checks.push({ name: "curl", ok: have("curl") });
    checks.push({ name: "sqlite3", ok: have("sqlite3") });

    const hostdBin = resolveHostdBin();
    const hostdExists =
      Bun.spawnSync(["bash", "-lc", `test -x ${JSON.stringify(hostdBin)} && echo ok || true`], {
        stdout: "pipe",
        stderr: "ignore",
      }).exitCode === 0;
    checks.push({ name: "relay-hostd", ok: hostdExists, detail: hostdBin });

    const state = await readDaemonState();
    if (state) {
      checks.push({ name: "daemon.running", ok: isProcessRunning(state.pid), detail: `pid=${state.pid}` });
      const sockOk =
        Bun.spawnSync(["bash", "-lc", `test -S ${JSON.stringify(state.sock)} && echo ok || true`], {
          stdout: "pipe",
          stderr: "ignore",
        }).exitCode === 0;
      checks.push({ name: "hostd.sock", ok: sockOk, detail: state.sock });

      // Best-effort: local API check.
      if (sockOk && have("curl")) {
        const r = Bun.spawnSync(
          ["curl", "--silent", "--show-error", "--unix-socket", state.sock, "http://localhost/runs"],
          { stdout: "pipe", stderr: "pipe" },
        );
        checks.push({ name: "hostd.api", ok: r.exitCode === 0, detail: r.exitCode === 0 ? "ok" : "curl failed" });
      }
    } else {
      checks.push({ name: "daemon.state", ok: false, detail: "no ~/.relay/daemon.state.json" });
    }

    const server = await (async () => {
      try {
        return await resolveServer();
      } catch {
        return null;
      }
    })();
    if (server) {
      try {
        const h = await fetch(`${server.server}/health`);
        checks.push({ name: "server.health", ok: h.ok, detail: `${h.status}` });
      } catch {
        checks.push({ name: "server.health", ok: false, detail: "fetch failed" });
      }
    } else {
      checks.push({ name: "server", ok: false, detail: "missing RELAY_SERVER or settings server" });
    }

    console.log(JSON.stringify({ ok: checks.every((c) => c.ok), checks }, null, 2));
    return;
  }

  if (cmd === "auth") {
    const sub = requireCmd(process.argv[3]);

    if (sub === "status") {
      const server = await (async () => {
        try {
          return await resolveServer();
        } catch {
          return null;
        }
      })();
      const token = await (async () => {
        try {
          return await resolveToken();
        } catch {
          return null;
        }
      })();
      console.log(
        JSON.stringify(
          {
            server: server?.server ?? null,
            server_source: server?.source ?? null,
            token_present: Boolean(token?.token),
            token_source: token?.source ?? null,
          },
          null,
          2,
        ),
      );
      return;
    }

    if (sub === "login") {
      const server = getArg("--server") ?? envOrUndefined("RELAY_SERVER");
      const username = getArg("--username");
      const password = getArg("--password");
      const save = hasFlag("--save");
      if (!server || !username || !password) usage();

      const data = await postJson(`${stripTrailingSlash(server)}/auth/login`, { username, password });
      const token = data.access_token;
      if (typeof token !== "string" || !token) throw new Error("unexpected login response");

      if (save) {
        const prev = await readSettings();
        await writeSettings({ ...prev, server: stripTrailingSlash(server), token });
        console.log(JSON.stringify({ saved: true }, null, 2));
      } else {
        console.log(JSON.stringify({ access_token: token }, null, 2));
      }
      return;
    }

    if (sub === "logout") {
      const prev = await readSettings();
      await writeSettings({ ...prev, token: undefined });
      console.log(JSON.stringify({ saved: true }, null, 2));
      return;
    }

    usage();
  }

  if (cmd === "codex" || cmd === "claude" || cmd === "iflow" || cmd === "gemini") {
    const sock = getArg("--sock") ?? envOrUndefined("RELAY_HOSTD_SOCK");
    const runCmd = cmdOrDefault(getArg("--cmd"), cmd);
    const cwd = getArg("--cwd");
    if (!sock) usage();
    const { out } = await localStartRun(sock, cmd, runCmd, cwd);
    console.log(out.trim());
    return;
  }

  if (cmd === "login") {
    const server = getArg("--server");
    const username = getArg("--username");
    const password = getArg("--password");
    if (!server || !username || !password) usage();

    const data = await postJson(`${server.replace(/\/$/, "")}/auth/login`, {
      username,
      password,
    });
    console.log(JSON.stringify(data, null, 2));
    return;
  }

  if (cmd === "local") {
    const sub = process.argv[3];
    if (!sub) usage();

    const sock = getArg("--sock");
    if (!sock) usage();

    if (sub === "start") {
      const tool = getArg("--tool");
      const runCmd = getArg("--cmd") ?? tool;
      const cwd = getArg("--cwd");
      if (!tool || !runCmd) usage();

      const { out } = await localStartRun(sock, tool, runCmd, cwd ?? undefined);
      console.log(out.trim());
      return;
    }

    if (sub === "input") {
      const runId = getArg("--run");
      const text = getArg("--text");
      const inputId = getArg("--input-id") ?? crypto.randomUUID();
      if (!runId || text === undefined) usage();

      const body = { input_id: inputId, text, actor: "cli" };
      const curlArgs = [
        "--silent",
        "--show-error",
        "--unix-socket",
        sock,
        "-X",
        "POST",
        `http://localhost/runs/${encodeURIComponent(runId)}/input`,
        "-H",
        "content-type: application/json",
        "--data-binary",
        JSON.stringify(body),
      ];
      const p = Bun.spawn(["curl", ...curlArgs], { stdout: "pipe", stderr: "pipe" });
      const out = await new Response(p.stdout).text();
      const err = await new Response(p.stderr).text();
      const code = await p.exited;
      if (code !== 0) throw new Error(err || out);
      if (out.trim()) console.log(out);
      return;
    }

    usage();
  }

  if (cmd === "ws-send-input") {
    const server = getArg("--server") ?? (await resolveServer()).server;
    const token = getArg("--token") ?? (await resolveToken()).token;
    const runId = getArg("--run");
    const text = getArg("--text");
    const inputId = getArg("--input-id") ?? crypto.randomUUID();
    if (!server || !token || !runId || text === undefined) usage();

    const url = `${stripTrailingSlash(server)}/runs/${encodeURIComponent(runId)}/input`;
    const res = await fetch(url, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${token}`,
        "content-type": "application/json",
      },
      body: JSON.stringify({ input_id: inputId, actor: "cli", text }),
    });
    const body = await res.text();
    if (!res.ok) throw new Error(`${res.status} ${res.statusText}: ${body}`);

    console.log("sent");
    return;
  }

  if (cmd === "ws-stop") {
    const server = getArg("--server") ?? (await resolveServer()).server;
    const token = getArg("--token") ?? (await resolveToken()).token;
    const runId = getArg("--run");
    const signal = getArg("--signal") ?? "term";
    if (!server || !token || !runId) usage();
    if (signal !== "term" && signal !== "kill") throw new Error("invalid --signal (expected term|kill)");

    await wsSend(server, token, {
      type: "run.stop",
      ts: new Date().toISOString(),
      run_id: runId,
      data: { signal, actor: "cli" },
    });

    console.log("sent");
    return;
  }

  if (cmd === "ws-approve" || cmd === "ws-deny") {
    const server = getArg("--server") ?? (await resolveServer()).server;
    const token = getArg("--token") ?? (await resolveToken()).token;
    const runId = getArg("--run");
    const requestId = getArg("--request-id");
    if (!server || !token || !runId || !requestId) usage();

    await wsSend(server, token, {
      type: cmd === "ws-approve" ? "run.permission.approve" : "run.permission.deny",
      ts: new Date().toISOString(),
      run_id: runId,
      data: { request_id: requestId, actor: "cli" },
    });

    console.log("sent");
    return;
  }

  if (cmd === "ws-start-run") {
    const server = getArg("--server") ?? (await resolveServer()).server;
    const token = getArg("--token") ?? (await resolveToken()).token;
    const hostId = getArg("--host-id");
    const tool = getArg("--tool") ?? "codex";
    const runCmd = getArg("--cmd") ?? tool;
    const cwd = getArg("--cwd");
    if (!server || !token || !hostId || !runCmd) usage();

    const requestId = crypto.randomUUID();
    const env: Record<string, JsonValue> = {
      type: "rpc.run.start",
      ts: new Date().toISOString(),
      data: { request_id: requestId, host_id: hostId, tool, cmd: runCmd, cwd: cwd ?? null },
    };

    const resp = await wsRpc(
      server,
      token,
      env,
      (m) => {
        if (m.type !== "rpc.response") return false;
        const data = (m.data as Record<string, JsonValue> | undefined) ?? undefined;
        return Boolean(data && data.request_id === requestId);
      },
      15_000,
    );
    const ok = (resp.data as Record<string, JsonValue> | undefined)?.ok;
    if (ok !== true) {
      const err = (resp.data as Record<string, JsonValue> | undefined)?.error;
      throw new Error(typeof err === "string" ? err : "rpc failed");
    }
    const runId = resp.run_id;
    if (typeof runId !== "string" || !runId) throw new Error("missing run_id in rpc response");
    console.log(JSON.stringify({ run_id: runId }, null, 2));
    return;
  }

  if (
    cmd === "ws-rpc-fs-read" ||
    cmd === "ws-rpc-fs-search" ||
    cmd === "ws-rpc-fs-list" ||
    cmd === "ws-rpc-git-status" ||
    cmd === "ws-rpc-git-diff"
  ) {
    const server = getArg("--server") ?? (await resolveServer()).server;
    const token = getArg("--token") ?? (await resolveToken()).token;
    const runId = getArg("--run");
    if (!server || !token || !runId) usage();

    const requestId = crypto.randomUUID();
    const rpcType =
      cmd === "ws-rpc-fs-read"
        ? "rpc.fs.read"
        : cmd === "ws-rpc-fs-search"
          ? "rpc.fs.search"
          : cmd === "ws-rpc-fs-list"
            ? "rpc.fs.list"
          : cmd === "ws-rpc-git-status"
            ? "rpc.git.status"
            : "rpc.git.diff";

    const data: Record<string, JsonValue> = { request_id: requestId, actor: "cli" };
    if (rpcType === "rpc.fs.read") {
      const path = getArg("--path");
      if (!path) usage();
      data.path = path;
    }
    if (rpcType === "rpc.fs.search") {
      const q = getArg("--q");
      if (!q) usage();
      data.q = q;
    }
    if (rpcType === "rpc.fs.list") {
      const path = getArg("--path");
      if (path) data.path = path;
    }
    if (rpcType === "rpc.git.diff") {
      const path = getArg("--path");
      if (path) data.path = path;
    }

    const env: Record<string, JsonValue> = {
      type: rpcType,
      ts: new Date().toISOString(),
      run_id: runId,
      data,
    };

    const resp = await wsRpc(
      server,
      token,
      env,
      (m) => {
        if (m.type !== "rpc.response") return false;
        const respData = (m.data as Record<string, JsonValue> | undefined) ?? undefined;
        return Boolean(respData && respData.request_id === requestId);
      },
      15_000,
    );
    console.log(JSON.stringify(resp, null, 2));
    return;
  }

  if (cmd === "ws-rpc-run-stop") {
    const server = getArg("--server") ?? (await resolveServer()).server;
    const token = getArg("--token") ?? (await resolveToken()).token;
    const runId = getArg("--run");
    const signal = getArg("--signal") ?? "term";
    if (!server || !token || !runId) usage();
    if (signal !== "term" && signal !== "kill") throw new Error("invalid --signal (expected term|kill)");

    const requestId = crypto.randomUUID();
    const env: Record<string, JsonValue> = {
      type: "rpc.run.stop",
      ts: new Date().toISOString(),
      run_id: runId,
      data: { request_id: requestId, actor: "cli", signal },
    };

    const resp = await wsRpc(
      server,
      token,
      env,
      (m) => {
        if (m.type !== "rpc.response") return false;
        const respData = (m.data as Record<string, JsonValue> | undefined) ?? undefined;
        return Boolean(respData && respData.request_id === requestId);
      },
      15_000,
    );
    console.log(JSON.stringify(resp, null, 2));
    return;
  }

  if (cmd === "ws-rpc-runs-list") {
    const server = getArg("--server") ?? (await resolveServer()).server;
    const token = getArg("--token") ?? (await resolveToken()).token;
    const runId = getArg("--run");
    if (!server || !token || !runId) usage();

    const requestId = crypto.randomUUID();
    const env: Record<string, JsonValue> = {
      type: "rpc.runs.list",
      ts: new Date().toISOString(),
      run_id: runId,
      data: { request_id: requestId, actor: "cli" },
    };

    const resp = await wsRpc(
      server,
      token,
      env,
      (m) => {
        if (m.type !== "rpc.response") return false;
        const respData = (m.data as Record<string, JsonValue> | undefined) ?? undefined;
        return Boolean(respData && respData.request_id === requestId);
      },
      15_000,
    );
    console.log(JSON.stringify(resp, null, 2));
    return;
  }

  if (cmd === "ws-rpc-host-info" || cmd === "ws-rpc-host-doctor") {
    const server = getArg("--server") ?? (await resolveServer()).server;
    const token = getArg("--token") ?? (await resolveToken()).token;
    const hostId = getArg("--host-id");
    if (!server || !token || !hostId) usage();

    const requestId = crypto.randomUUID();
    const rpcType = cmd === "ws-rpc-host-info" ? "rpc.host.info" : "rpc.host.doctor";
    const env: Record<string, JsonValue> = {
      type: rpcType,
      ts: new Date().toISOString(),
      data: { request_id: requestId, actor: "cli", host_id: hostId },
    };

    const resp = await wsRpc(
      server,
      token,
      env,
      (m) => {
        if (m.type !== "rpc.response") return false;
        const respData = (m.data as Record<string, JsonValue> | undefined) ?? undefined;
        return Boolean(respData && respData.request_id === requestId);
      },
      15_000,
    );
    console.log(JSON.stringify(resp, null, 2));
    return;
  }

  if (cmd === "ws-rpc-host-capabilities" || cmd === "ws-rpc-host-logs-tail") {
    const server = getArg("--server") ?? (await resolveServer()).server;
    const token = getArg("--token") ?? (await resolveToken()).token;
    const hostId = getArg("--host-id");
    if (!server || !token || !hostId) usage();

    const requestId = crypto.randomUUID();
    const rpcType = cmd === "ws-rpc-host-capabilities" ? "rpc.host.capabilities" : "rpc.host.logs.tail";
    const data: Record<string, JsonValue> = { request_id: requestId, actor: "cli", host_id: hostId };
    if (rpcType === "rpc.host.logs.tail") {
      const lines = getArg("--lines");
      const maxBytes = getArg("--max-bytes");
      if (lines) data.lines = Number(lines);
      if (maxBytes) data.max_bytes = Number(maxBytes);
    }

    const env: Record<string, JsonValue> = {
      type: rpcType,
      ts: new Date().toISOString(),
      data,
    };

    const resp = await wsRpc(
      server,
      token,
      env,
      (m) => {
        if (m.type !== "rpc.response") return false;
        const respData = (m.data as Record<string, JsonValue> | undefined) ?? undefined;
        return Boolean(respData && respData.request_id === requestId);
      },
      15_000,
    );
    console.log(JSON.stringify(resp, null, 2));
    return;
  }

  usage();
}

main().catch((err) => {
  console.error(err?.stack || String(err));
  process.exit(2);
});
