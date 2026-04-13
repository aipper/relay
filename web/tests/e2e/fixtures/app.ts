import { test as base, expect, type Page } from "@playwright/test";
export { expect };

export interface TestFixtures {
  page: Page;
}

export async function resetMockState() {}

export async function setScenario(_baseUrl: string, _scenario: object) {}

export const test = base.extend<TestFixtures>({
  page: async ({ browser }, use) => {
    const context = await browser.newContext({ ignoreHTTPSErrors: true });
    const page = await context.newPage();

    await page.addInitScript(async () => {
      const MOCK_TOKEN = "mock-token-abc123";
      const DEFAULT_HOSTS = [
        { id: "host-dev", name: "Development", online: true },
        { id: "host-prod", name: "Production", online: false },
      ];
      const DEFAULT_SESSIONS = [
        { id: "run-001", host_id: "host-dev", tool: "opencode", cwd: "/home/ab/projects/relay", status: "running", started_at: new Date(Date.now() - 120_000).toISOString(), last_active_at: new Date(Date.now() - 30_000).toISOString() },
        { id: "run-002", host_id: "host-dev", tool: "opencode", cwd: "/home/ab/projects/web", status: "exited", exit_code: 0, started_at: new Date(Date.now() - 600_000).toISOString(), last_active_at: new Date(Date.now() - 580_000).toISOString() },
        { id: "run-003", host_id: "host-dev", tool: "opencode", cwd: "/tmp", status: "awaiting_approval", pending_request_id: "req-001", pending_reason: "tool", pending_op_tool: "fs.write", pending_op_args_summary: '{"path":"/etc/passwd"}', started_at: new Date(Date.now() - 90_000).toISOString(), last_active_at: new Date(Date.now() - 5_000).toISOString() },
      ];
      const DEFAULT_MESSAGES = [
        { id: 1, seq: 1, ts: new Date(Date.now() - 100_000).toISOString(), role: "assistant", kind: "run.started", text: "run started" },
        { id: 2, seq: 2, ts: new Date(Date.now() - 95_000).toISOString(), role: "assistant", kind: "run.output", text: "Hello! I'm ready to help.\n" },
        { id: 3, seq: 3, ts: new Date(Date.now() - 80_000).toISOString(), role: "assistant", kind: "tool.call", text: 'tool.call fs.read {"path":"package.json"}' },
        { id: 4, ts: new Date(Date.now() - 75_000).toISOString(), role: "assistant", kind: "tool.result", text: 'tool.result fs.read ok=true duration_ms=12 {"name":"relay"}' },
        { id: 5, seq: 5, ts: new Date(Date.now() - 50_000).toISOString(), role: "assistant", kind: "run.output", text: "Found the project. Let me continue working on this.\n" },
      ];
      const WS_STREAM: Record<string, object[]> = {
        "run-001": [{ type: "run.output", ts: new Date(Date.now() - 20_000).toISOString(), run_id: "run-001", seq: 10, data: { text: "[mock] real-time update from WebSocket\n" } }],
        "run-003": [{ type: "run.permission_requested", ts: new Date(Date.now() - 5_000).toISOString(), run_id: "run-003", data: { request_id: "req-001", reason: "tool", op_tool: "fs.write", op_args_summary: '{"path":"/etc/passwd"}', approve_text: "同意", deny_text: "拒绝", prompt: "Tool fs.write wants to write to /etc/passwd. Allow?" } }],
      };

      const originalFetch = window.fetch.bind(window);
      (window as unknown as Record<string, unknown>).WebSocket = class extends EventTarget {
        url: string;
        readyState: number = 0;
        CONNECTING = 0;
        OPEN = 1;
        CLOSING = 2;
        CLOSED = 3;
        private _timeoutId: ReturnType<typeof setTimeout> | null = null;

        constructor(url: string, _protocols?: string | string[]) {
          super();
          this.url = url;
          const path = (() => { try { return new URL(url, "http://localhost").pathname; } catch { return url; } })();
          if (path.startsWith("/ws/")) {
            this.readyState = 1;
            this._timeoutId = setTimeout(() => {
              this.dispatchEvent(new Event("open"));
              this.dispatchEvent(new MessageEvent("message", { data: JSON.stringify({ type: "connected", ts: new Date().toISOString() }) }));
            }, 10);
          } else {
            this.readyState = 1;
            this._timeoutId = setTimeout(() => { this.dispatchEvent(new Event("open")); }, 10);
          }
        }

        set onopen(fn: ((this: unknown, ev: Event) => void) | null) { if (fn) this.addEventListener("open", fn as EventListener); }
        set onmessage(fn: ((this: unknown, ev: MessageEvent) => void) | null) { if (fn) this.addEventListener("message", fn as EventListener); }
        set onclose(fn: ((this: unknown, ev: CloseEvent) => void) | null) { if (fn) this.addEventListener("close", fn as EventListener); }
        set onerror(fn: ((this: unknown, ev: Event) => void) | null) { if (fn) this.addEventListener("error", fn as EventListener); }

        send(data: string) {
          if (this.readyState !== 1) return;
          try {
            const msg = JSON.parse(data);
            if (msg.type === "run.subscribe" && msg.run_id) {
              const stream = WS_STREAM[msg.run_id] ?? [];
              stream.forEach((event, i) => { setTimeout(() => this.dispatchEvent(new MessageEvent("message", { data: JSON.stringify(event) })), 50 + i * 50); });
              setTimeout(() => this.dispatchEvent(new MessageEvent("message", { data: JSON.stringify({ type: "run.ready", ts: new Date().toISOString(), run_id: msg.run_id, data: {} }) })), 100);
            }
            if (msg.type === "run.permission.approve") {
              this.dispatchEvent(new MessageEvent("message", { data: JSON.stringify({ type: "run.input", ts: new Date().toISOString(), run_id: msg.run_id, data: { actor: "web", text: "" } }) }));
            }
            if (msg.type === "run.permission.deny") {
              this.dispatchEvent(new MessageEvent("message", { data: JSON.stringify({ type: "run.input", ts: new Date().toISOString(), run_id: msg.run_id, data: { actor: "web", text: "" } }) }));
              setTimeout(() => this.dispatchEvent(new MessageEvent("message", { data: JSON.stringify({ type: "run.exited", ts: new Date().toISOString(), run_id: msg.run_id, data: { exit_code: 1 } }) })), 100);
            }
            if (msg.type === "run.send_input") {
              this.dispatchEvent(new MessageEvent("message", { data: JSON.stringify({ type: "run.input", ts: new Date().toISOString(), run_id: msg.run_id, data: { input_id: msg.data?.input_id ?? "i1", actor: "web", text: msg.data?.text ?? "" } }) }));
            }
            if (msg.type === "rpc.run.start") {
              const runId = "run-" + Date.now();
              this.dispatchEvent(new MessageEvent("message", { data: JSON.stringify({ type: "rpc.response", ts: new Date().toISOString(), data: { request_id: msg.data?.request_id, ok: true, result: { run_id: runId } } }) }));
              this.dispatchEvent(new MessageEvent("message", { data: JSON.stringify({ type: "run.started", ts: new Date().toISOString(), run_id: runId, host_id: "host-dev", data: { tool: msg.data?.tool ?? "opencode" } }) }));
            }
            if (msg.type === "rpc.host.info") {
              this.dispatchEvent(new MessageEvent("message", { data: JSON.stringify({ type: "rpc.response", ts: new Date().toISOString(), data: { request_id: msg.data?.request_id ?? "req", ok: true, result: { tools: [{ tool: "opencode", bin: "opencode", ok: true, models: ["gpt-4o"], default_model: "gpt-4o" }] } } }) }));
            }
          } catch { /* ignore */ }
        }

        close() {
          this.readyState = 3;
          if (this._timeoutId) clearTimeout(this._timeoutId);
          this.dispatchEvent(new CloseEvent("close", { code: 1000, reason: "normal", wasClean: true }));
        }
      } as unknown as typeof WebSocket;

      window.fetch = async function(input: RequestInfo | URL, init?: RequestInit): Promise<Response> {
        const url = typeof input === "string" ? input : input instanceof URL ? input.href : (input as Request).url;
        const method = init?.method ?? "GET";
        const path = (() => { try { return new URL(url, "http://localhost").pathname; } catch { return url; } })();
        const headers = init?.headers as Record<string, string> | undefined;
        const bearer = headers?.["Authorization"] ?? "";

        if (path === "/health" && method === "GET") {
          return new Response(JSON.stringify({ name: "relay", version: "0.1.0-mock" }), { status: 200, headers: { "Content-Type": "application/json" } });
        }
        if (path === "/auth/login" && method === "POST") {
          let body: Record<string, unknown> = {};
          try { if (init?.body) body = JSON.parse(init.body as string); } catch { /* ignore */ }
          if (body.username === "admin" && body.password === "password") {
            return new Response(JSON.stringify({ access_token: MOCK_TOKEN }), { status: 200, headers: { "Content-Type": "application/json" } });
          }
          return new Response("invalid credentials", { status: 401, headers: { "Content-Type": "text/plain" } });
        }
        if (!bearer.startsWith("Bearer ") && !bearer.startsWith("bearer ")) {
          return new Response("unauthorized", { status: 401, headers: { "Content-Type": "text/plain" } });
        }
        if (path === "/hosts" && method === "GET") {
          return new Response(JSON.stringify(DEFAULT_HOSTS), { status: 200, headers: { "Content-Type": "application/json" } });
        }
        if (path.startsWith("/sessions/recent") && method === "GET") {
          return new Response(JSON.stringify(DEFAULT_SESSIONS), { status: 200, headers: { "Content-Type": "application/json" } });
        }
        const msgMatch = path.match(/^\/sessions\/([^/]+)\/messages$/);
        if (msgMatch && method === "GET") {
          return new Response(JSON.stringify(DEFAULT_MESSAGES), { status: 200, headers: { "Content-Type": "application/json" } });
        }
        return originalFetch(input, init);
      } as typeof fetch;

      window.dispatchEvent(new CustomEvent("e2e-mock-ready"));
    });

    await page.goto("/");
    await page.waitForLoadState("domcontentloaded");
    await use(page);
    await context.close();
  },
});
