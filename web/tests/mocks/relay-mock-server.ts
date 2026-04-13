/**
 * Bun-based mock relay server for Playwright E2E tests.
 * 
 * Implements the minimal protocol required by web/src/App.svelte:
 * - GET /health
 * - POST /auth/login
 * - GET /hosts
 * - GET /sessions/recent
 * - GET /sessions/:id/messages
 * - WS /ws/app?token=...
 * 
 * Run: bun run tests/mocks/relay-mock-server.ts
 */

import type { ServerWebSocket } from "bun";

// --- Types ---

interface WsEnvelope {
  type: string;
  ts: string;
  host_id?: string;
  run_id?: string;
  seq?: number;
  data: unknown;
}

interface RunRow {
  id: string;
  host_id: string;
  tool: string;
  opencode_session_id?: string | null;
  cwd: string;
  status: string;
  started_at: string;
  last_active_at?: string | null;
  pending_request_id?: string | null;
  pending_reason?: string | null;
  pending_prompt?: string | null;
  pending_op_tool?: string | null;
  pending_op_args_summary?: string | null;
  ended_at?: string | null;
  exit_code?: number | null;
}

interface HostInfo {
  id: string;
  name?: string | null;
  last_seen_at?: string | null;
  online: boolean;
}

interface ChatMessage {
  id: number;
  seq?: number;
  ts: string;
  role: string;
  kind: string;
  actor?: string | null;
  request_id?: string | null;
  text: string;
  data?: unknown;
}

// --- Scenario State ---

interface Scenario {
  authenticated: boolean;
  token: string;
  hosts: HostInfo[];
  runs: RunRow[];
  messages: Record<string, ChatMessage[]>;
  wsClients: Set<ServerWebSocket<{ token: string }>>;
}

const state: Scenario = {
  authenticated: false,
  token: "mock-jwt-token",
  hosts: [
    { id: "host-dev", name: "Dev Machine", last_seen_at: new Date().toISOString(), online: true },
  ],
  runs: [
    {
      id: "run-001",
      host_id: "host-dev",
      tool: "opencode",
      opencode_session_id: "oc-session-001",
      cwd: "/home/user/project",
      status: "running",
      started_at: new Date(Date.now() - 60000).toISOString(),
      last_active_at: new Date().toISOString(),
    },
    {
      id: "run-002",
      host_id: "host-dev",
      tool: "opencode",
      cwd: "/home/user/other",
      status: "awaiting_approval",
      started_at: new Date(Date.now() - 120000).toISOString(),
      last_active_at: new Date().toISOString(),
      pending_request_id: "req-001",
      pending_reason: "tool_permission",
      pending_op_tool: "rpc.fs.write",
      pending_op_args_summary: '{"path":"test.txt","content":"..."}',
    },
  ],
  messages: {
    "run-001": [
      {
        id: 1,
        ts: new Date(Date.now() - 59000).toISOString(),
        role: "assistant",
        kind: "run.output",
        text: "Hello! How can I help you today?",
      },
    ],
    "run-002": [
      {
        id: 1,
        ts: new Date(Date.now() - 119000).toISOString(),
        role: "assistant",
        kind: "run.output",
        text: "I need permission to write a file.",
      },
      {
        id: 2,
        ts: new Date(Date.now() - 118000).toISOString(),
        role: "system",
        kind: "run.permission_requested",
        text: "Permission required",
        request_id: "req-001",
        data: {
          request_id: "req-001",
          op_tool: "rpc.fs.write",
          op_args_summary: '{"path":"test.txt","content":"..."}',
          reason: "tool_permission",
        },
      },
    ],
  },
  wsClients: new Set(),
};

// --- WebSocket helpers ---

function makeEnvelope(type: string, data: unknown, extra: Partial<WsEnvelope> = {}): WsEnvelope {
  return {
    type,
    ts: new Date().toISOString(),
    seq: 1,
    ...extra,
    data,
  };
}

function broadcastToClients(envelope: WsEnvelope, exclude?: ServerWebSocket<{ token: string }>) {
  const msg = JSON.stringify(envelope);
  for (const client of state.wsClients) {
    if (client !== exclude) {
      client.send(msg);
    }
  }
}

function handleWsMessage(ws: ServerWebSocket<{ token: string }>, rawMsg: string) {
  try {
    const msg = JSON.parse(rawMsg) as WsEnvelope;
    console.log("[mock-ws] received:", msg.type);

    if (msg.type === "run.subscribe") {
      const runId = msg.run_id;
      if (!runId) return;

      // Send run.started if exists
      const run = state.runs.find((r) => r.id === runId);
      if (run) {
        ws.send(
          JSON.stringify(
            makeEnvelope("run.started", {
              tool: run.tool,
              cwd: run.cwd,
              opencode_session_id: run.opencode_session_id,
            }, { run_id: runId, host_id: run.host_id })
          )
        );
      }
    }

    if (msg.type === "run.send_input") {
      const runId = msg.run_id;
      if (!runId) return;

      const inputText = (msg.data as { text?: string })?.text ?? "";
      
      // Broadcast to all clients
      broadcastToClients(
        makeEnvelope("run.input", { input_id: `input-${Date.now()}`, text_redacted: inputText }, { run_id: runId }),
        ws
      );

      // Simulate response
      setTimeout(() => {
        ws.send(
          JSON.stringify(
            makeEnvelope("run.output", { text: `Received: ${inputText}\n` }, { run_id: runId })
          )
        );
      }, 100);
    }

    if (msg.type === "run.send_stdin") {
      const runId = msg.run_id;
      if (!runId) return;
      const inputText = (msg.data as { text?: string })?.text ?? "";
      
      broadcastToClients(
        makeEnvelope("run.output", { text: inputText }, { run_id: runId }),
        ws
      );
    }

    if (msg.type === "run.stop") {
      const runId = msg.run_id;
      if (!runId) return;

      const run = state.runs.find((r) => r.id === runId);
      if (run) {
        run.status = "exited";
        run.ended_at = new Date().toISOString();
        run.exit_code = 0;
      }

      broadcastToClients(makeEnvelope("run.exited", { exit_code: 0 }, { run_id: runId }));
    }

    if (msg.type === "run.permission.approve" || msg.type === "run.permission.deny") {
      const runId = msg.run_id;
      if (!runId) return;

      const run = state.runs.find((r) => r.id === runId);
      if (run) {
        run.status = "running";
        run.pending_request_id = null;
        run.pending_reason = null;
        run.pending_prompt = null;
        run.pending_op_tool = null;
        run.pending_op_args_summary = null;
      }

      broadcastToClients(makeEnvelope("run.input", {}, { run_id: runId }));
    }

    if (msg.type === "rpc.run.start") {
      const tool = (msg.data as { tool?: string })?.tool ?? "opencode";
      const cwd = (msg.data as { cwd?: string })?.cwd ?? "/tmp";
      const newRun: RunRow = {
        id: `run-${Date.now()}`,
        host_id: (msg.data as { host_id?: string })?.host_id ?? "host-dev",
        tool,
        cwd,
        status: "running",
        started_at: new Date().toISOString(),
        last_active_at: new Date().toISOString(),
      };
      state.runs.unshift(newRun);
      state.messages[newRun.id] = [];

      ws.send(
        JSON.stringify(
          makeEnvelope("rpc.response", {
            ok: true,
            result: { run_id: newRun.id },
            request_id: (msg.data as { request_id?: string })?.request_id,
          })
        )
      );

      broadcastToClients(
        makeEnvelope("run.started", { tool, cwd }, { run_id: newRun.id, host_id: newRun.host_id })
      );
    }

    if (msg.type === "rpc.host.info") {
      const hostId = (msg.data as { host_id?: string })?.host_id ?? "host-dev";
      ws.send(
        JSON.stringify(
          makeEnvelope("rpc.response", {
            ok: true,
            result: {
              tools: [
                {
                  tool: "opencode",
                  ok: true,
                  models: ["gpt-4o", "claude-3-5-sonnet"],
                  default_model: "gpt-4o",
                },
              ],
            },
            request_id: (msg.data as { request_id?: string })?.request_id,
          })
        )
      );
    }

  } catch (err) {
    console.error("[mock-ws] parse error:", err);
  }
}

// --- HTTP Handlers ---

const server = Bun.serve({
  port: 8787,
  fetch(req, server) {
    const url = new URL(req.url);
    const path = url.pathname;

    // WebSocket upgrade
    if (path === "/ws/app") {
      const token = url.searchParams.get("token");
      if (!token) {
        return new Response("Unauthorized", { status: 401 });
      }
      const success = server.upgrade(req, { data: { token } });
      if (success) return;
      return new Response("WebSocket upgrade failed", { status: 500 });
    }

    // --- HTTP Endpoints ---

    if (path === "/health") {
      return Response.json({ name: "relay-mock", version: "0.1.0" });
    }

    if (path === "/auth/login" && req.method === "POST") {
      return req.json().then((body) => {
        const username = body?.username;
        const password = body?.password;

        if (username === "admin" && password === "123456") {
          state.authenticated = true;
          return Response.json({ access_token: state.token });
        }

        return Response.json({ error: "Invalid credentials" }, { status: 401 });
      });
    }

    if (path === "/hosts" && req.method === "GET") {
      const auth = req.headers.get("Authorization");
      if (!auth?.startsWith("Bearer ")) {
        return Response.json({ error: "Unauthorized" }, { status: 401 });
      }
      return Response.json(state.hosts);
    }

    if ((path === "/sessions" || path === "/sessions/recent") && req.method === "GET") {
      const auth = req.headers.get("Authorization");
      if (!auth?.startsWith("Bearer ")) {
        return Response.json({ error: "Unauthorized" }, { status: 401 });
      }
      const limit = parseInt(url.searchParams.get("limit") ?? "200", 10);
      return Response.json(state.runs.slice(0, limit));
    }

    if (path.startsWith("/sessions/") && path.endsWith("/messages") && req.method === "GET") {
      const auth = req.headers.get("Authorization");
      if (!auth?.startsWith("Bearer ")) {
        return Response.json({ error: "Unauthorized" }, { status: 401 });
      }

      const runId = path.split("/")[2];
      const messages = state.messages[runId ?? ""] ?? [];
      return Response.json(messages);
    }

    // Test control endpoint: reset state
    if (path === "/__test/reset" && req.method === "POST") {
      // Reset to initial state
      state.authenticated = false;
      state.token = "mock-jwt-token";
      state.hosts = [
        { id: "host-dev", name: "Dev Machine", last_seen_at: new Date().toISOString(), online: true },
      ];
      state.runs = [
        {
          id: "run-001",
          host_id: "host-dev",
          tool: "opencode",
          opencode_session_id: "oc-session-001",
          cwd: "/home/user/project",
          status: "running",
          started_at: new Date(Date.now() - 60000).toISOString(),
          last_active_at: new Date().toISOString(),
        },
      ];
      state.messages = {
        "run-001": [
          {
            id: 1,
            ts: new Date(Date.now() - 59000).toISOString(),
            role: "assistant",
            kind: "run.output",
            text: "Hello! How can I help you today?",
          },
        ],
      };

      // Disconnect all WS clients
      for (const client of state.wsClients) {
        client.close();
      }
      state.wsClients.clear();

      return Response.json({ ok: true });
    }

    return Response.json({ error: "Not found" }, { status: 404 });
  },

  websocket: {
    open(ws: ServerWebSocket<{ token: string }>) {
      console.log("[mock-ws] client connected");
      state.wsClients.add(ws);

      // Send initial runs list
      for (const run of state.runs) {
        ws.send(
          JSON.stringify(
            makeEnvelope("run.started", {
              tool: run.tool,
              cwd: run.cwd,
              opencode_session_id: run.opencode_session_id,
            }, { run_id: run.id, host_id: run.host_id })
          )
        );
      }
    },

    message(ws: ServerWebSocket<{ token: string }>, msg: string | Buffer) {
      handleWsMessage(ws, msg.toString());
    },

    close(ws: ServerWebSocket<{ token: string }>) {
      console.log("[mock-ws] client disconnected");
      state.wsClients.delete(ws);
    },
  },
});

console.log(`[mock-server] listening on http://127.0.0.1:${server.port}`);
