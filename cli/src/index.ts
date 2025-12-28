#!/usr/bin/env bun

type JsonValue = unknown;

function usage(): never {
  console.log(`relay (skeleton)

Usage:
  relay login --server http://127.0.0.1:8787 --username admin --password '...'
  relay local start --sock /tmp/relay-hostd.sock --tool codex --cmd "echo hi; read -p 'Proceed? ' x; echo ok"
  relay local input --sock /tmp/relay-hostd.sock --run <run_id> --text "y\\n"
  relay ws-send-input --server http://127.0.0.1:8787 --token <jwt> --run <run_id> --text "y\\n"

Notes:
  - This CLI is a thin control layer. The long-running tool processes should be owned by hostd.
  - local commands use curl --unix-socket (requires curl in PATH).
`);
  process.exit(1);
}

function getArg(flag: string): string | undefined {
  const idx = process.argv.indexOf(flag);
  if (idx === -1) return undefined;
  return process.argv[idx + 1];
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

async function main() {
  const cmd = process.argv[2];
  if (!cmd) usage();

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
      const runCmd = getArg("--cmd");
      const cwd = getArg("--cwd");
      if (!tool || !runCmd) usage();

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
      console.log(out);
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
    const server = getArg("--server");
    const token = getArg("--token");
    const runId = getArg("--run");
    const text = getArg("--text");
    const inputId = getArg("--input-id") ?? crypto.randomUUID();
    if (!server || !token || !runId || text === undefined) usage();

    const wsUrl = server
      .replace(/^http:/, "ws:")
      .replace(/^https:/, "wss:")
      .replace(/\/$/, "");

    await new Promise<void>((resolve, reject) => {
      const ws = new WebSocket(`${wsUrl}/ws/app?token=${encodeURIComponent(token)}`);
      ws.onopen = () => {
        ws.send(
          JSON.stringify({
            type: "run.send_input",
            ts: new Date().toISOString(),
            run_id: runId,
            data: { input_id: inputId, actor: "cli", text },
          }),
        );
        ws.close();
        resolve();
      };
      ws.onerror = () => reject(new Error("websocket error"));
    });

    console.log("sent");
    return;
  }

  usage();
}

main().catch((err) => {
  console.error(err?.stack || String(err));
  process.exit(2);
});
