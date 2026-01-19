#!/usr/bin/env node
/* eslint-disable no-console */

const fs = require("fs/promises");
const path = require("path");
const os = require("os");

function log(msg) {
  console.log(`[relay-cli postinstall] ${msg}`);
}

function warn(msg) {
  console.warn(`[relay-cli postinstall] warning: ${msg}`);
}

function envTruthy(name) {
  const v = process.env[name];
  if (!v) return false;
  switch (String(v).trim().toLowerCase()) {
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

async function isExecutable(filePath) {
  try {
    await fs.access(filePath, require("fs").constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

function platformId() {
  const platform = process.platform;
  const arch = process.arch;

  if (platform !== "darwin" && platform !== "linux") return null;
  if (arch !== "x64" && arch !== "arm64") return null;

  return { os: platform, arch };
}

function stripTrailingSlash(url) {
  return String(url || "").replace(/\/$/, "");
}

async function loadPackageVersion() {
  try {
    const pkgPath = path.join(__dirname, "..", "package.json");
    const raw = await fs.readFile(pkgPath, "utf8");
    const pkg = JSON.parse(raw);
    if (!pkg || typeof pkg.version !== "string" || !pkg.version.trim()) return null;
    return pkg.version.trim();
  } catch {
    return null;
  }
}

async function loadRepositoryUrl() {
  try {
    const pkgPath = path.join(__dirname, "..", "package.json");
    const raw = await fs.readFile(pkgPath, "utf8");
    const pkg = JSON.parse(raw);
    const repo = pkg && pkg.repository ? pkg.repository : null;
    const url = repo && typeof repo.url === "string" ? repo.url : null;
    return url && String(url).trim() ? String(url).trim() : null;
  } catch {
    return null;
  }
}

function normalizeRepositoryHttpUrl(raw) {
  const u = String(raw || "").trim();
  if (!u) return null;
  const cleaned = u.replace(/^git\+/, "").replace(/\.git$/, "").replace(/\/$/, "");
  if (cleaned.startsWith("http://") || cleaned.startsWith("https://")) return cleaned;

  if (cleaned.startsWith("ssh://")) {
    try {
      const url = new URL(cleaned);
      const host = url.hostname;
      const pathname = String(url.pathname || "").replace(/^\/+/, "").replace(/\/$/, "");
      if (!host || !pathname) return null;
      return `https://${host}/${pathname}`;
    } catch {
      return null;
    }
  }

  // scp-like git URL: git@github.com:owner/repo
  const m = cleaned.match(/^(?:[^@]+@)?([^:/]+):(.+)$/);
  if (m) {
    const host = String(m[1] || "").trim();
    const pathname = String(m[2] || "").replace(/^\/+/, "").replace(/\/$/, "");
    if (!host || !pathname) return null;
    return `https://${host}/${pathname}`;
  }

  return null;
}

async function defaultReleaseBaseUrl(version) {
  const repo = await loadRepositoryUrl();
  const http = repo ? normalizeRepositoryHttpUrl(repo) : null;
  if (http) return `${http}/releases/download/v${version}`;
  return `https://github.com/aipper/relay/releases/download/v${version}`;
}

async function downloadToFile(url, dst) {
  if (typeof fetch !== "function") throw new Error("node fetch is not available (need Node 18+)");
  const res = await fetch(url, { redirect: "follow" });
  if (!res.ok) {
    const body = await res.text().catch(() => "");
    throw new Error(`download failed: ${res.status} ${res.statusText} ${body}`.trim());
  }
  const buf = Buffer.from(await res.arrayBuffer());
  await fs.writeFile(dst, buf);
}

async function safeRm(filePath) {
  try {
    await fs.unlink(filePath);
  } catch {
    // ignore
  }
}

async function main() {
  if (envTruthy("RELAY_SKIP_POSTINSTALL") || envTruthy("RELAY_NO_POSTINSTALL")) {
    log("skip (RELAY_SKIP_POSTINSTALL/RELAY_NO_POSTINSTALL is set)");
    return;
  }

  const id = platformId();
  if (!id) {
    log(`skip (unsupported platform: ${process.platform}/${process.arch})`);
    return;
  }

  const version = process.env.RELAY_RELEASE_VERSION || (await loadPackageVersion());
  if (!version) {
    warn("skip (cannot resolve package version)");
    return;
  }

  const baseUrl = process.env.RELAY_RELEASE_BASE_URL || (await defaultReleaseBaseUrl(version));
  const installDir = path.join(os.homedir(), ".relay", "bin");
  const relayHome = path.join(os.homedir(), ".relay");
  const tmpBase = path.join(relayHome, "tmp-install-");

  const hostdUrl = `${stripTrailingSlash(baseUrl)}/relay-hostd-${id.os}-${id.arch}`;
  const relayUrl = `${stripTrailingSlash(baseUrl)}/relay-${id.os}-${id.arch}`;
  const hostdDst = path.join(installDir, "relay-hostd");
  const relayDst = path.join(installDir, "relay");

  const force = envTruthy("RELAY_POSTINSTALL_FORCE") || envTruthy("RELAY_HOSTD_INSTALL_FORCE");

  const hostdOk = await isExecutable(hostdDst);
  const relayOk = await isExecutable(relayDst);
  if (!force && hostdOk && relayOk) {
    log(`already installed at ${installDir} (set RELAY_POSTINSTALL_FORCE=1 to overwrite)`);
    return;
  }

  try {
    await fs.mkdir(relayHome, { recursive: true, mode: 0o700 });
    await fs.mkdir(installDir, { recursive: true, mode: 0o700 });
  } catch (e) {
    warn(`cannot create install dir (${installDir}): ${e && e.message ? e.message : String(e)}`);
    return;
  }

  const tmpDir = await fs.mkdtemp(tmpBase).catch((e) => {
    warn(`cannot create temp dir in ${relayHome}: ${e && e.message ? e.message : String(e)}`);
    return null;
  });
  if (!tmpDir) return;

  const hostdTmp = path.join(tmpDir, `relay-hostd-${id.os}-${id.arch}`);
  const relayTmp = path.join(tmpDir, `relay-${id.os}-${id.arch}`);

  try {
    if (force || !hostdOk) {
      log(`downloading relay-hostd from ${hostdUrl}`);
      await downloadToFile(hostdUrl, hostdTmp);
      await fs.chmod(hostdTmp, 0o755).catch(() => {});
      if (force) await safeRm(hostdDst);
      await fs.rename(hostdTmp, hostdDst);
    } else {
      log(`keep existing relay-hostd at ${hostdDst}`);
    }

    if (force || !relayOk) {
      log(`downloading relay (mcp) from ${relayUrl}`);
      await downloadToFile(relayUrl, relayTmp);
      await fs.chmod(relayTmp, 0o755).catch(() => {});
      if (force) await safeRm(relayDst);
      await fs.rename(relayTmp, relayDst);
    } else {
      log(`keep existing relay at ${relayDst}`);
    }

    log(`installed: ${hostdDst}`);
    log(`installed: ${relayDst}`);
    log("next: run `relay init --server http://<your-vps>:8787` then `relay codex` (auto-starts hostd)");
  } catch (e) {
    warn(e && e.message ? e.message : String(e));
    warn("installation failed (non-fatal); you can later run: `relay hostd install`");
  } finally {
    try {
      await fs.rm(tmpDir, { recursive: true, force: true });
    } catch {
      // ignore
    }
  }
}

main().catch((e) => {
  warn(e && e.message ? e.message : String(e));
  warn("postinstall failed (non-fatal); you can later run: `relay hostd install`");
});
