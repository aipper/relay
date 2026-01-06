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

  type ChatMessage = {
    key: string;
    ts: string;
    role: "user" | "assistant" | "system";
    kind: string;
    actor?: string | null;
    request_id?: string | null;
    text: string;
  };

  type ChatMessageApi = {
    id: number;
    ts: string;
    role: string;
    kind: string;
    actor?: string | null;
    request_id?: string | null;
    text: string;
  };

  type HostInfo = {
    id: string;
    name?: string | null;
    last_seen_at?: string | null;
    online: boolean;
  };

  type WsEnvelope = {
    type: string;
    ts: string;
    host_id?: string;
    run_id?: string;
    seq?: number;
    data: unknown;
  };

  const browserOrigin =
    typeof window !== "undefined" && typeof window.location?.origin === "string" ? window.location.origin : "";

  let useCustomServer = false;
  let customBaseUrl = "";
  if (typeof window !== "undefined") {
    const savedBaseUrl = localStorage.getItem("relay.baseUrl") ?? "";
    const savedUseCustom = localStorage.getItem("relay.useCustomServer") === "1";
    customBaseUrl = savedBaseUrl;
    // Prefer current page (same-origin) by default; only opt into a custom server explicitly.
    useCustomServer = savedUseCustom || (!browserOrigin && Boolean(savedBaseUrl));
  }

  $: apiBaseUrl =
    (useCustomServer ? customBaseUrl.trim() : browserOrigin) ||
    customBaseUrl.trim() ||
    "http://127.0.0.1:8787";

  let username = "admin";
  let password = "";
  let token = "";
  let status = "disconnected";
  let view: "sessions" | "hosts" | "start" | "tools" | "settings" = "sessions";
  let health: Health | null = null;
  let events: WsEnvelope[] = [];
  let runs: RunRow[] = [];
  let hosts: HostInfo[] = [];
  let ws: WebSocket | null = null;
  let messagesByRun: Record<string, ChatMessage[]> = {};

  let selectedRunId = "";
  let inputText = "";
  let lastError = "";
  let outputByRun: Record<string, string> = {};
  let awaitingByRun: Record<string, { reason?: string; prompt?: string; request_id?: string } | undefined> = {};
  let filePath = "README.md";
  let fileContent = "";
  let fileError = "";

  let searchQuery = "TODO";
  let searchMatches: SearchMatch[] = [];
  let searchTruncated = false;
  let searchError = "";

  let gitDiffPath = "";
  let gitStatus = "";
  let gitDiff = "";
  let gitError = "";

  let hostDiagHostId = "host-dev";
  let hostInfo = "";
  let hostDoctor = "";
  let hostCapabilities = "";
  let hostLogs = "";
  let hostLogsLines = "200";
  let hostLogsMaxBytes = "200000";
  let hostDiagError = "";

  type TodoItem = { id: string; text: string; done: boolean; created_at: string };
  let todos: TodoItem[] = [];
  let todoText = "";

  let startHostId = "host-dev";
  let startTool = "codex";
  let startCmd = "echo Proceed?; cat";
  let startCwd = "";
  let startError = "";
  let recentSessions: RunRow[] = [];

  const pendingRpc = new Map<string, (msg: WsEnvelope) => void>();

  type SearchMatch = { path: string; line: number; column: number; text: string };

  function persistServerPrefs() {
    if (typeof window === "undefined") return;
    localStorage.setItem("relay.useCustomServer", useCustomServer ? "1" : "0");
    if (customBaseUrl.trim()) localStorage.setItem("relay.baseUrl", customBaseUrl.trim());
    else localStorage.removeItem("relay.baseUrl");
  }

  function isRecord(v: unknown): v is Record<string, unknown> {
    return Boolean(v) && typeof v === "object";
  }

  function escapeHtml(s: string): string {
    return s
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#39;");
  }

  function linkifySafe(s: string): string {
    // Minimal markdown link: [text](https://example.com)
    // Only allow http(s) to avoid javascript: URLs.
    const re = /\[([^\]]+)\]\(([^)]+)\)/g;
    return s.replace(re, (_m, text, href) => {
      const t = escapeHtml(String(text));
      const rawHref = String(href).trim();
      if (!/^https?:\/\//i.test(rawHref)) return t;
      const h = escapeHtml(rawHref);
      return `<a href="${h}" target="_blank" rel="noreferrer noopener">${t}</a>`;
    });
  }

  function extractMarkdownLinks(raw: string): { text: string; links: string[] } {
    const links: string[] = [];
    const re = /\[([^\]]+)\]\(([^)]+)\)/g;
    const text = raw.replace(re, (_m, text, href) => {
      const label = String(text);
      const rawHref = String(href).trim();
      const idx = links.length;
      if (!/^https?:\/\//i.test(rawHref)) return label;
      links.push(
        `<a href="${escapeHtml(rawHref)}" target="_blank" rel="noreferrer noopener">${escapeHtml(label)}</a>`,
      );
      return `\u0000L${idx}\u0000`;
    });
    return { text, links };
  }

  function restoreMarkdownLinks(escapedWithTokens: string, links: string[]): string {
    return escapedWithTokens.replace(/\u0000L(\d+)\u0000/g, (_m, i) => {
      const idx = Number(i);
      if (!Number.isFinite(idx)) return "";
      return links[idx] ?? "";
    });
  }

  function extractInlineCode(raw: string): { text: string; codes: string[] } {
    const codes: string[] = [];
    let out = "";
    let i = 0;
    while (i < raw.length) {
      const ch = raw[i] ?? "";
      if (ch !== "`") {
        out += ch;
        i++;
        continue;
      }
      const end = raw.indexOf("`", i + 1);
      if (end === -1) {
        out += ch;
        i++;
        continue;
      }
      const code = raw.slice(i + 1, end);
      const idx = codes.length;
      codes.push(
        `<code style="background:#f3f4f6;border:1px solid #e5e7eb;padding:1px 4px;border-radius:4px">${escapeHtml(
          code,
        )}</code>`,
      );
      out += `\u0000C${idx}\u0000`;
      i = end + 1;
    }
    return { text: out, codes };
  }

  function restoreInlineCode(escapedWithTokens: string, codes: string[]): string {
    return escapedWithTokens.replace(/\u0000C(\d+)\u0000/g, (_m, i) => {
      const idx = Number(i);
      if (!Number.isFinite(idx)) return "";
      return codes[idx] ?? "";
    });
  }

  function renderInlineMarkdown(raw: string): string {
    const { text: withCode, codes } = extractInlineCode(raw);
    const { text, links } = extractMarkdownLinks(withCode);
    let s = escapeHtml(text);

    // Bold / italic (best-effort, non-nested).
    s = s.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
    s = s.replace(/__([^_]+)__/g, "<strong>$1</strong>");
    s = s.replace(/\*([^*]+)\*/g, "<em>$1</em>");
    s = s.replace(/_([^_]+)_/g, "<em>$1</em>");

    s = restoreMarkdownLinks(s, links);
    s = restoreInlineCode(s, codes);
    return s;
  }

  function renderMarkdownTextBlock(raw: string): string {
    const lines = raw.split("\n");
    let i = 0;
    const blocks: string[] = [];

    const flushParagraph = (buf: string[]) => {
      if (buf.length === 0) return;
      const body = buf.map((l) => renderInlineMarkdown(l)).join("<br/>");
      blocks.push(`<p style="margin:6px 0">${body}</p>`);
      buf.length = 0;
    };

    while (i < lines.length) {
      const line = lines[i] ?? "";
      const trimmed = line.trimEnd();

      if (trimmed.trim() === "") {
        i++;
        continue;
      }

      // Headings
      const h = trimmed.match(/^(#{1,6})\s+(.*)$/);
      if (h) {
        const level = h[1]?.length ?? 1;
        const text = renderInlineMarkdown(h[2] ?? "");
        blocks.push(`<h${level} style="margin:10px 0 6px 0">${text}</h${level}>`);
        i++;
        continue;
      }

      // Blockquote (consecutive)
      if (trimmed.startsWith(">")) {
        const q: string[] = [];
        while (i < lines.length) {
          const l = (lines[i] ?? "").trimEnd();
          if (!l.startsWith(">")) break;
          q.push(l.replace(/^>\s?/, ""));
          i++;
        }
        const body = q.map((l) => renderInlineMarkdown(l)).join("<br/>");
        blocks.push(
          `<blockquote style="margin:8px 0;padding:8px 10px;border-left:4px solid #d1d5db;background:#f8fafc">${body}</blockquote>`,
        );
        continue;
      }

      // Unordered list
      const ul = trimmed.match(/^[-*+]\s+(.*)$/);
      if (ul) {
        const items: string[] = [];
        while (i < lines.length) {
          const l = (lines[i] ?? "").trimEnd();
          const m = l.match(/^[-*+]\s+(.*)$/);
          if (!m) break;
          items.push(`<li>${renderInlineMarkdown(m[1] ?? "")}</li>`);
          i++;
        }
        blocks.push(`<ul style="margin:8px 0 8px 18px">${items.join("")}</ul>`);
        continue;
      }

      // Ordered list
      const ol = trimmed.match(/^\d+\.\s+(.*)$/);
      if (ol) {
        const items: string[] = [];
        while (i < lines.length) {
          const l = (lines[i] ?? "").trimEnd();
          const m = l.match(/^\d+\.\s+(.*)$/);
          if (!m) break;
          items.push(`<li>${renderInlineMarkdown(m[1] ?? "")}</li>`);
          i++;
        }
        blocks.push(`<ol style="margin:8px 0 8px 18px">${items.join("")}</ol>`);
        continue;
      }

      // Paragraph (until blank line)
      const para: string[] = [];
      while (i < lines.length) {
        const l = (lines[i] ?? "").trimEnd();
        if (l.trim() === "") break;
        if (/^(#{1,6})\s+/.test(l) || l.startsWith(">") || /^[-*+]\s+/.test(l) || /^\d+\.\s+/.test(l)) break;
        para.push(l);
        i++;
      }
      flushParagraph(para);
    }

    return blocks.join("") || `<div style="color:#6b7280">(empty)</div>`;
  }

  function renderMarkdownBasic(src: string): string {
    // Supports:
    // - fenced code blocks ```lang\n...\n```
    // - inline code `...`
    // - links [text](https://...)
    // - headings, blockquotes, lists, bold/italic (best-effort)
    // Everything else is escaped; output uses a small safe tag set.
    const input = src ?? "";
    const parts = input.split("```");
    let out = "";

    for (let i = 0; i < parts.length; i++) {
      const chunk = parts[i] ?? "";
      if (i % 2 === 1) {
        // code block; first line may be language
        const firstNl = chunk.indexOf("\n");
        const body = firstNl === -1 ? chunk : chunk.slice(firstNl + 1);
        out += `<pre style="white-space:pre-wrap;word-break:break-word;margin:8px 0;padding:12px;border:1px solid #e5e7eb;background:#0b1020;color:#e5e7eb;overflow:auto"><code>${escapeHtml(
          body.trimEnd(),
        )}</code></pre>`;
        continue;
      }

      out += renderMarkdownTextBlock(chunk);
    }

    return out || `<div style="color:#6b7280">(empty)</div>`;
  }

  function toolMetaFromText(kind: string, text: string): { label: string; details: string } {
    if (kind === "tool.call" && text.startsWith("tool.call ")) {
      return { label: text.slice("tool.call ".length).split(" ")[0] ?? "tool.call", details: text };
    }
    if (kind === "tool.result" && text.startsWith("tool.result ")) {
      return { label: text.slice("tool.result ".length).split(" ")[0] ?? "tool.result", details: text };
    }
    return { label: kind, details: text };
  }

  function jsonTrunc(v: unknown, maxChars: number): string {
    try {
      const s = JSON.stringify(v);
      if (s.length <= maxChars) return s;
      return `${s.slice(0, maxChars)}…`;
    } catch {
      return "";
    }
  }

  function truncateTail(s: string, maxChars: number) {
    if (s.length <= maxChars) return s;
    return s.slice(s.length - maxChars);
  }

  function dataString(e: WsEnvelope, key: string): string | undefined {
    if (!isRecord(e.data)) return undefined;
    const v = e.data[key];
    return typeof v === "string" ? v : undefined;
  }

  function dataBool(e: WsEnvelope, key: string): boolean | undefined {
    if (!isRecord(e.data)) return undefined;
    const v = e.data[key];
    return typeof v === "boolean" ? v : undefined;
  }

  function dataAny(e: WsEnvelope, key: string): unknown {
    if (!isRecord(e.data)) return undefined;
    return e.data[key];
  }

  function toWsBase(url: string) {
    return url.replace(/^http:/, "ws:").replace(/^https:/, "wss:").replace(/\/$/, "");
  }

  function isProbablyInsecureUrl(url: string) {
    if (!/^http:\/\//i.test(url)) return false;
    try {
      const u = new URL(url);
      const host = u.hostname.toLowerCase();
      if (host === "localhost" || host === "127.0.0.1") return false;
      return true;
    } catch {
      return true;
    }
  }

  function truncateHead<T>(arr: T[], maxLen: number): T[] {
    if (arr.length <= maxLen) return arr;
    return arr.slice(arr.length - maxLen);
  }

  function appendMessage(runId: string, msg: ChatMessage) {
    const existing = messagesByRun[runId] ?? [];
    messagesByRun = { ...messagesByRun, [runId]: truncateHead([...existing, msg], 1000) };
  }

  function envToMessage(env: WsEnvelope): ChatMessage | null {
    if (!env.run_id) return null;
    if (env.type === "run.output") {
      return {
        key: `${env.ts}:run.output:${env.seq ?? crypto.randomUUID()}`,
        ts: env.ts,
        role: "assistant",
        kind: env.type,
        actor: dataString(env, "actor"),
        text: dataString(env, "text") ?? "",
      };
    }
    if (env.type === "run.input") {
      return {
        key: `${env.ts}:run.input:${dataString(env, "input_id") ?? crypto.randomUUID()}`,
        ts: env.ts,
        role: "user",
        kind: env.type,
        actor: dataString(env, "actor"),
        text: dataString(env, "text_redacted") ?? "",
      };
    }
    if (env.type === "run.permission_requested") {
      return {
        key: `${env.ts}:run.permission_requested:${dataString(env, "request_id") ?? crypto.randomUUID()}`,
        ts: env.ts,
        role: "system",
        kind: env.type,
        request_id: dataString(env, "request_id"),
        text: dataString(env, "prompt") ?? "",
      };
    }
    if (env.type === "run.started" || env.type === "run.exited") {
      return {
        key: `${env.ts}:${env.type}:${env.seq ?? crypto.randomUUID()}`,
        ts: env.ts,
        role: "system",
        kind: env.type,
        text: env.type === "run.started" ? "run started" : "run exited",
      };
    }
    if (env.type === "tool.call") {
      return {
        key: `${env.ts}:tool.call:${dataString(env, "request_id") ?? crypto.randomUUID()}`,
        ts: env.ts,
        role: "system",
        kind: env.type,
        request_id: dataString(env, "request_id"),
        actor: dataString(env, "actor"),
        text: `tool.call ${dataString(env, "tool") ?? "unknown"} ${jsonTrunc(dataAny(env, "args"), 2000)}`,
      };
    }
    if (env.type === "tool.result") {
      const ok = dataBool(env, "ok") ?? false;
      const dur = isRecord(env.data) && typeof env.data["duration_ms"] === "number" ? env.data["duration_ms"] : 0;
      const base = `tool.result ${dataString(env, "tool") ?? "unknown"} ok=${ok} duration_ms=${dur}`;
      const extra = ok ? jsonTrunc(dataAny(env, "result"), 2000) : String(dataAny(env, "error") ?? "");
      return {
        key: `${env.ts}:tool.result:${dataString(env, "request_id") ?? crypto.randomUUID()}`,
        ts: env.ts,
        role: "system",
        kind: env.type,
        request_id: dataString(env, "request_id"),
        actor: dataString(env, "actor"),
        text: `${base} ${extra}`.trim(),
      };
    }
    return null;
  }

  async function connect() {
    lastError = "";
    try {
      status = "checking";
      events = [];
      health = null;
      outputByRun = {};
      awaitingByRun = {};
      hosts = [];
      messagesByRun = {};

      if (ws) {
        try {
          ws.close();
        } catch {
          // ignore
        }
        ws = null;
      }

      const h = await fetch(`${apiBaseUrl.replace(/\/$/, "")}/health`);
      if (!h.ok) {
        const body = await h.text().catch(() => "");
        throw new Error(`health failed: ${h.status} ${body}`.trim());
      }
      health = (await h.json()) as Health;

      const l = await fetch(`${apiBaseUrl.replace(/\/$/, "")}/auth/login`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ username, password }),
      });
      if (!l.ok) {
        const body = await l.text().catch(() => "");
        const hint =
          body.includes("bad password hash") || l.status === 500
            ? "（服务端 ADMIN_PASSWORD_HASH 配置无效；请在 VPS 重新运行 scripts/docker-init.sh --reset-password，或设置 ADMIN_PASSWORD 让容器启动时自动生成）"
            : "";
        throw new Error(`login failed: ${l.status} ${body} ${hint}`.trim());
      }
      const login = (await l.json()) as LoginResponse;
      token = login.access_token;
      view = "sessions";

      persistServerPrefs();

      await refreshHosts();
      await refreshRuns();
      if (selectedRunId) await loadMessages(selectedRunId);

      status = "connecting";
      const nextWs = new WebSocket(`${toWsBase(apiBaseUrl)}/ws/app?token=${encodeURIComponent(token)}`);
      ws = nextWs;
      nextWs.onopen = () => {
        if (ws === nextWs) status = "connected";
      };
      nextWs.onclose = () => {
        if (ws === nextWs) status = "disconnected";
      };
      nextWs.onerror = () => {
        if (ws === nextWs) status = "error";
      };
      nextWs.onmessage = (ev) => {
        try {
          const msg = JSON.parse(ev.data) as WsEnvelope;
          events = [msg, ...events].slice(0, 2000);
          if (msg.type === "rpc.response") {
            const reqId = dataString(msg, "request_id");
            if (reqId) {
              const cb = pendingRpc.get(reqId);
              if (cb) {
                pendingRpc.delete(reqId);
                cb(msg);
              }
            }
          }
          if (msg.run_id && msg.type === "run.output") {
            const text = dataString(msg, "text") ?? "";
            const existing = outputByRun[msg.run_id] ?? "";
            outputByRun = { ...outputByRun, [msg.run_id]: truncateTail(existing + text, 200_000) };
          }
          if (msg.run_id && msg.type === "run.awaiting_input") {
            awaitingByRun = {
              ...awaitingByRun,
              [msg.run_id]: {
                reason: dataString(msg, "reason"),
                prompt: dataString(msg, "prompt"),
                request_id: dataString(msg, "request_id"),
              },
            };
          }
          if (msg.run_id && msg.type === "run.permission_requested") {
            awaitingByRun = {
              ...awaitingByRun,
              [msg.run_id]: {
                reason: dataString(msg, "reason"),
                prompt: dataString(msg, "prompt"),
                request_id: dataString(msg, "request_id"),
              },
            };
          }
          if (msg.run_id && msg.type === "run.input") {
            awaitingByRun = { ...awaitingByRun, [msg.run_id]: undefined };
          }
          if (msg.run_id && msg.type === "run.exited") {
            awaitingByRun = { ...awaitingByRun, [msg.run_id]: undefined };
          }

          const m = envToMessage(msg);
          if (m && msg.run_id) appendMessage(msg.run_id, m);

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
      lastError = `${e instanceof Error ? e.message : String(e)}\nserver=${apiBaseUrl}`.trim();
      status = "error";
    }
  }

  function disconnect() {
    if (ws) ws.close();
    ws = null;
    token = "";
    status = "disconnected";
    view = "sessions";
  }

  async function refreshHosts() {
    if (!token) return;
    const r = await fetch(`${apiBaseUrl.replace(/\/$/, "")}/hosts`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    if (r.ok) {
      hosts = (await r.json()) as HostInfo[];
      const online = hosts.filter((h) => h.online);
      if (online.length > 0 && !online.some((h) => h.id === startHostId)) startHostId = online[0].id;
      if (online.length > 0 && !online.some((h) => h.id === hostDiagHostId)) hostDiagHostId = online[0].id;
    } else {
      hosts = [];
    }
  }

  async function refreshRuns() {
    if (!token) return;
    const r = await fetch(`${apiBaseUrl.replace(/\/$/, "")}/sessions`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    if (r.ok) {
      runs = (await r.json()) as RunRow[];
      if (!selectedRunId && runs.length > 0) selectedRunId = runs[0].id;
    } else {
      runs = [];
    }
  }

  async function refreshRecentSessions() {
    if (!token) return;
    const r = await fetch(`${apiBaseUrl.replace(/\/$/, "")}/sessions/recent?limit=50`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    if (r.ok) {
      recentSessions = (await r.json()) as RunRow[];
    } else {
      recentSessions = [];
    }
  }

  async function loadMessages(runId: string) {
    if (!token) return;
    const r = await fetch(
      `${apiBaseUrl.replace(/\/$/, "")}/sessions/${encodeURIComponent(runId)}/messages?limit=200`,
      {
        headers: { Authorization: `Bearer ${token}` },
      },
    );
    if (!r.ok) return;
    const msgs = (await r.json()) as ChatMessageApi[];
    const mapped: ChatMessage[] = msgs.map((m) => ({
      key: String(m.id),
      ts: m.ts,
      role: m.role === "assistant" || m.role === "user" ? m.role : "system",
      kind: m.kind,
      actor: m.actor,
      request_id: m.request_id,
      text: m.text,
    }));
    messagesByRun = { ...messagesByRun, [runId]: mapped };
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

  async function rpcCall(rpcType: string, data: Record<string, unknown>): Promise<WsEnvelope> {
    if (!ws || ws.readyState !== WebSocket.OPEN) throw new Error("ws not connected");
    if (!selectedRunId) throw new Error("missing run_id");
    const requestId = crypto.randomUUID();
    const msg: WsEnvelope = {
      type: rpcType,
      ts: new Date().toISOString(),
      run_id: selectedRunId,
      data: { ...data, request_id: requestId },
    };
    const p = new Promise<WsEnvelope>((resolve) => {
      pendingRpc.set(requestId, resolve);
    });
    sendWs(msg);
    return await p;
  }

  async function rpcCallNoRun(rpcType: string, data: Record<string, unknown>): Promise<WsEnvelope> {
    if (!ws || ws.readyState !== WebSocket.OPEN) throw new Error("ws not connected");
    const requestId = crypto.randomUUID();
    const msg: WsEnvelope = {
      type: rpcType,
      ts: new Date().toISOString(),
      data: { ...data, request_id: requestId },
    };
    const p = new Promise<WsEnvelope>((resolve) => {
      pendingRpc.set(requestId, resolve);
    });
    sendWs(msg);
    return await p;
  }

  async function startRun() {
    startError = "";
    try {
      const data: Record<string, unknown> = {
        host_id: startHostId.trim(),
        tool: startTool.trim(),
        cmd: startCmd,
        cwd: startCwd.trim() ? startCwd.trim() : null,
      };
      const resp = await rpcCallNoRun("rpc.run.start", data);
      const ok = dataBool(resp, "ok");
      if (!ok) throw new Error(dataString(resp, "error") ?? "rpc failed");
      if (resp.run_id) {
        await refreshRuns();
        selectedRunId = resp.run_id;
      }
    } catch (e) {
      startError = e instanceof Error ? e.message : String(e);
    }
  }

  async function fetchFile() {
    fileError = "";
    fileContent = "";
    try {
      const resp = await rpcCall("rpc.fs.read", { path: filePath });
      const ok = dataBool(resp, "ok");
      if (!ok) throw new Error(dataString(resp, "error") ?? "rpc failed");
      const result = dataAny(resp, "result");
      if (!isRecord(result)) throw new Error("bad rpc result");
      fileContent = String(result.content ?? "");
    } catch (e) {
      fileError = e instanceof Error ? e.message : String(e);
    }
  }

  async function searchFiles() {
    searchError = "";
    searchMatches = [];
    searchTruncated = false;
    try {
      const resp = await rpcCall("rpc.fs.search", { q: searchQuery });
      const ok = dataBool(resp, "ok");
      if (!ok) throw new Error(dataString(resp, "error") ?? "rpc failed");
      const result = dataAny(resp, "result");
      if (!isRecord(result)) throw new Error("bad rpc result");
      const matches = result.matches;
      if (!Array.isArray(matches)) throw new Error("bad rpc matches");
      searchMatches = matches
        .map((m) => (isRecord(m) ? m : null))
        .filter(Boolean)
        .map((m) => ({
          path: String(m!.path ?? ""),
          line: Number(m!.line ?? 0),
          column: Number(m!.column ?? 0),
          text: String(m!.text ?? ""),
        }))
        .filter((m) => Boolean(m.path));
      searchTruncated = Boolean(result.truncated);
    } catch (e) {
      searchError = e instanceof Error ? e.message : String(e);
    }
  }

  async function fetchGitStatus() {
    gitError = "";
    gitStatus = "";
    try {
      const resp = await rpcCall("rpc.git.status", {});
      const ok = dataBool(resp, "ok");
      if (!ok) throw new Error(dataString(resp, "error") ?? "rpc failed");
      const result = dataAny(resp, "result");
      if (!isRecord(result)) throw new Error("bad rpc result");
      gitStatus = String(result.stdout ?? "");
    } catch (e) {
      gitError = e instanceof Error ? e.message : String(e);
    }
  }

  async function fetchGitDiff() {
    gitError = "";
    gitDiff = "";
    try {
      const data: Record<string, unknown> = {};
      if (gitDiffPath.trim()) data.path = gitDiffPath.trim();
      const resp = await rpcCall("rpc.git.diff", data);
      const ok = dataBool(resp, "ok");
      if (!ok) throw new Error(dataString(resp, "error") ?? "rpc failed");
      const result = dataAny(resp, "result");
      if (!isRecord(result)) throw new Error("bad rpc result");
      gitDiff = String(result.stdout ?? "");
    } catch (e) {
      gitError = e instanceof Error ? e.message : String(e);
    }
  }

  async function hostRpc(rpcType: string, data: Record<string, unknown>) {
    hostDiagError = "";
    try {
      const resp = await rpcCallNoRun(rpcType, data);
      const ok = dataBool(resp, "ok");
      if (!ok) throw new Error(dataString(resp, "error") ?? "rpc failed");
      const result = dataAny(resp, "result");
      return JSON.stringify(result ?? null, null, 2);
    } catch (e) {
      hostDiagError = e instanceof Error ? e.message : String(e);
      return "";
    }
  }

  async function fetchHostInfo() {
    hostInfo = await hostRpc("rpc.host.info", { host_id: hostDiagHostId });
  }

  async function fetchHostDoctor() {
    hostDoctor = await hostRpc("rpc.host.doctor", { host_id: hostDiagHostId });
  }

  async function fetchHostCapabilities() {
    hostCapabilities = await hostRpc("rpc.host.capabilities", { host_id: hostDiagHostId });
  }

  async function fetchHostLogs() {
    const lines = Number(hostLogsLines);
    const maxBytes = Number(hostLogsMaxBytes);
    hostLogs = await hostRpc("rpc.host.logs.tail", {
      host_id: hostDiagHostId,
      lines: Number.isFinite(lines) ? lines : 200,
      max_bytes: Number.isFinite(maxBytes) ? maxBytes : 200000,
    });
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

  function sendDecision(decision: "approve" | "deny") {
    if (!selectedRunId) return;
    const reqId = selectedAwaiting?.request_id;
    if (!reqId) {
      sendInput(decision === "approve" ? "y\n" : "n\n");
      return;
    }
    sendWs({
      type: decision === "approve" ? "run.permission.approve" : "run.permission.deny",
      ts: new Date().toISOString(),
      run_id: selectedRunId,
      data: { request_id: reqId, actor: "web" },
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
  $: selectedOutput = selectedRunId ? outputByRun[selectedRunId] ?? "" : "";
  $: selectedAwaiting = selectedRunId ? awaitingByRun[selectedRunId] ?? null : null;
  $: selectedMessages = selectedRunId ? messagesByRun[selectedRunId] ?? [] : [];
  $: displayMessages = (() => {
    const msgs = selectedMessages ?? [];
    const out: ChatMessage[] = [];
    for (const m of msgs) {
      const prev = out[out.length - 1];
      if (prev && prev.kind === "run.output" && m.kind === "run.output" && prev.role === "assistant" && m.role === "assistant") {
        prev.text = `${prev.text ?? ""}${m.text ?? ""}`;
        continue;
      }
      out.push({ ...m });
    }
    return out;
  })();

  function todoStorageKey(runId: string) {
    return `relay.todo.${runId}`;
  }

  function loadTodos(runId: string) {
    try {
      const raw = localStorage.getItem(todoStorageKey(runId));
      if (!raw) return [];
      const parsed = JSON.parse(raw) as unknown;
      if (!Array.isArray(parsed)) return [];
      return parsed
        .map((x) => (isRecord(x) ? x : null))
        .filter(Boolean)
        .map((x) => ({
          id: typeof x!.id === "string" ? x!.id : crypto.randomUUID(),
          text: typeof x!.text === "string" ? x!.text : "",
          done: typeof x!.done === "boolean" ? x!.done : false,
          created_at: typeof x!.created_at === "string" ? x!.created_at : new Date().toISOString(),
        }))
        .filter((x) => x.text.trim().length > 0);
    } catch {
      return [];
    }
  }

  function saveTodos(runId: string, next: TodoItem[]) {
    localStorage.setItem(todoStorageKey(runId), JSON.stringify(next));
  }

  function addTodo(text: string) {
    if (!selectedRunId) return;
    const t = text.trim();
    if (!t) return;
    const next = [{ id: crypto.randomUUID(), text: t, done: false, created_at: new Date().toISOString() }, ...todos];
    todos = next;
    saveTodos(selectedRunId, next);
  }

  function toggleTodo(id: string) {
    if (!selectedRunId) return;
    const next = todos.map((t) => (t.id === id ? { ...t, done: !t.done } : t));
    todos = next;
    saveTodos(selectedRunId, next);
  }

  function removeTodo(id: string) {
    if (!selectedRunId) return;
    const next = todos.filter((t) => t.id !== id);
    todos = next;
    saveTodos(selectedRunId, next);
  }

  function extractTodoSuggestions(output: string): string[] {
    const lines = output.split(/\r?\n/);
    const out: string[] = [];
    for (const line of lines) {
      const m1 = line.match(/^\s*TODO\s*:\s*(.+)\s*$/i);
      if (m1?.[1]) out.push(m1[1].trim());
      const m2 = line.match(/^\s*[-*]\s*\[\s*\]\s*(.+)\s*$/);
      if (m2?.[1]) out.push(m2[1].trim());
    }
    const uniq = Array.from(new Set(out.map((s) => s.trim()).filter(Boolean)));
    return uniq.slice(0, 50);
  }

  $: todoSuggestions = extractTodoSuggestions(selectedOutput).filter((s) => !todos.some((t) => t.text === s));

  $: if (selectedRunId) {
    todos = loadTodos(selectedRunId);
  } else {
    todos = [];
  }
</script>

<main>
  <header class="topbar">
    <div>
      <h1 style="margin:0">Relay</h1>
      <div class="subtle">
        {#if health}
          {health.name} {health.version}
        {:else}
          {apiBaseUrl}
        {/if}
      </div>
    </div>
    <div style="display:flex;gap:8px;align-items:center;flex-wrap:wrap;justify-content:flex-end">
      <span class="status-pill" data-kind={status}>{status}</span>
      {#if token}
        <span class="subtle"><code>{username}</code></span>
      {/if}
    </div>
  </header>

  {#if token}
    <nav class="segmented" aria-label="navigation">
      <button class:active={view === "sessions"} on:click={() => (view = "sessions")}>会话</button>
      <button class:active={view === "hosts"} on:click={() => (view = "hosts")}>主机</button>
      <button class:active={view === "start"} on:click={() => (view = "start")}>启动</button>
      <button class:active={view === "tools"} on:click={() => (view = "tools")}>工具</button>
      <button class:active={view === "settings"} on:click={() => (view = "settings")}>设置</button>
    </nav>
  {/if}

  {#if !token}
  <section>
    <div style="display:flex;justify-content:space-between;align-items:center;gap:12px;flex-wrap:wrap">
      <div>
        <div style="font-weight:600">Server</div>
        <div style="font-size:12px;color:#6b7280">
          {#if useCustomServer}
            自定义：<code>{apiBaseUrl}</code>
          {:else}
            当前页面（同源）：<code>{apiBaseUrl}</code>
          {/if}
        </div>
      </div>
      <label style="display:flex;gap:8px;align-items:center;margin:0">
        <input
          type="checkbox"
          bind:checked={useCustomServer}
          on:change={() => {
            persistServerPrefs();
          }}
        />
        使用自定义 Server URL
      </label>
    </div>
    {#if useCustomServer}
      <label>
        Server URL
        <input
          bind:value={customBaseUrl}
          placeholder="http(s)://host:8787"
          on:change={() => {
            persistServerPrefs();
          }}
        />
      </label>
    {/if}
    <label>
      用户名
      <input bind:value={username} />
    </label>
    <label>
      密码
      <input type="password" bind:value={password} />
    </label>
    <div style="display:flex;gap:8px;flex-wrap:wrap">
      <button on:click={connect}>登录</button>
      <button on:click={disconnect} disabled={!ws && !token}>断开</button>
    </div>
    {#if isProbablyInsecureUrl(apiBaseUrl)}
      <div style="margin-top:8px;padding:8px;border:1px solid #f59e0b;background:#fffbeb">
        注意：当前是 <code>http://</code>，密码会明文传输。建议通过 HTTPS 访问（例如用 Caddy 反代）。
      </div>
    {/if}
    <div>Status: {status}</div>
    {#if health}
      <div>Health: {health.name} {health.version}</div>
    {/if}
    {#if lastError}
      <div style="color:#b91c1c">{lastError}</div>
    {/if}
  </section>
  {/if}

  {#if token && view === "settings"}
  <section>
    <h2>设置</h2>
    <div style="display:flex;justify-content:space-between;align-items:center;gap:12px;flex-wrap:wrap">
      <div>
        <div style="font-weight:600">当前服务</div>
        <div class="subtle">
          <code>{apiBaseUrl}</code>
          {#if health}
            <span class="subtle"> · {health.name} {health.version}</span>
          {/if}
        </div>
      </div>
      <div style="display:flex;gap:8px;flex-wrap:wrap">
        <button on:click={disconnect} disabled={!ws && !token}>断开</button>
      </div>
    </div>

    <div style="margin-top:12px">
      <label style="display:flex;gap:8px;align-items:center;margin:0">
        <input
          type="checkbox"
          bind:checked={useCustomServer}
          on:change={() => {
            persistServerPrefs();
          }}
        />
        使用自定义 Server URL（仅当 PWA 与服务不同源时需要）
      </label>
      {#if useCustomServer}
        <label style="margin-top:10px">
          Server URL
          <input
            bind:value={customBaseUrl}
            placeholder="http(s)://host:8787"
            on:change={() => {
              persistServerPrefs();
            }}
          />
        </label>
      {/if}
      {#if isProbablyInsecureUrl(apiBaseUrl)}
        <div style="margin-top:10px;padding:8px;border:1px solid #f59e0b;background:#fffbeb">
          检测到 <code>http://</code>：密码与 token 在传输层不加密。建议通过 HTTPS 访问。
        </div>
      {/if}
    </div>
  </section>
  {/if}

  {#if token && view === "sessions"}
  <div class="sessions-layout">
  <section class="sessions-list">
    <h2>会话</h2>
    <div style="display:flex;gap:8px;flex-wrap:wrap">
      <button on:click={refreshRuns} disabled={!token}>Refresh all</button>
      <button on:click={refreshRecentSessions} disabled={!token}>Refresh recent</button>
    </div>
    {#if awaitingRuns.length > 0}
      <div style="margin-top:8px;padding:8px;border:1px solid #f59e0b;background:#fffbeb">
        <strong>Needs input:</strong>
        {#each awaitingRuns as r (r.id)}
          <div><code>{r.id}</code> <code>host={r.host_id}</code> <code>tool={r.tool}</code></div>
        {/each}
      </div>
    {/if}
    {#if runs.length === 0}
      <div>No sessions loaded yet.</div>
    {:else}
      <ul>
        {#each runs as r (r.id)}
          <li>
            <button
              on:click={async () => {
                selectedRunId = r.id;
                await loadMessages(r.id);
              }}
              style="margin-right:8px"
            >
              Select
            </button>
            <code>{r.id}</code>
            <strong style={r.status === "awaiting_input" ? "color:#b45309" : ""}>{r.status}</strong>
            <code>host={r.host_id}</code>
            <code>tool={r.tool}</code>
          </li>
        {/each}
      </ul>
    {/if}

    {#if recentSessions.length > 0}
      <div style="margin-top:12px">
        <strong>Recent sessions</strong>
        <ul>
          {#each recentSessions as r (r.id)}
            <li>
              <button
                on:click={async () => {
                  selectedRunId = r.id;
                  await loadMessages(r.id);
                }}
                style="margin-right:8px"
              >
                Select
              </button>
              <code>{r.id}</code>
              <strong style={r.status === "awaiting_input" ? "color:#b45309" : ""}>{r.status}</strong>
              <code>host={r.host_id}</code>
              <code>tool={r.tool}</code>
            </li>
          {/each}
        </ul>
      </div>
    {/if}
  </section>

  <section class="sessions-messages">
    <h2>消息</h2>
    <button on:click={() => selectedRunId && loadMessages(selectedRunId)} disabled={!selectedRunId || !token} style="margin-bottom:8px">
      Refresh messages
    </button>
    <div style="max-height:360px;overflow:auto;border:1px solid #e5e7eb;padding:12px;background:#fafafa">
      {#if displayMessages.length === 0}
        <div>(no messages yet)</div>
      {:else}
        {#each displayMessages as m, i (m.key)}
          <div style="margin:8px 0;display:flex;flex-direction:column;gap:4px">
            <div style="font-size:12px;color:#6b7280">
              <code>{m.ts}</code> <code>{m.role}</code> <code>{m.kind}</code>
              {#if m.request_id}<code> request_id={m.request_id}</code>{/if}
            </div>
            {#if m.kind === "tool.call" && displayMessages[i + 1]?.kind === "tool.result" && displayMessages[i + 1]?.request_id && displayMessages[i + 1]?.request_id === m.request_id}
              {@const callMeta = toolMetaFromText(m.kind, m.text || "")}
              {@const res = displayMessages[i + 1]}
              {@const resMeta = toolMetaFromText(res.kind, res.text || "")}
              <details open style="border:1px solid #e5e7eb;border-radius:10px;padding:10px;background:#f8fafc">
                <summary style="cursor:pointer;display:flex;gap:8px;align-items:center;flex-wrap:wrap">
                  <code>{callMeta.label}</code>
                  {#if m.actor}<code>actor={m.actor}</code>{/if}
                  {#if (res.text || "").includes(" ok=true")}
                    <span style="color:#065f46">ok</span>
                  {:else if (res.text || "").includes(" ok=false")}
                    <span style="color:#b91c1c">error</span>
                  {/if}
                </summary>
                <div style="margin-top:8px">
                  <div style="font-size:12px;color:#6b7280;margin-bottom:6px">call</div>
                  {@html renderMarkdownBasic(callMeta.details || "")}
                  <div style="font-size:12px;color:#6b7280;margin:10px 0 6px 0">result</div>
                  {@html renderMarkdownBasic(resMeta.details || "")}
                </div>
              </details>
            {:else if m.kind === "tool.result" && displayMessages[i - 1]?.kind === "tool.call" && displayMessages[i - 1]?.request_id && displayMessages[i - 1]?.request_id === m.request_id}
              <!-- paired with previous tool.call; skip rendering -->
            {:else if m.role === "assistant"}
              <div style="margin:0;padding:10px;border-radius:10px;border:1px solid #e5e7eb;background:#eff6ff">
                {@html renderMarkdownBasic(m.text || "")}
              </div>
            {:else if m.role === "system"}
              <div style="margin:0;padding:10px;border-radius:10px;border:1px solid #e5e7eb;background:#fff7ed">
                {@html renderMarkdownBasic(m.text || "")}
              </div>
            {:else}
              <div style="margin:0;padding:10px;border-radius:10px;border:1px solid #e5e7eb;background:#ecfdf5">
                {@html renderMarkdownBasic(m.text || "")}
              </div>
            {/if}
          </div>
        {/each}
      {/if}
    </div>
  </section>

  <section class="sessions-input">
    <h2>发送输入</h2>
    <div style="margin:8px 0">
      <strong>Selected:</strong>
      {#if selectedRun}
        <code>{selectedRun.id}</code> <code>{selectedRun.status}</code> <code>{selectedRun.tool}</code>
      {:else}
        <span>none</span>
      {/if}
    </div>
    {#if selectedAwaiting}
      <div style="margin:8px 0;padding:8px;border:1px solid #60a5fa;background:#eff6ff">
        <strong>Awaiting input</strong>
        {#if selectedAwaiting.reason}<code>reason={selectedAwaiting.reason}</code>{/if}
        {#if selectedAwaiting.prompt}
          <div style="margin-top:6px"><code>prompt</code> {selectedAwaiting.prompt}</div>
        {/if}
      </div>
    {/if}
    <div style="display:flex;gap:8px;flex-wrap:wrap;margin-bottom:8px">
      <button on:click={() => sendDecision("approve")} disabled={!selectedRunId || status !== "connected"}>Approve (y)</button>
      <button on:click={() => sendDecision("deny")} disabled={!selectedRunId || status !== "connected"}>Deny (n)</button>
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

  <section class="sessions-output">
    <h2>输出</h2>
    <div style="display:flex;gap:8px;flex-wrap:wrap;margin-bottom:8px">
      <button
        on:click={() => {
          if (!selectedRunId) return;
          outputByRun = { ...outputByRun, [selectedRunId]: "" };
        }}
        disabled={!selectedRunId}
      >
        Clear
      </button>
      <button on:click={refreshRuns} disabled={!token}>Refresh runs</button>
    </div>
    <pre style="white-space:pre-wrap;word-break:break-word;max-height:360px;overflow:auto;border:1px solid #e5e7eb;padding:12px">
{selectedOutput || "(no output yet)"}</pre
    >
  </section>

  <section class:hidden={!token || view !== "tools"}>
    <h2>文件（run cwd）</h2>
    <label>
      Path (relative)
      <input bind:value={filePath} placeholder="README.md" />
    </label>
    <button on:click={fetchFile} disabled={!selectedRunId || status !== "connected"}>Read</button>
    {#if fileError}
      <div style="color:#b91c1c">{fileError}</div>
    {/if}
    <pre style="white-space:pre-wrap;word-break:break-word;max-height:240px;overflow:auto;border:1px solid #e5e7eb;padding:12px">
{fileContent || "(empty)"}</pre
    >
  </section>

  <section class:hidden={!token || view !== "tools"}>
    <h2>搜索（run cwd）</h2>
    <label>
      Query
      <input bind:value={searchQuery} placeholder="TODO" />
    </label>
    <button on:click={searchFiles} disabled={!selectedRunId || status !== "connected"}>Search</button>
    {#if searchError}
      <div style="color:#b91c1c">{searchError}</div>
    {/if}
    {#if searchMatches.length === 0}
      <div>(no matches)</div>
    {:else}
      {#if searchTruncated}
        <div style="color:#92400e">results truncated</div>
      {/if}
      <ul>
        {#each searchMatches as m (m.path + ":" + m.line + ":" + m.column)}
          <li>
            <code>{m.path}:{m.line}:{m.column}</code> {m.text}
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section class:hidden={!token || view !== "hosts"}>
    <h2>主机诊断（WS-RPC）</h2>
    <div style="display:flex;gap:8px;flex-wrap:wrap;margin-bottom:8px">
      <button on:click={refreshHosts} disabled={!token}>Refresh hosts</button>
    </div>
    <label>
      Host ID
      {#if hosts.length > 0}
        <select bind:value={hostDiagHostId} style="width:100%;padding:10px;box-sizing:border-box">
          {#each hosts as h (h.id)}
            <option value={h.id}>{h.id}{h.online ? " (online)" : " (offline)"}</option>
          {/each}
        </select>
      {:else}
        <input bind:value={hostDiagHostId} placeholder="host-dev" />
      {/if}
    </label>
    <div style="display:flex;gap:8px;flex-wrap:wrap;margin:8px 0">
      <button on:click={fetchHostInfo} disabled={status !== "connected"}>host.info</button>
      <button on:click={fetchHostDoctor} disabled={status !== "connected"}>host.doctor</button>
      <button on:click={fetchHostCapabilities} disabled={status !== "connected"}>host.capabilities</button>
    </div>
    <div style="display:flex;gap:8px;flex-wrap:wrap;align-items:flex-end;margin:8px 0">
      <label style="flex:1;min-width:140px">
        lines
        <input bind:value={hostLogsLines} placeholder="200" />
      </label>
      <label style="flex:1;min-width:140px">
        max_bytes
        <input bind:value={hostLogsMaxBytes} placeholder="200000" />
      </label>
      <button on:click={fetchHostLogs} disabled={status !== "connected"}>host.logs.tail</button>
    </div>
    {#if hostDiagError}
      <div style="color:#b91c1c">{hostDiagError}</div>
    {/if}
    {#if hostInfo}
      <h3>info</h3>
      <pre style="white-space:pre-wrap;word-break:break-word;max-height:220px;overflow:auto;border:1px solid #e5e7eb;padding:12px">
{hostInfo}</pre
      >
    {/if}
    {#if hostDoctor}
      <h3>doctor</h3>
      <pre style="white-space:pre-wrap;word-break:break-word;max-height:220px;overflow:auto;border:1px solid #e5e7eb;padding:12px">
{hostDoctor}</pre
      >
    {/if}
    {#if hostCapabilities}
      <h3>capabilities</h3>
      <pre style="white-space:pre-wrap;word-break:break-word;max-height:220px;overflow:auto;border:1px solid #e5e7eb;padding:12px">
{hostCapabilities}</pre
      >
    {/if}
    {#if hostLogs}
      <h3>logs.tail</h3>
      <pre style="white-space:pre-wrap;word-break:break-word;max-height:240px;overflow:auto;border:1px solid #e5e7eb;padding:12px">
{hostLogs}</pre
      >
    {/if}
  </section>

  <section class:hidden={!token || view !== "start"}>
    <h2>启动运行（远程）</h2>
    <button on:click={refreshHosts} disabled={!token} style="margin-bottom:8px">Refresh hosts</button>
    <label>
      Host ID
      {#if hosts.length > 0}
        <select bind:value={startHostId} style="width:100%;padding:10px;box-sizing:border-box">
          {#each hosts as h (h.id)}
            <option value={h.id}>{h.id}{h.online ? " (online)" : " (offline)"}</option>
          {/each}
        </select>
      {:else}
        <input bind:value={startHostId} placeholder="host-dev" />
      {/if}
    </label>
    <label>
      Tool
      <input bind:value={startTool} placeholder="codex" />
    </label>
    <label>
      CWD (optional, host path)
      <input bind:value={startCwd} placeholder="/path/to/project" />
    </label>
    <label>
      Command
      <input bind:value={startCmd} placeholder="echo hi; cat" />
    </label>
    <button on:click={startRun} disabled={status !== "connected"}>Start</button>
    {#if startError}
      <div style="color:#b91c1c">{startError}</div>
    {/if}
  </section>

  <section class:hidden={!token || view !== "tools"}>
    <h2>Git（run cwd）</h2>
    <div style="display:flex;gap:8px;flex-wrap:wrap">
      <button on:click={fetchGitStatus} disabled={!selectedRunId || status !== "connected"}>Status</button>
      <button on:click={fetchGitDiff} disabled={!selectedRunId || status !== "connected"}>Diff</button>
    </div>
    <label>
      Diff path (optional, relative)
      <input bind:value={gitDiffPath} placeholder="src/main.rs" />
    </label>
    {#if gitError}
      <div style="color:#b91c1c">{gitError}</div>
    {/if}
    <h3>status</h3>
    <pre style="white-space:pre-wrap;word-break:break-word;max-height:160px;overflow:auto;border:1px solid #e5e7eb;padding:12px">
{gitStatus || "(empty)"}</pre
    >
    <h3>diff</h3>
    <pre style="white-space:pre-wrap;word-break:break-word;max-height:240px;overflow:auto;border:1px solid #e5e7eb;padding:12px">
{gitDiff || "(empty)"}</pre
    >
  </section>

  <section class="sessions-todo">
    <h2>待办</h2>
    <div style="display:flex;gap:8px;flex-wrap:wrap;align-items:center;margin-bottom:8px">
      <input bind:value={todoText} placeholder="Add a todo..." />
      <button
        on:click={() => {
          addTodo(todoText);
          todoText = "";
        }}
        disabled={!selectedRunId}
      >
        Add
      </button>
    </div>

    {#if todoSuggestions.length > 0}
      <div style="margin:8px 0;padding:8px;border:1px solid #e5e7eb;background:#f8fafc">
        <strong>Suggestions (from output)</strong>
        <ul>
          {#each todoSuggestions as s (s)}
            <li>
              <button on:click={() => addTodo(s)} disabled={!selectedRunId} style="margin-right:8px">Add</button>
              {s}
            </li>
          {/each}
        </ul>
      </div>
    {/if}

    {#if todos.length === 0}
      <div>(no todos)</div>
    {:else}
      <ul>
        {#each todos as t (t.id)}
          <li>
            <label style="display:flex;gap:8px;align-items:center">
              <input type="checkbox" checked={t.done} on:change={() => toggleTodo(t.id)} />
              <span style={t.done ? "text-decoration:line-through;color:#6b7280" : ""}>{t.text}</span>
            </label>
            <button on:click={() => removeTodo(t.id)} style="margin-left:8px">Remove</button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
  </div>
  {/if}

  <section class:hidden={!token || view !== "settings"}>
    <h2>事件</h2>
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
  :global(body) {
    margin: 0;
    font-family: system-ui, -apple-system, BlinkMacSystemFont, "SF Pro Display", "SF Pro Text", Segoe UI, Roboto, sans-serif;
    background: #f2f2f7;
    color: #111827;
  }

  :global(code) {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
    font-size: 12px;
  }

  main {
    max-width: 1040px;
    margin: 0 auto;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .topbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  .subtle {
    font-size: 12px;
    color: #6b7280;
    margin-top: 2px;
  }

  .segmented {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 6px;
    background: rgba(17, 24, 39, 0.06);
    border-radius: 14px;
    padding: 6px;
  }

  .segmented button {
    border: none;
    background: transparent;
    border-radius: 12px;
    padding: 10px 8px;
    font-weight: 800;
    font-size: 13px;
  }

  .segmented button.active {
    background: rgba(255, 255, 255, 0.92);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.06);
    border: 1px solid rgba(17, 24, 39, 0.08);
  }

  .status-pill {
    display: inline-flex;
    align-items: center;
    border-radius: 999px;
    padding: 6px 10px;
    font-size: 12px;
    border: 1px solid rgba(17, 24, 39, 0.1);
    background: rgba(255, 255, 255, 0.85);
  }

  .status-pill[data-kind="connected"] {
    background: rgba(52, 199, 89, 0.12);
    border-color: rgba(52, 199, 89, 0.28);
    color: #065f46;
  }

  .status-pill[data-kind="checking"],
  .status-pill[data-kind="connecting"] {
    background: rgba(0, 122, 255, 0.12);
    border-color: rgba(0, 122, 255, 0.28);
    color: #1d4ed8;
  }

  .status-pill[data-kind="error"] {
    background: rgba(239, 68, 68, 0.1);
    border-color: rgba(239, 68, 68, 0.22);
    color: #991b1b;
  }

  .hidden {
    display: none;
  }

  .sessions-layout {
    display: grid;
    grid-template-columns: 360px 1fr;
    gap: 12px;
    align-items: start;
  }

  @media (max-width: 920px) {
    .sessions-layout {
      grid-template-columns: 1fr;
    }
  }

  .sessions-list {
    grid-column: 1;
    grid-row: 1 / span 4;
  }

  @media (max-width: 920px) {
    .sessions-list {
      grid-row: auto;
    }
  }

  .sessions-messages {
    grid-column: 2;
    grid-row: 1;
  }

  .sessions-input {
    grid-column: 2;
    grid-row: 2;
  }

  .sessions-output {
    grid-column: 2;
    grid-row: 3;
  }

  .sessions-todo {
    grid-column: 2;
    grid-row: 4;
  }

  @media (max-width: 920px) {
    .sessions-messages,
    .sessions-input,
    .sessions-output,
    .sessions-todo {
      grid-column: 1;
      grid-row: auto;
    }
  }

  .sessions-list ul {
    max-height: 520px;
    overflow: auto;
    margin-top: 10px;
    padding-left: 18px;
  }

  h1 {
    margin: 6px 0 2px 0;
    font-weight: 800;
    letter-spacing: 0.2px;
    font-size: 20px;
  }

  h2 {
    margin: 0 0 10px 0;
    font-size: 15px;
    font-weight: 800;
  }

  h3 {
    margin: 12px 0 6px 0;
    font-size: 13px;
    font-weight: 800;
    color: #374151;
  }

  section {
    background: rgba(255, 255, 255, 0.86);
    backdrop-filter: saturate(180%) blur(18px);
    -webkit-backdrop-filter: saturate(180%) blur(18px);
    border: 1px solid rgba(17, 24, 39, 0.08);
    border-radius: 16px;
    padding: 14px;
    box-shadow:
      0 1px 2px rgba(0, 0, 0, 0.04),
      0 10px 30px rgba(0, 0, 0, 0.06);
  }

  label {
    display: block;
    margin: 10px 0;
    font-size: 12px;
    font-weight: 700;
    color: #374151;
  }

  input,
  select {
    width: 100%;
    padding: 10px 12px;
    box-sizing: border-box;
    border-radius: 12px;
    border: 1px solid rgba(17, 24, 39, 0.12);
    background: rgba(255, 255, 255, 0.92);
    font-size: 14px;
    outline: none;
  }

  input:focus,
  select:focus {
    border-color: rgba(0, 122, 255, 0.5);
    box-shadow: 0 0 0 4px rgba(0, 122, 255, 0.18);
  }

  button {
    border: 1px solid rgba(17, 24, 39, 0.12);
    border-radius: 12px;
    padding: 10px 12px;
    background: rgba(255, 255, 255, 0.92);
    font-weight: 700;
    font-size: 13px;
    cursor: pointer;
  }

  button:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  pre {
    white-space: pre-wrap;
    word-break: break-word;
    border-radius: 14px;
    border: 1px solid rgba(17, 24, 39, 0.08);
    background: rgba(255, 255, 255, 0.88);
    padding: 12px;
    overflow: auto;
  }

  ul {
    padding-left: 18px;
  }
</style>
