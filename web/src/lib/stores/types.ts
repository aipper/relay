export type Health = { name: string; version: string };
export type LoginResponse = { access_token: string };

export type RunRow = {
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
};

export type ChatMessage = {
  key: string;
  ts: string;
  role: "user" | "assistant" | "system";
  kind: string;
  actor?: string | null;
  request_id?: string | null;
  text: string;
  data?: unknown;
};

export type ChatMessageApi = {
  id: number;
  seq?: number;
  ts: string;
  role: string;
  kind: string;
  actor?: string | null;
  request_id?: string | null;
  text: string;
  data?: unknown;
};

export type HostInfo = {
  id: string;
  name?: string | null;
  last_seen_at?: string | null;
  online: boolean;
};

export type HostToolStatus = {
  tool: string;
  bin?: string | null;
  ok: boolean;
  error?: string | null;
  models?: string[] | null;
  default_model?: string | null;
  models_error?: string | null;
  models_note?: string | null;
};

export type WsEnvelope = {
  type: string;
  ts: string;
  host_id?: string;
  run_id?: string;
  seq?: number;
  data: unknown;
};

export type TodoItem = {
  id: string;
  text: string;
  done: boolean;
  created_at: string;
};

export type SearchMatch = {
  path: string;
  line: number;
  column: number;
  text: string;
};

export type RiskKind = "read" | "write" | "exec" | "other";

export type AwaitingState = {
  reason?: string;
  prompt?: string;
  request_id?: string;
  op_tool?: string;
  op_args?: unknown;
  op_args_summary?: string;
  approve_text?: string;
  deny_text?: string;
  questions?: unknown;
};

export type OutputMatch = {
  id: string;
  line: number;
  start: number;
  end: number;
};

export type HostGroup = {
  host_id: string;
  host: HostInfo | null;
  display_name: string;
  online: boolean;
  last_seen_at: string | null;
  sessions: RunRow[];
};
