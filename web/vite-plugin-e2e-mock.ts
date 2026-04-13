/**
 * Vite E2E Mock Plugin — provides same-origin mock API for Playwright tests.
 *
 * When RELAY_E2E_MOCK=1, this plugin intercepts:
 *   GET  /health
 *   POST /auth/login
 *   GET  /hosts
 *   GET  /sessions/recent
 *   GET  /sessions/:id/messages
 *   WS   /ws/app
 *   POST /__test/reset   (test control)
 *
 * Browser only ever talks to :4173, so requests bypass the system proxy entirely.
 */

import type { Plugin, ViteDevServer } from "vite";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface WsClient {
  send(data: string): void;
  close(): void;
}

interface MockScenario {
  health?: object;
  loginStatus?: 200 | 401 | 500;
  loginBody?: object;
  hosts?: object[];
  sessions?: object[];
  messages?: object[];
  /** Messages streamed via WS after subscribe (by run_id). */
  wsStream?: Record<string, object[]>;
}

const DEFAULT_SCENARIO: MockScenario = {
  health: { name: "relay", version: "0.1.0-mock" },
  loginStatus: 200,
  loginBody: { access_token: "mock-token-abc123" },
  hosts: [
    { id: "host-dev", name: "Development", online: true },
    { id: "host-prod", name: "Production", online: false },
  ],
  sessions: [
    {
      id: "run-001",
      host_id: "host-dev",
      tool: "opencode",
      cwd: "/home/ab/projects/relay",
      status: "running",
      started_at: new Date(Date.now() - 120_000).toISOString(),
      last_active_at: new Date(Date.now() - 30_000).toISOString(),
    },
    {
      id: "run-002",
      host_id: "host-dev",
      tool: "opencode",
      cwd: "/home/ab/projects/web",
      status: "exited",
      exit_code: 0,
      started_at: new Date(Date.now() - 600_000).toISOString(),
      last_active_at: new Date(Date.now() - 580_000).toISOString(),
    },
    {
      id: "run-003",
      host_id: "host-dev",
      tool: "opencode",
      cwd: "/tmp",
      status: "awaiting_approval",
      pending_request_id: "req-001",
      pending_reason: "tool",
      pending_op_tool: "fs.write",
      pending_op_args_summary: '{"path":"/etc/passwd"}',
      started_at: new Date(Date.now() - 90_000).toISOString(),
      last_active_at: new Date(Date.now() - 5_000).toISOString(),
    },
  ],
  messages: [
    {
      id: 1,
      seq: 1,
      ts: new Date(Date.now() - 100_000).toISOString(),
      role: "assistant",
      kind: "run.started",
      text: "run started",
    },
    {
      id: 2,
      seq: 2,
      ts: new Date(Date.now() - 95_000).toISOString(),
      role: "assistant",
      kind: "run.output",
      text: "Hello! I'm ready to help.\n",
    },
    {
      id: 3,
      seq: 3,
      ts: new Date(Date.now() - 80_000).toISOString(),
      role: "assistant",
      kind: "tool.call",
      text: 'tool.call fs.read {"path":"package.json"}',
    },
    {
      id: 4,
      ts: new Date(Date.now() - 75_000).toISOString(),
      role: "assistant",
      kind: "tool.result",
      text: 'tool.result fs.read ok=true duration_ms=12 {"name":"relay"}',
    },
    {
      id: 5,
      seq: 5,
      ts: new Date(Date.now() - 50_000).toISOString(),
      role: "assistant",
      kind: "run.output",
      text: "Found the project. Let me continue working on this.\n",
    },
  ],
  wsStream: {
    "run-001": [
      {
        type: "run.output",
        ts: new Date(Date.now() - 20_000).toISOString(),
        run_id: "run-001",
        seq: 10,
        data: { text: "[mock] real-time update from WebSocket\n" },
      },
    ],
    "run-003": [
      {
        type: "run.permission_requested",
        ts: new Date(Date.now() - 5_000).toISOString(),
        run_id: "run-003",
        data: {
          request_id: "req-001",
          reason: "tool",
          op_tool: "fs.write",
          op_args_summary: '{"path":"/etc/passwd"}',
          approve_text: "同意",
          deny_text: "拒绝",
          prompt: "Tool fs.write wants to write to /etc/passwd. Allow?",
        },
      },
    ],
  },
};

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

let currentScenario: MockScenario = structuredClone(DEFAULT_SCENARIO);
const wsClients = new Map<WsClient, { token: string; subscribedRunId: string }>();

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function jsonResponse(data: object, status = 200): Response {
  return new Response(JSON.stringify(data), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

function textResponse(text: string, status = 200): Response {
  return new Response(text, { status, headers: { "Content-Type": "text/plain" } });
}

function wsMessage(type: string, data: object): string {
  return JSON.stringify({ type, ts: new Date().toISOString(), ...data });
}

function broadcastWs(msg: string, runId?: string) {
  for (const [client] of wsClients) {
    if (runId) {
      const sub = wsClients.get(client);
      if (!sub || sub.subscribedRunId !== runId) continue;
    }
    try {
      client.send(msg);
    } catch {
      // client already closed
    }
  }
}

function sendToClient(client: WsClient, msg: string) {
  try {
    client.send(msg);
  } catch {
    // ignore
  }
}

// ---------------------------------------------------------------------------
// HTTP handler
// ---------------------------------------------------------------------------

async function handleHttp(req: Request): Promise<Response | null> {
  const url = new URL(req.url);
  const path = url.pathname;

  // CORS preflight (not needed for same-origin, but good to have)
  if (req.method === "OPTIONS") {
    return new Response(null, {
      status: 204,
      headers: {
        "Access-Control-Allow-Origin": "*",
        "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
        "Access-Control-Allow-Headers": "Content-Type, Authorization",
      },
    });
  }

  // Test control
  if (req.method === "POST" && path === "/__test/reset") {
    const body = await req.json().catch(() => ({}));
    currentScenario = body.scenario
      ? structuredClone(body.scenario as MockScenario)
      : structuredClone(DEFAULT_SCENARIO);
    return jsonResponse({ ok: true });
  }

  if (req.method === "GET" && path === "/health") {
    return jsonResponse(currentScenario.health ?? { name: "relay", version: "mock" });
  }

  if (req.method === "POST" && path === "/auth/login") {
    const body = await req.json().catch(() => ({}));
    if (body.username === "admin" && body.password === "password") {
      return jsonResponse(currentScenario.loginBody ?? { access_token: "mock-token-abc123" });
    }
    return jsonResponse({ error: "invalid credentials" }, 401);
  }

  // Require Bearer token for the rest
  const auth = req.headers.get("Authorization") ?? "";
  if (!auth.startsWith("Bearer ")) {
    return jsonResponse({ error: "unauthorized" }, 401);
  }

  if (req.method === "GET" && path === "/hosts") {
    return jsonResponse(currentScenario.hosts ?? []);
  }

  if (req.method === "GET" && path === "/sessions/recent") {
    return jsonResponse(currentScenario.sessions ?? []);
  }

  // GET /sessions/:id/messages
  const msgMatch = path.match(/^\/sessions\/([^/]+)\/messages$/);
  if (req.method === "GET" && msgMatch) {
    return jsonResponse(currentScenario.messages ?? []);
  }

  return null; // not handled
}

// ---------------------------------------------------------------------------
// WebSocket frame handling
// ---------------------------------------------------------------------------

function handleWsMessage(client: WsClient, raw: string) {
  try {
    const msg = JSON.parse(raw) as Record<string, unknown>;
    const type = String(msg.type ?? "");
    const runId = String(msg.run_id ?? "");

    if (type === "run.subscribe" && runId) {
      const entry = wsClients.get(client);
      if (entry) entry.subscribedRunId = runId;

      // Send buffered stream for this run_id
      const stream = currentScenario.wsStream?.[runId] ?? [];
      for (const event of stream) {
        sendToClient(client, JSON.stringify(event));
      }

      // Send run.ready
      sendToClient(
        client,
        wsMessage("run.ready", { run_id: runId, data: {} }),
      );
    }

    if (type === "run.permission.approve") {
      // Echo back a run.input to simulate approval resolution
      sendToClient(
        client,
        wsMessage("run.input", { run_id: runId, data: { actor: "web", text: "" } }),
      );
    }

    if (type === "run.permission.deny") {
      sendToClient(
        client,
        wsMessage("run.exited", { run_id: runId, data: { exit_code: 1 } }),
      );
    }

    if (type === "run.send_input") {
      const inputId = String((msg.data as Record<string, unknown>)?.input_id ?? "i1");
      sendToClient(
        client,
        wsMessage("run.input", { run_id: runId, data: { input_id: inputId, actor: "web", text: "" } }),
      );
    }

    if (type === "run.send_stdin") {
      const text = String((msg.data as Record<string, unknown>)?.text ?? "");
      sendToClient(
        client,
        wsMessage("run.output", { run_id: runId, seq: 999, data: { text: `[stdin echo] ${text}` } }),
      );
    }

    if (type === "rpc.run.start") {
      const runId = `run-${Date.now()}`;
      sendToClient(
        client,
        wsMessage("rpc.response", {
          data: { request_id: (msg.data as Record<string, unknown>)?.request_id, ok: true, result: { run_id: runId } },
        }),
      );
      sendToClient(
        client,
        wsMessage("run.started", { run_id: runId, host_id: "host-dev", data: { tool: "opencode" } }),
      );
    }

    if (type === "rpc.host.info") {
      const hostId = String((msg.data as Record<string, unknown>)?.host_id ?? "host-dev");
      sendToClient(
        client,
        wsMessage("rpc.response", {
          data: {
            request_id: (msg.data as Record<string, unknown>)?.request_id,
            ok: true,
            result: {
              tools: [
                { tool: "opencode", bin: "opencode", ok: true, models: ["gpt-4o"], default_model: "gpt-4o" },
              ],
            },
          },
        }),
      );
    }
  } catch {
    // ignore malformed JSON
  }
}

// ---------------------------------------------------------------------------
// Vite plugin
// ---------------------------------------------------------------------------

export function e2eMockPlugin(): Plugin {
  return {
    name: "vite-plugin-e2e-mock",

    configureServer(server: ViteDevServer) {
      // Only activate when RELAY_E2E_MOCK=1
      if (process.env.RELAY_E2E_MOCK !== "1") return;

      // ---- HTTP mock via Vite's internal http proxy ----
      // Vite's `server.middlewares.use` adds connect-style middleware.
      server.middlewares.use(async (req, res, next) => {
        // Only mock API paths
        const path = req.url ?? "";
        const shouldMock =
          path.startsWith("/health") ||
          path.startsWith("/auth/") ||
          path.startsWith("/hosts") ||
          path.startsWith("/sessions/") ||
          path.startsWith("/__test/") ||
          path.startsWith("/server/");

        if (!shouldMock) {
          next();
          return;
        }

        // Convert connect IncomingMessage + ServerResponse to a Fetch Request
        const headers: Record<string, string> = {};
        for (const [k, v] of Object.entries(req.headers)) {
          if (typeof v === "string") headers[k] = v;
          else if (Array.isArray(v) && typeof v[0] === "string") headers[k] = v[0];
        }

        const fetchReq = new Request(`http://localhost:${server.config.server.port}${path}`, {
          method: req.method ?? "GET",
          headers,
          body: ["POST", "PUT", "PATCH"].includes(req.method ?? "")
            ? (req as unknown as { body: Buffer }).body
            : undefined,
        });

        const resp = await handleHttp(fetchReq);
        if (!resp) {
          next();
          return;
        }

        res.statusCode = resp.status;
        for (const [k, v] of resp.headers.entries()) {
          res.setHeader(k, v);
        }
        const text = await resp.text();
        res.end(text);
      });

      // ---- WebSocket mock ----
      // Use the WebSocket server if available (vite >= 5)
      const wss = (server as unknown as { ws?: { on: (event: string, cb: (info: { data: string; client: { send: (d: string) => void; close: () => void }; accept: () => void }) => void) => void; handleUpgrade: (req: unknown, socket: unknown, head: unknown, cb: (ws: { send: (d: string) => void; close: () => void; on: (event: string, cb: (data: string) => void) => void }) => void) => void } }).ws;

      if (wss?.on) {
        wss.on("connection", (info: { data: string; client: { send: (d: string) => void; close: () => void }; accept: () => void }, _req: unknown) => {
          const { client, data: handshakeData } = info;
          // Parse token from handshake
          let token = "";
          let subscribedRunId = "";
          try {
            const parsedUrl = new URL(`http://localhost${handshakeData}`);
            token = parsedUrl.searchParams.get("token") ?? "";
          } catch {
            // ignore
          }

          const wsClient: WsClient = {
            send: (d: string) => {
              try {
                (client as { send: (d: string) => void }).send(d);
              } catch {
                // ignore
              }
            },
            close: () => {
              try {
                (client as { close: () => void }).close();
              } catch {
                // ignore
              }
            },
          };

          wsClients.set(wsClient, { token, subscribedRunId });

          wsClient.send(
            JSON.stringify({ type: "connected", ts: new Date().toISOString() }),
          );

          // Store ref to the client's message handler so we can remove it on close
          const msgHandler = (data: string) => handleWsMessage(wsClient, data);
          const closeHandler = () => {
            wsClients.delete(wsClient);
            (client as unknown as { off: (event: string, cb: (data: string) => void) => void }).off("message", msgHandler);
            (client as unknown as { off: (event: string, cb: () => void) => void }).off("close", closeHandler);
          };
          (client as unknown as { on: (event: string, cb: (data: string) => void) => void }).on("message", msgHandler);
          (client as unknown as { on: (event: string, cb: () => void) => void }).on("close", closeHandler);
        });
      }
    },
  };
}
