#!/usr/bin/env bun

type JsonValue = unknown;

function usage(): never {
  console.log(`relay (skeleton)

Usage:
  relay init [--server http://127.0.0.1:8787] [--start-daemon] [--install-systemd-user] [--yes] [--force]

  relay auth status
  relay auth login --server http://127.0.0.1:8787 --username admin --password '...' [--save]
  relay auth logout

  relay hostd install [--version 0.1.0] [--base-url <url>] [--dir ~/.relay/bin] [--yes] [--force] [--dry-run]
  relay hostd uninstall [--dir ~/.relay/bin] [--yes]

  relay codex  [--sock /tmp/relay-hostd.sock] [--cmd "codex ..."] [--cwd .]    (default: codex)
  relay claude [--sock /tmp/relay-hostd.sock] [--cmd "claude ..."] [--cwd .]   (default: claude)
  relay iflow  [--sock /tmp/relay-hostd.sock] [--cmd "iflow ..."] [--cwd .]    (default: iflow)
  relay gemini [--sock /tmp/relay-hostd.sock] [--cmd "gemini ..."] [--cwd .]   (default: gemini)

  relay daemon start [--server http://127.0.0.1:8787] [--host-id <id>] [--host-token <token>]
                    [--sock ~/.relay/relay-hostd.sock] [--spool ~/.relay/hostd-spool.db] [--log ~/.relay/hostd.log]
                    [--hostd-config ~/.config/abrelay/hostd.json]
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
  relay ws-rpc-fs-write  --server http://127.0.0.1:8787 --token <jwt> --run <run_id> --path relative/file.txt --content "hello"
  relay ws-rpc-git-status --server http://127.0.0.1:8787 --token <jwt> --run <run_id>
  relay ws-rpc-git-diff   --server http://127.0.0.1:8787 --token <jwt> --run <run_id> [--path relative/file.txt]
  relay ws-rpc-bash       --server http://127.0.0.1:8787 --token <jwt> --run <run_id> --cmd "ls -la"
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
  - local commands auto-start relay-hostd if the unix socket is missing (requires server config via 'relay init' or '--server ...').
  - to auto-confirm downloads/prompts: pass --yes/-y or set RELAY_YES=1 (also used by runtime auto-install).
  - npm install (macOS/Linux): best-effort downloads relay-hostd into ~/.relay/bin; if it failed, run: relay hostd install.
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

function envTruthy(name: string): boolean {
  const v = envOrUndefined(name);
  if (!v) return false;
  switch (v.trim().toLowerCase()) {
    case "1":
    case "true":
    case "yes":
    case "y":
    case "on":
      return true;
    default:
      return false;
  }
}

function isInteractive(): boolean {
  return Boolean(process.stdin.isTTY && process.stdout.isTTY);
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

function xdgConfigHomeDir(): string | null {
  const raw = envOrUndefined("XDG_CONFIG_HOME");
  if (raw && raw.trim()) return raw.trim().replace(/\/$/, "");
  const home = envOrUndefined("HOME");
  if (!home || !home.trim()) return null;
  return `${home.trim().replace(/\/$/, "")}/.config`;
}

function defaultHostdConfigPath(): string {
  const base = xdgConfigHomeDir();
  if (!base) throw new Error("cannot resolve hostd config path (missing HOME/XDG_CONFIG_HOME)");
  return `${base}/abrelay/hostd.json`;
}

async function defaultHostId(): Promise<string> {
  const os = await import("node:os");
  const raw = os.hostname();
  const short = raw.split(".")[0]?.trim() || raw.trim() || "unknown";
  return `host-${short}`;
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

function isValidServerUrl(u: string): boolean {
  const s = u.trim();
  return s.startsWith("http://") || s.startsWith("https://") || s.startsWith("ws://") || s.startsWith("wss://");
}

async function promptLine(label: string, def?: string): Promise<string> {
  const rl = (await import("node:readline/promises")).createInterface({
    input: process.stdin,
    output: process.stdout,
  });
  try {
    const q = def && def.trim() ? `${label} [${def.trim()}]: ` : `${label}: `;
    const out = (await rl.question(q)).trim();
    return out || (def ?? "");
  } finally {
    rl.close();
  }
}

async function confirmYesNo(label: string, defYes: boolean): Promise<boolean> {
  const def = defYes ? "Y/n" : "y/N";
  const v = (await promptLine(`${label} (${def})`)).trim().toLowerCase();
  if (!v) return defYes;
  return v === "y" || v === "yes";
}

function randomToken(): string {
  const bytes = new Uint8Array(16);
  if (!globalThis.crypto?.getRandomValues) {
    throw new Error("crypto.getRandomValues is not available; cannot generate host token");
  }
  globalThis.crypto.getRandomValues(bytes);

  // RFC 4122 v4
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;

  const hex = Array.from(bytes, (b) => b.toString(16).padStart(2, "0"));
  return `${hex.slice(0, 4).join("")}-${hex.slice(4, 6).join("")}-${hex.slice(6, 8).join("")}-${hex
    .slice(8, 10)
    .join("")}-${hex.slice(10, 16).join("")}`;
}

async function readJsonFileIfExists(path: string): Promise<Record<string, unknown> | null> {
  const file = Bun.file(path);
  if (!(await file.exists())) return null;
  const raw = await file.text();
  const parsed = JSON.parse(raw) as unknown;
  if (!parsed || typeof parsed !== "object") return null;
  return parsed as Record<string, unknown>;
}

async function writeJsonFile(path: string, obj: Record<string, unknown>): Promise<void> {
  const fs = await import("node:fs/promises");
  const p = await import("node:path");
  await fs.mkdir(p.dirname(path), { recursive: true, mode: 0o700 });
  await Bun.write(path, JSON.stringify(obj, null, 2) + "\n");
  try {
    await fs.chmod(path, 0o600);
  } catch {
    // ignore (e.g. non-posix)
  }
}

function fileIsExecutable(path: string): boolean {
  return (
    Bun.spawnSync(["bash", "-lc", `test -x ${JSON.stringify(path)}`], {
      stdout: "ignore",
      stderr: "ignore",
    }).exitCode === 0
  );
}

function platformId(): { os: "darwin" | "linux"; arch: "x64" | "arm64" } {
  const os = process.platform;
  const arch = process.arch;
  if (os !== "darwin" && os !== "linux") throw new Error(`unsupported platform: ${os} (expected darwin/linux)`);
  if (arch !== "x64" && arch !== "arm64") throw new Error(`unsupported arch: ${arch} (expected x64/arm64)`);
  return { os, arch };
}

async function packageVersion(): Promise<string | null> {
  try {
    const pkgUrl = new URL("../package.json", import.meta.url);
    const raw = await Bun.file(pkgUrl).text();
    const parsed = JSON.parse(raw) as unknown;
    if (!parsed || typeof parsed !== "object") return null;
    const obj = parsed as Record<string, unknown>;
    return typeof obj.version === "string" && obj.version.trim() ? obj.version.trim() : null;
  } catch {
    return null;
  }
}

async function packageRepositoryUrl(): Promise<string | null> {
  try {
    const pkgUrl = new URL("../package.json", import.meta.url);
    const raw = await Bun.file(pkgUrl).text();
    const parsed = JSON.parse(raw) as unknown;
    if (!parsed || typeof parsed !== "object") return null;
    const obj = parsed as Record<string, unknown>;
    const repo = obj.repository;
    if (!repo || typeof repo !== "object") return null;
    const url = (repo as Record<string, unknown>).url;
    return typeof url === "string" && url.trim() ? url.trim() : null;
  } catch {
    return null;
  }
}

function normalizeRepositoryHttpUrl(raw: string): string | null {
  const u = raw.trim();
  if (!u) return null;
  // npm supports `git+https://...`
  const cleaned = u.replace(/^git\+/, "").replace(/\.git$/, "").replace(/\/$/, "");
  if (cleaned.startsWith("http://") || cleaned.startsWith("https://")) return cleaned;

  if (cleaned.startsWith("ssh://")) {
    try {
      const url = new URL(cleaned);
      const host = url.hostname;
      const pathname = url.pathname.replace(/^\/+/, "").replace(/\/$/, "");
      if (!host || !pathname) return null;
      return `https://${host}/${pathname}`;
    } catch {
      return null;
    }
  }

  // scp-like git URL: git@github.com:owner/repo
  const scp = cleaned.match(/^(?:[^@]+@)?([^:/]+):(.+)$/);
  if (scp) {
    const host = (scp[1] ?? "").trim();
    const pathname = (scp[2] ?? "").replace(/^\/+/, "").replace(/\/$/, "");
    if (!host || !pathname) return null;
    return `https://${host}/${pathname}`;
  }

  return null;
}

async function defaultReleaseBaseUrl(version: string): Promise<string> {
  const repo = await packageRepositoryUrl();
  const http = repo ? normalizeRepositoryHttpUrl(repo) : null;
  if (http) return `${http}/releases/download/v${version}`;
  // Fallback: legacy default (can always be overridden via RELAY_RELEASE_BASE_URL / --base-url).
  return `https://github.com/aipper/relay/releases/download/v${version}`;
}

async function downloadToFile(url: string, dst: string): Promise<void> {
  const res = await fetch(url, { redirect: "follow" });
  if (!res.ok) {
    const body = await res.text().catch(() => "");
    throw new Error(`download failed: ${res.status} ${res.statusText} ${body}`.trim());
  }
  const buf = await res.arrayBuffer();
  await Bun.write(dst, buf);
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

function spawnSyncText(argv: string[]): { exitCode: number; stdout: string; stderr: string } {
  const r = Bun.spawnSync(argv, { stdout: "pipe", stderr: "pipe" });
  const td = new TextDecoder();
  const stdout = r.stdout ? td.decode(r.stdout).trim() : "";
  const stderr = r.stderr ? td.decode(r.stderr).trim() : "";
  return { exitCode: r.exitCode ?? 0, stdout, stderr };
}

function systemdUserUnitDirOrThrow(): string {
  if (process.platform !== "linux") {
    throw new Error("systemd user service is supported on Linux only");
  }

  requireBinaryInPath("systemctl");

  const envRes = spawnSyncText(["systemctl", "--user", "show-environment"]);
  if (envRes.exitCode !== 0) {
    throw new Error(
      "systemctl --user is not available (no user systemd session). Hint: loginctl enable-linger <user> or install as a system service.",
    );
  }

  let systemdXdgConfigHome = "";
  for (const line of envRes.stdout.split("\n")) {
    const idx = line.indexOf("=");
    if (idx <= 0) continue;
    const key = line.slice(0, idx).trim();
    const val = line.slice(idx + 1).trim();
    if (key === "XDG_CONFIG_HOME") {
      systemdXdgConfigHome = val;
      break;
    }
  }

  const base = systemdXdgConfigHome || xdgConfigHomeDir() || `${envOrUndefined("HOME") ?? ""}/.config`;
  const trimmed = base.replace(/\/$/, "");
  if (!trimmed) throw new Error("cannot resolve systemd user unit directory (missing HOME/XDG_CONFIG_HOME)");
  return `${trimmed}/systemd/user`;
}

async function writeFileWithOptionalBackup(args: {
  path: string;
  content: string;
  mode?: number;
  force: boolean;
}): Promise<{ wrote: true; backupPath?: string }> {
  const fs = await import("node:fs/promises");
  const p = await import("node:path");

  const dir = p.dirname(args.path);
  await fs.mkdir(dir, { recursive: true, mode: 0o700 });

  let backupPath: string | undefined;
  const exists = await fs
    .stat(args.path)
    .then(() => true)
    .catch(() => false);
  if (exists) {
    if (!args.force) throw new Error(`destination exists: ${args.path} (use --force to overwrite)`);
    const ts = new Date().toISOString().replace(/[-:]/g, "").replace(/\..*$/, "");
    backupPath = `${args.path}.bak.${ts}`;
    await fs.copyFile(args.path, backupPath);
  }

  await fs.writeFile(args.path, args.content, { encoding: "utf8" });
  if (args.mode !== undefined) {
    await fs.chmod(args.path, args.mode).catch(() => {});
  }
  return { wrote: true, ...(backupPath ? { backupPath } : {}) };
}

async function installSystemdUserService(args: { hostdBin: string; hostdConfigPath: string; force: boolean }) {
  if (process.platform !== "linux") throw new Error("systemd install is supported on Linux only");

  const home = envOrUndefined("HOME");
  if (!home) throw new Error("HOME is not set; cannot install systemd user service");

  const unitDir = systemdUserUnitDirOrThrow();
  const unitPath = `${unitDir}/relay-hostd.service`;
  const envPath = `${relayHomeDir()}/hostd.env`;

  const rustLog = (envOrUndefined("RUST_LOG") ?? "warn").trim() || "warn";
  const pathEnv = envOrUndefined("PATH") ?? "";

  const envLines = [`ABRELAY_CONFIG=${args.hostdConfigPath}`, `RUST_LOG=${rustLog}`];
  if (pathEnv.trim()) envLines.push(`PATH=${pathEnv}`);
  const envContent = envLines.join("\n") + "\n";

  const unitContent =
    `[Unit]
Description=relay-hostd (user)
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
EnvironmentFile=%h/.relay/hostd.env
ExecStart=${args.hostdBin}
Restart=always
RestartSec=2
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=false
ReadWritePaths=%h/.relay

[Install]
WantedBy=default.target
`;

  const envWrite = await writeFileWithOptionalBackup({ path: envPath, content: envContent, mode: 0o600, force: args.force });
  const unitWrite = await writeFileWithOptionalBackup({ path: unitPath, content: unitContent, mode: 0o644, force: args.force });

  const reload = spawnSyncText(["systemctl", "--user", "daemon-reload"]);
  if (reload.exitCode !== 0) throw new Error("systemctl --user daemon-reload failed");

  const enable = spawnSyncText(["systemctl", "--user", "enable", "--now", "relay-hostd.service"]);
  if (enable.exitCode !== 0) {
    const enableByPath = spawnSyncText(["systemctl", "--user", "enable", "--now", unitPath]);
    if (enableByPath.exitCode !== 0) {
      const fallbackDir = `${home.replace(/\/$/, "")}/.config/systemd/user`;
      const fallbackPath = `${fallbackDir}/relay-hostd.service`;
      const fs = await import("node:fs/promises");
      await fs.mkdir(fallbackDir, { recursive: true, mode: 0o700 }).catch(() => {});
      await fs.copyFile(unitPath, fallbackPath).catch(() => {});
      spawnSyncText(["systemctl", "--user", "daemon-reload"]);

      const enableFallback = spawnSyncText(["systemctl", "--user", "enable", "--now", "relay-hostd.service"]);
      if (enableFallback.exitCode !== 0) {
        throw new Error("failed to enable relay-hostd.service via systemctl --user");
      }
    }
  }

  return {
    installed: true,
    env: { path: envPath, ...(envWrite.backupPath ? { backup: envWrite.backupPath } : {}) },
    unit: { path: unitPath, ...(unitWrite.backupPath ? { backup: unitWrite.backupPath } : {}) },
    next: ["systemctl --user status relay-hostd", "journalctl --user -u relay-hostd -f", "loginctl enable-linger <user> (optional)"],
  };
}

function resolveHostdBin(): string {
  const env = envOrUndefined("RELAY_HOSTD_BIN");
  if (env) return env;

  // Prefer a user-local install (relay hostd install).
  try {
    const installed = `${relayHomeDir()}/bin/relay-hostd`;
    if (fileIsExecutable(installed)) return installed;
  } catch {
    // ignore
  }

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

function wantsYes(): boolean {
  return hasFlag("--yes") || hasFlag("-y") || envTruthy("RELAY_YES");
}

async function pathIsSocket(p: string): Promise<boolean> {
  const fs = await import("node:fs/promises");
  try {
    const st = await fs.lstat(p);
    return st.isSocket();
  } catch {
    return false;
  }
}

async function waitForSocket(p: string, timeoutMs: number): Promise<boolean> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    if (await pathIsSocket(p)) return true;
    await new Promise((r) => setTimeout(r, 100));
  }
  return await pathIsSocket(p);
}

async function ensureHostdInstalled(): Promise<void> {
  const env = envOrUndefined("RELAY_HOSTD_BIN");
  if (env) {
    if (!fileIsExecutable(env)) throw new Error(`RELAY_HOSTD_BIN is set but not executable: ${env}`);
    return;
  }

  // Fast path: already installed.
  const installedHostd = `${relayHomeDir()}/bin/relay-hostd`;
  if (fileIsExecutable(installedHostd)) return;

  const inPath = Bun.spawnSync(["bash", "-lc", "command -v relay-hostd 2>/dev/null || true"], {
    stdout: "pipe",
    stderr: "ignore",
  });
  const found = (inPath.stdout ? new TextDecoder().decode(inPath.stdout).trim() : "").trim();
  if (found && fileIsExecutable(found)) return;

  const dev = `${process.cwd()}/target/debug/relay-hostd`;
  if (fileIsExecutable(dev)) return;

  if (!isInteractive() && !wantsYes()) {
    throw new Error("relay-hostd is not installed; run `relay hostd install` (or set RELAY_HOSTD_BIN)");
  }

  const ver = envOrUndefined("RELAY_RELEASE_VERSION") ?? (await packageVersion());
  if (!ver) throw new Error("relay-hostd is not installed and package version is unavailable; run `relay hostd install --version <ver>`");

  const { os, arch } = platformId();
  const baseUrl = envOrUndefined("RELAY_RELEASE_BASE_URL") ?? (await defaultReleaseBaseUrl(ver));
  const dir = `${relayHomeDir()}/bin`;
  const hostdUrl = `${baseUrl.replace(/\/$/, "")}/relay-hostd-${os}-${arch}`;
  const relayUrl = `${baseUrl.replace(/\/$/, "")}/relay-${os}-${arch}`;

  if (!wantsYes()) {
    const ok = await confirmYesNo(`relay-hostd is not installed. Install now?\n  ${hostdUrl}\n  ${relayUrl}\n-> ${dir}`, true);
    if (!ok) throw new Error("hostd install aborted");
  }

  const fs = await import("node:fs/promises");
  const p = await import("node:path");
  const hostdDst = p.join(dir, "relay-hostd");
  const relayDst = p.join(dir, "relay");
  const needsHostd = !fileIsExecutable(hostdDst);
  const needsRelay = !fileIsExecutable(relayDst);
  if (!needsHostd && !needsRelay) return;

  await fs.mkdir(relayHomeDir(), { recursive: true, mode: 0o700 });
  const tmpDir = await fs.mkdtemp(p.join(relayHomeDir(), "tmp-install-"));
  const hostdTmp = p.join(tmpDir, `relay-hostd-${os}-${arch}`);
  const relayTmp = p.join(tmpDir, `relay-${os}-${arch}`);
  await fs.mkdir(dir, { recursive: true, mode: 0o700 });

  try {
    if (needsHostd) {
      await downloadToFile(hostdUrl, hostdTmp);
      await fs.chmod(hostdTmp, 0o755);
      await fs.rename(hostdTmp, hostdDst);
    }
    if (needsRelay) {
      await downloadToFile(relayUrl, relayTmp);
      await fs.chmod(relayTmp, 0o755);
      await fs.rename(relayTmp, relayDst);
    }
  } finally {
    try {
      await fs.rm(tmpDir, { recursive: true, force: true });
    } catch {
      // ignore
    }
  }

  if (!fileIsExecutable(hostdDst)) throw new Error("hostd install failed (relay-hostd not executable after install)");
}

type StartDaemonResult =
  | { started: true; pid: number; sock: string; log: string }
  | { started: false; reason: "already_running"; pid: number; sock: string; log: string };

async function startDaemonDetached(args?: {
  serverHttp?: string;
  hostIdOverride?: string;
  hostTokenOverride?: string;
  sockOverride?: string;
  spoolOverride?: string;
  logOverride?: string;
  hostdConfigOverride?: string;
}): Promise<StartDaemonResult> {
  await ensureHostdInstalled();

  const serverHttp = args?.serverHttp ?? (await resolveServer()).server;
  const serverWs =
    serverHttp.startsWith("ws://") || serverHttp.startsWith("wss://") ? stripTrailingSlash(serverHttp) : toWsBaseFromHttp(serverHttp);

  const hostdConfig = args?.hostdConfigOverride ?? envOrUndefined("ABRELAY_CONFIG") ?? defaultHostdConfigPath();
  const hostdCfg = await readJsonFileIfExists(hostdConfig);
  const cfgSock = hostdCfg && typeof hostdCfg.local_unix_socket === "string" ? String(hostdCfg.local_unix_socket) : "";
  const cfgSpool = hostdCfg && typeof hostdCfg.spool_db_path === "string" ? String(hostdCfg.spool_db_path) : "";
  const cfgLog = hostdCfg && typeof hostdCfg.log_path === "string" ? String(hostdCfg.log_path) : "";
  const cfgHostId = hostdCfg && typeof hostdCfg.host_id === "string" ? String(hostdCfg.host_id) : "";
  const cfgHostToken = hostdCfg && typeof hostdCfg.host_token === "string" ? String(hostdCfg.host_token) : "";

  const hostIdOverride = args?.hostIdOverride ?? envOrUndefined("RELAY_HOST_ID");
  const hostTokenOverride = args?.hostTokenOverride ?? envOrUndefined("RELAY_HOST_TOKEN");

  const sockArg = args?.sockOverride ?? envOrUndefined("RELAY_HOSTD_SOCK");
  const sock = sockArg && sockArg.trim() ? sockArg.trim() : cfgSock.trim() ? cfgSock.trim() : `${relayHomeDir()}/relay-hostd.sock`;

  const spoolArg = args?.spoolOverride ?? envOrUndefined("RELAY_SPOOL_DB");
  const spool = spoolArg && spoolArg.trim() ? spoolArg.trim() : cfgSpool.trim() ? cfgSpool.trim() : `${relayHomeDir()}/hostd-spool.db`;

  const logArg = args?.logOverride ?? envOrUndefined("RELAY_HOSTD_LOG");
  const log = logArg && logArg.trim() ? logArg.trim() : cfgLog.trim() ? cfgLog.trim() : `${relayHomeDir()}/hostd.log`;

  const hostdBin = resolveHostdBin();
  if (!fileIsExecutable(hostdBin)) throw new Error(`relay-hostd is not executable: ${hostdBin}`);

  const fs = await import("node:fs/promises");
  const p = await import("node:path");
  await fs.mkdir(p.dirname(log), { recursive: true });
  await fs.mkdir(p.dirname(sock), { recursive: true });
  await fs.mkdir(p.dirname(spool), { recursive: true });

  const existing = await readDaemonState();
  if (existing && isProcessRunning(existing.pid)) {
    return { started: false, reason: "already_running", pid: existing.pid, sock: existing.sock, log: existing.log };
  }
  if (existing && !isProcessRunning(existing.pid)) await clearDaemonState();

  // Best-effort: clear stale socket file.
  try {
    await fs.unlink(sock);
  } catch {
    // ignore
  }

  // Spawn detached hostd.
  const out = await fs.open(log, "a");
  try {
    const hostId = hostIdOverride ?? (cfgHostId.trim() ? cfgHostId.trim() : await defaultHostId());
    const child = Bun.spawn([hostdBin], {
      env: {
        ...process.env,
        ABRELAY_CONFIG: hostdConfig,
        SERVER_BASE_URL: serverWs,
        LOCAL_UNIX_SOCKET: sock,
        SPOOL_DB_PATH: spool,
        HOSTD_LOG_PATH: log,
        ...(hostIdOverride || !cfgHostId.trim() ? { HOST_ID: hostId } : {}),
        ...(hostTokenOverride ? { HOST_TOKEN: hostTokenOverride } : {}),
      },
      stdout: out.fd,
      stderr: out.fd,
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
      host_token: hostTokenOverride ?? cfgHostToken.trim() ?? "",
      sock,
      spool,
      log,
      hostd_bin: hostdBin,
    };
    await writeDaemonState(state);

    return { started: true, pid, sock, log };
  } finally {
    try {
      await out.close();
    } catch {
      // ignore
    }
  }
}

async function ensureDaemonRunning(sock: string): Promise<void> {
  if (await pathIsSocket(sock)) return;

  const existing = await readDaemonState();
  if (existing && isProcessRunning(existing.pid)) {
    if (existing.sock !== sock) {
      if (await pathIsSocket(existing.sock)) {
        throw new Error(`relay-hostd is running, but sock mismatch: running=${existing.sock} requested=${sock}`);
      }
    } else {
      // Give hostd a moment to create the socket after spawn.
      if (await waitForSocket(sock, 5000)) return;
      throw new Error(`relay-hostd is running (pid=${existing.pid}) but unix socket not ready: ${sock} (log: ${existing.log})`);
    }
  }

  const res = await startDaemonDetached({ sockOverride: sock });
  const pid = res.pid;
  const log = res.log;
  if (res.started === false && res.reason === "already_running") {
    if (res.sock !== sock) {
      throw new Error(`relay-hostd is running, but sock mismatch: running=${res.sock} requested=${sock}`);
    }
  }

  const ok = await waitForSocket(sock, 5000);
  if (!ok) throw new Error(`relay-hostd started (pid=${pid}) but unix socket not ready: ${sock} (log: ${log})`);
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

  if (cmd === "init") {
    const yes = hasFlag("--yes") || hasFlag("-y");
    const force = hasFlag("--force");
    const startDaemon = hasFlag("--start-daemon") || envTruthy("RELAY_START_DAEMON");
    const installSystemdUser = hasFlag("--install-systemd-user") || envTruthy("RELAY_INSTALL_SYSTEMD_USER");
    if (startDaemon && installSystemdUser) {
      throw new Error("cannot use --start-daemon and --install-systemd-user together");
    }

    const prev = await readSettings();
    const serverDefault = prev.server;
    let serverHttp = getArg("--server") ?? envOrUndefined("RELAY_SERVER") ?? serverDefault ?? "";
    if (!serverHttp.trim()) {
      serverHttp = await promptLine("relay-server URL (http(s)://host:port)", "");
    }
    serverHttp = stripTrailingSlash(serverHttp.trim());
    if (!isValidServerUrl(serverHttp)) {
      throw new Error("invalid --server (expected http(s)://... or ws(s)://...)");
    }

    if (!yes) {
      const ok = await confirmYesNo(`Save server to ${settingsPath()}?`, true);
      if (!ok) {
        console.log(JSON.stringify({ saved: false, reason: "user_aborted" }, null, 2));
        return;
      }
    }

    await writeSettings({ ...prev, server: serverHttp });

    const hostdPath = getArg("--hostd-config") ?? defaultHostdConfigPath();
    const existing = await readJsonFileIfExists(hostdPath);

    const next: Record<string, unknown> = existing ? { ...existing } : {};
    const serverWs =
      serverHttp.startsWith("ws://") || serverHttp.startsWith("wss://") ? serverHttp : toWsBaseFromHttp(serverHttp);
    next.server_base_url = serverWs;

    const hostIdArg = getArg("--host-id") ?? envOrUndefined("RELAY_HOST_ID");
    const existingHostId = typeof next.host_id === "string" ? (next.host_id as string) : "";
    {
      const hostId = (hostIdArg ?? existingHostId ?? "").trim();
      next.host_id = hostId || (await defaultHostId());
    }

    const rotateToken = hasFlag("--rotate-token");
    const existingToken = typeof next.host_token === "string" ? (next.host_token as string) : "";
    if (!existingToken || rotateToken) {
      if (rotateToken && existing && !yes) {
        const ok = await confirmYesNo(
          "Rotate host token? (This will break TOFU on an existing server unless you also change host_id or delete the host record)",
          false,
        );
        if (!ok) {
          console.log(
            JSON.stringify({ saved: true, hostd_config: { updated: false, reason: "token_rotation_aborted" } }, null, 2),
          );
          return;
        }
      }
      next.host_token = randomToken();
    }

    const relayHome = relayHomeDir();
    const sockDefault = `${relayHome}/relay-hostd.sock`;
    const spoolDefault = `${relayHome}/hostd-spool.db`;
    const logDefault = `${relayHome}/hostd.log`;

    if (next.local_unix_socket === undefined) next.local_unix_socket = sockDefault;
    if (next.spool_db_path === undefined) next.spool_db_path = spoolDefault;
    if (next.log_path === undefined) next.log_path = logDefault;
    if (next.redaction_extra_regex === undefined) next.redaction_extra_regex = [];

    if (existing && !force) {
      // Default behavior: update server only and keep existing identity to avoid breaking TOFU.
      const safe: Record<string, unknown> = {
        ...existing,
        server_base_url: next.server_base_url,
      };
      await writeJsonFile(hostdPath, safe);
    } else {
      await writeJsonFile(hostdPath, next);
    }

    const systemd = await (async () => {
      if (!installSystemdUser) return null;
      const existing = await readDaemonState();
      if (existing && isProcessRunning(existing.pid)) {
        throw new Error(`relay daemon is running (pid=${existing.pid}); stop it first: relay daemon stop`);
      }
      await ensureHostdInstalled();
      const hostdBin = resolveHostdBin();
      if (!fileIsExecutable(hostdBin)) throw new Error(`relay-hostd is not executable: ${hostdBin}`);
      return await installSystemdUserService({ hostdBin, hostdConfigPath: hostdPath, force });
    })();

    const daemon = await (async () => {
      if (!startDaemon) return null;
      const res = await startDaemonDetached({ serverHttp, hostdConfigOverride: hostdPath });
      return res.started
        ? { started: true, pid: res.pid, sock: res.sock, log: res.log }
        : { started: false, reason: res.reason, pid: res.pid, sock: res.sock, log: res.log };
    })();

    console.log(
      JSON.stringify(
        {
          saved: true,
          settings: { path: settingsPath(), server: serverHttp },
          hostd: { config_path: hostdPath, server_base_url: serverWs, host_id: next.host_id },
          ...(systemd ? { systemd } : {}),
          ...(daemon ? { daemon } : {}),
        },
        null,
        2,
      ),
    );
    return;
  }

  if (cmd === "hostd") {
    const sub = requireCmd(process.argv[3]);

    if (sub === "install") {
      const yes = hasFlag("--yes") || hasFlag("-y");
      const force = hasFlag("--force");
      const dryRun = hasFlag("--dry-run");

      const ver = getArg("--version") ?? envOrUndefined("RELAY_RELEASE_VERSION") ?? (await packageVersion());
      if (!ver) throw new Error("missing --version and package version is unavailable");

      const { os, arch } = platformId();
      const baseUrl =
        getArg("--base-url") ?? envOrUndefined("RELAY_RELEASE_BASE_URL") ?? (await defaultReleaseBaseUrl(ver));

      const dir = getArg("--dir") ?? `${relayHomeDir()}/bin`;
      const hostdUrl = `${baseUrl.replace(/\/$/, "")}/relay-hostd-${os}-${arch}`;
      const relayUrl = `${baseUrl.replace(/\/$/, "")}/relay-${os}-${arch}`;

      const fs = await import("node:fs/promises");
      const p = await import("node:path");

      const hostdDst = p.join(dir, "relay-hostd");
      const relayDst = p.join(dir, "relay");

      if (!force) {
        if (fileIsExecutable(hostdDst) || fileIsExecutable(relayDst)) {
          throw new Error(`already installed at ${dir} (use --force to overwrite)`);
        }
      }

      if (!yes && !dryRun) {
        const ok = await confirmYesNo(
          `Download and install native binaries?\n  ${hostdUrl}\n  ${relayUrl}\n-> ${dir}`,
          true,
        );
        if (!ok) {
          console.log(JSON.stringify({ installed: false, reason: "user_aborted" }, null, 2));
          return;
        }
      }

      if (dryRun) {
        console.log(
          JSON.stringify(
            {
              dry_run: true,
              version: ver,
              platform: { os, arch },
              urls: { relay_hostd: hostdUrl, relay: relayUrl },
              install_dir: dir,
              dest: { relay_hostd: hostdDst, relay: relayDst },
            },
            null,
            2,
          ),
        );
        return;
      }

      await fs.mkdir(relayHomeDir(), { recursive: true, mode: 0o700 });
      const tmpDir = await fs.mkdtemp(p.join(relayHomeDir(), "tmp-install-"));
      const hostdTmp = p.join(tmpDir, `relay-hostd-${os}-${arch}`);
      const relayTmp = p.join(tmpDir, `relay-${os}-${arch}`);

      await fs.mkdir(dir, { recursive: true, mode: 0o700 });

      await downloadToFile(hostdUrl, hostdTmp);
      await downloadToFile(relayUrl, relayTmp);
      await fs.chmod(hostdTmp, 0o755);
      await fs.chmod(relayTmp, 0o755);

      if (force) {
        try {
          await fs.unlink(hostdDst);
        } catch {
          // ignore
        }
        try {
          await fs.unlink(relayDst);
        } catch {
          // ignore
        }
      }

      await fs.rename(hostdTmp, hostdDst);
      await fs.rename(relayTmp, relayDst);

      try {
        await fs.rmdir(tmpDir);
      } catch {
        // ignore
      }

      console.log(
        JSON.stringify(
          {
            installed: true,
            dir,
            binaries: { relay_hostd: hostdDst, relay: relayDst },
            next: ["relay init --server http://<your-vps>:8787", "relay daemon start"],
          },
          null,
          2,
        ),
      );
      return;
    }

    if (sub === "uninstall") {
      const yes = hasFlag("--yes") || hasFlag("-y");
      const dir = getArg("--dir") ?? `${relayHomeDir()}/bin`;
      const p = await import("node:path");
      const fs = await import("node:fs/promises");
      const hostdDst = p.join(dir, "relay-hostd");
      const relayDst = p.join(dir, "relay");

      if (!yes) {
        const ok = await confirmYesNo(`Remove installed binaries?\n  ${hostdDst}\n  ${relayDst}`, false);
        if (!ok) {
          console.log(JSON.stringify({ removed: false, reason: "user_aborted" }, null, 2));
          return;
        }
      }

      let removed = 0;
      for (const f of [hostdDst, relayDst]) {
        try {
          await fs.unlink(f);
          removed += 1;
        } catch {
          // ignore
        }
      }
      console.log(JSON.stringify({ removed: true, count: removed, dir }, null, 2));
      return;
    }

    usage();
  }

  if (cmd === "runs") {
    const sub = requireCmd(process.argv[3]);
    const sock = getArg("--sock") ?? envOrUndefined("RELAY_HOSTD_SOCK") ?? `${relayHomeDir()}/relay-hostd.sock`;
    if (!sock) usage();
    await ensureDaemonRunning(sock);

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
    const sock = getArg("--sock") ?? envOrUndefined("RELAY_HOSTD_SOCK") ?? `${relayHomeDir()}/relay-hostd.sock`;
    const runId = getArg("--run");
    if (!sock || !runId) usage();
    await ensureDaemonRunning(sock);

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
    const sock = getArg("--sock") ?? envOrUndefined("RELAY_HOSTD_SOCK") ?? `${relayHomeDir()}/relay-hostd.sock`;
    const runId = getArg("--run");
    if (!sock || !runId) usage();
    await ensureDaemonRunning(sock);

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
      const serverHttp = getArg("--server");
      const hostIdOverride = getArg("--host-id");
      const hostTokenOverride = getArg("--host-token");
      const sockOverride = getArg("--sock");
      const spoolOverride = getArg("--spool");
      const logOverride = getArg("--log");
      const hostdConfigOverride = getArg("--hostd-config");

      const res = await startDaemonDetached({
        ...(serverHttp ? { serverHttp } : {}),
        ...(hostIdOverride ? { hostIdOverride } : {}),
        ...(hostTokenOverride ? { hostTokenOverride } : {}),
        ...(sockOverride ? { sockOverride } : {}),
        ...(spoolOverride ? { spoolOverride } : {}),
        ...(logOverride ? { logOverride } : {}),
        ...(hostdConfigOverride ? { hostdConfigOverride } : {}),
      });

      if (res.started === false) {
        console.log(JSON.stringify({ started: false, reason: res.reason, pid: res.pid, sock: res.sock, log: res.log }, null, 2));
        return;
      }
      console.log(JSON.stringify({ started: true, pid: res.pid, sock: res.sock, log: res.log }, null, 2));
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
      Bun.spawnSync(["bash", "-lc", `test -x ${JSON.stringify(hostdBin)}`], {
        stdout: "pipe",
        stderr: "ignore",
      }).exitCode === 0;
    checks.push({ name: "relay-hostd", ok: hostdExists, detail: hostdBin });
    try {
      const p = await import("node:path");
      const relayBin = p.join(p.dirname(hostdBin), "relay");
      checks.push({ name: "relay (mcp)", ok: fileIsExecutable(relayBin), detail: relayBin });
    } catch {
      // ignore
    }

    const state = await readDaemonState();
    if (state) {
      checks.push({ name: "daemon.running", ok: isProcessRunning(state.pid), detail: `pid=${state.pid}` });
      const sockOk =
        Bun.spawnSync(["bash", "-lc", `test -S ${JSON.stringify(state.sock)}`], {
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
    const sock = getArg("--sock") ?? envOrUndefined("RELAY_HOSTD_SOCK") ?? `${relayHomeDir()}/relay-hostd.sock`;
    const runCmd = cmdOrDefault(getArg("--cmd"), cmd);
    const cwd = getArg("--cwd");
    if (!sock) usage();
    await ensureDaemonRunning(sock);
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
    await ensureDaemonRunning(sock);

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
    cmd === "ws-rpc-fs-write" ||
    cmd === "ws-rpc-git-status" ||
    cmd === "ws-rpc-git-diff" ||
    cmd === "ws-rpc-bash"
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
            : cmd === "ws-rpc-fs-write"
              ? "rpc.fs.write"
              : cmd === "ws-rpc-git-status"
                ? "rpc.git.status"
                : cmd === "ws-rpc-bash"
                  ? "rpc.bash"
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
    if (rpcType === "rpc.fs.write") {
      const path = getArg("--path");
      const content = getArg("--content");
      if (!path || content === undefined) usage();
      data.path = path;
      data.content = content;
    }
    if (rpcType === "rpc.git.diff") {
      const path = getArg("--path");
      if (path) data.path = path;
    }
    if (rpcType === "rpc.bash") {
      const cmdline = getArg("--cmd");
      if (!cmdline) usage();
      data.cmd = cmdline;
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
      rpcType === "rpc.fs.write" || rpcType === "rpc.bash" ? 600_000 : 15_000,
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
