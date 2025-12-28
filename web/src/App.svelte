<script lang="ts">
  type Health = { name: string; version: string };
  type LoginResponse = { access_token: string };
  type RunRow = {
    id: string;
    host_id: string;
    tool: string;
    cwd: string;
    status: string;
    started_at: string;
    ended_at?: string | null;
    exit_code?: number | null;
  };

  type WsEnvelope = {
    type: string;
    ts: string;
    host_id?: string;
    run_id?: string;
    seq?: number;
    data: unknown;
  };

  let baseUrl = localStorage.getItem("relay.baseUrl") ?? "http://127.0.0.1:8787";
  let username = "admin";
  let password = "";
  let token = "";
  let status = "disconnected";
  let health: Health | null = null;
  let events: WsEnvelope[] = [];
  let runs: RunRow[] = [];
  let ws: WebSocket | null = null;

  let selectedRunId = "";
  let inputText = "";
  let lastError = "";

  function toWsBase(url: string) {
    return url.replace(/^http:/, "ws:").replace(/^https:/, "wss:").replace(/\/$/, "");
  }

  async function connect() {
    lastError = "";
    try {
      status = "checking";
      events = [];
      health = null;

      const h = await fetch(`${baseUrl.replace(/\/$/, "")}/health`);
      if (!h.ok) throw new Error(`health failed: ${h.status}`);
      health = (await h.json()) as Health;

      const l = await fetch(`${baseUrl.replace(/\/$/, "")}/auth/login`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ username, password }),
      });
      if (!l.ok) throw new Error(`login failed: ${l.status}`);
      const login = (await l.json()) as LoginResponse;
      token = login.access_token;

      localStorage.setItem("relay.baseUrl", baseUrl);

      await refreshRuns();

      status = "connecting";
      ws = new WebSocket(`${toWsBase(baseUrl)}/ws/app?token=${encodeURIComponent(token)}`);
      ws.onopen = () => (status = "connected");
      ws.onclose = () => (status = "disconnected");
      ws.onerror = () => (status = "error");
      ws.onmessage = (ev) => {
        try {
          const msg = JSON.parse(ev.data) as WsEnvelope;
          events = [msg, ...events].slice(0, 2000);
          // Best-effort local run status updates.
          if (msg.run_id) {
            const i = runs.findIndex((x) => x.id === msg.run_id);
            if (i !== -1) {
              if (msg.type === "run.awaiting_input") runs[i] = { ...runs[i], status: "awaiting_input" };
              if (msg.type === "run.exited") runs[i] = { ...runs[i], status: "exited" };
              runs = [...runs];
            }
          }
        } catch {
          // ignore
        }
      };
    } catch (e) {
      lastError = e instanceof Error ? e.message : String(e);
      status = "error";
    }
  }

  async function refreshRuns() {
    if (!token) return;
    const r = await fetch(`${baseUrl.replace(/\/$/, "")}/runs`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    if (r.ok) {
      runs = (await r.json()) as RunRow[];
      if (!selectedRunId && runs.length > 0) selectedRunId = runs[0].id;
    } else {
      runs = [];
    }
  }

  function runIds(): string[] {
    const ids = new Set<string>(runs.map((r) => r.id));
    for (const e of events) if (e.run_id) ids.add(e.run_id);
    return Array.from(ids);
  }

  function sendWs(env: WsEnvelope) {
    if (!ws || ws.readyState !== WebSocket.OPEN) return;
    ws.send(JSON.stringify(env));
  }

  function sendInput(text: string) {
    if (!selectedRunId) return;
    sendWs({
      type: "run.send_input",
      ts: new Date().toISOString(),
      run_id: selectedRunId,
      data: { input_id: crypto.randomUUID(), actor: "web", text },
    });
  }

  function sendStop(signal: "term" | "kill" = "term") {
    if (!selectedRunId) return;
    sendWs({
      type: "run.stop",
      ts: new Date().toISOString(),
      run_id: selectedRunId,
      data: { signal },
    });
  }

  $: selectedRun = runs.find((r) => r.id === selectedRunId) ?? null;
  $: awaitingRuns = runs.filter((r) => r.status === "awaiting_input");
</script>

<main>
  <h1>relay</h1>

  <section>
    <label>
      Server URL
      <input bind:value={baseUrl} placeholder="http://host:8787" />
    </label>
    <label>
      Username
      <input bind:value={username} />
    </label>
    <label>
      Password
      <input type="password" bind:value={password} />
    </label>
    <button on:click={connect}>Connect</button>
    <div>Status: {status}</div>
    {#if health}
      <div>Health: {health.name} {health.version}</div>
    {/if}
    {#if lastError}
      <div style="color:#b91c1c">{lastError}</div>
    {/if}
  </section>

  <section>
    <h2>Runs</h2>
    <button on:click={refreshRuns} disabled={!token}>Refresh</button>
    {#if awaitingRuns.length > 0}
      <div style="margin-top:8px;padding:8px;border:1px solid #f59e0b;background:#fffbeb">
        <strong>Needs input:</strong>
        {#each awaitingRuns as r (r.id)}
          <div><code>{r.id}</code> <code>host={r.host_id}</code> <code>tool={r.tool}</code></div>
        {/each}
      </div>
    {/if}
    {#if runs.length === 0}
      <div>No runs loaded yet.</div>
    {:else}
      <ul>
        {#each runs as r (r.id)}
          <li>
            <button on:click={() => (selectedRunId = r.id)} style="margin-right:8px">Select</button>
            <code>{r.id}</code>
            <strong style={r.status === "awaiting_input" ? "color:#b45309" : ""}>{r.status}</strong>
            <code>host={r.host_id}</code>
            <code>tool={r.tool}</code>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section>
    <h2>Send Input</h2>
    <div style="margin:8px 0">
      <strong>Selected:</strong>
      {#if selectedRun}
        <code>{selectedRun.id}</code> <code>{selectedRun.status}</code> <code>{selectedRun.tool}</code>
      {:else}
        <span>none</span>
      {/if}
    </div>
    <div style="display:flex;gap:8px;flex-wrap:wrap;margin-bottom:8px">
      <button on:click={() => sendInput("y\n")} disabled={!selectedRunId || status !== "connected"}>Approve (y)</button>
      <button on:click={() => sendInput("n\n")} disabled={!selectedRunId || status !== "connected"}>Deny (n)</button>
      <button on:click={() => sendStop("term")} disabled={!selectedRunId || status !== "connected"}>Stop</button>
    </div>
    <label>
      Text
      <input bind:value={inputText} placeholder="e.g. y\n or your feedback" />
    </label>
    <button
      on:click={() => {
        sendInput(inputText);
        inputText = "";
      }}
      disabled={!selectedRunId || status !== "connected"}
    >
      Send
    </button>
  </section>

  <section>
    <h2>Events</h2>
    <pre>{JSON.stringify(events[0] ?? null, null, 2)}</pre>
    <ul>
      {#each events as e (e.ts + e.type + (e.seq ?? 0))}
        <li>
          <code>{e.ts}</code> <strong>{e.type}</strong>
          {#if e.host_id}<code> host={e.host_id}</code>{/if}
          {#if e.run_id}<code> run={e.run_id}</code>{/if}
          {#if e.seq !== undefined}<code> seq={e.seq}</code>{/if}
        </li>
      {/each}
    </ul>
  </section>
</main>

<style>
  main {
    font-family: system-ui, -apple-system, Segoe UI, Roboto, sans-serif;
    max-width: 720px;
    margin: 0 auto;
    padding: 16px;
  }
  label {
    display: block;
    margin: 8px 0;
  }
  input {
    width: 100%;
    padding: 10px;
    box-sizing: border-box;
  }
  select {
    width: 100%;
    padding: 10px;
    box-sizing: border-box;
  }
  button {
    padding: 10px 14px;
    margin-top: 8px;
  }
  ul {
    padding-left: 18px;
  }
</style>
