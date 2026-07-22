import type { WsEnvelope, HostToolStatus, RunRow, ChatMessage, OutputMatch } from "./types";

let _counter = 0;
export function uid(): string {
  _counter++;
  return `${Date.now().toString(36)}-${_counter.toString(36)}-${Math.random().toString(36).slice(2, 6)}`;
}

export function inferDefaultApiBaseUrl(): string {
  if (typeof window === "undefined") return "http://127.0.0.1:8787";
  const host = window.location?.hostname || "127.0.0.1";
  const proto = window.location?.protocol === "https:" ? "https" : "http";
  if (window.location?.port === "8787") return window.location.origin;
  return `${proto}://${host}:8787`;
}

export function toWsBase(url: string) {
  return url.replace(/^http:/, "ws:").replace(/^https:/, "wss:").replace(/\/$/, "");
}

export async function fetchWithTimeout(url: string, init: RequestInit = {}, timeoutMs = 15_000): Promise<Response> {
  if (typeof AbortController === "undefined" || init.signal) return await fetch(url, init);
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    return await fetch(url, { ...init, signal: controller.signal });
  } catch (e) {
    const name = e && typeof e === "object" && "name" in e ? String((e as { name?: unknown }).name) : "";
    if (name === "AbortError") {
      throw new Error(`请求超时（${timeoutMs}ms）：${url}`);
    }
    throw e;
  } finally {
    clearTimeout(timer);
  }
}

export function isRecord(v: unknown): v is Record<string, unknown> {
  return Boolean(v) && typeof v === "object";
}

export function dataString(e: WsEnvelope, key: string): string | undefined {
  if (!isRecord(e.data)) return undefined;
  const v = e.data[key];
  return typeof v === "string" ? v : undefined;
}

export function dataBool(e: WsEnvelope, key: string): boolean | undefined {
  if (!isRecord(e.data)) return undefined;
  const v = e.data[key];
  return typeof v === "boolean" ? v : undefined;
}

export function dataAny(e: WsEnvelope, key: string): unknown {
  if (!isRecord(e.data)) return undefined;
  return e.data[key];
}

export function isProbablyInsecureUrl(url: string) {
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

export function truncateTail(s: string, maxChars: number) {
  if (s.length <= maxChars) return s;
  return s.slice(s.length - maxChars);
}

export function truncateHead<T>(arr: T[], maxLen: number): T[] {
  if (arr.length <= maxLen) return arr;
  return arr.slice(arr.length - maxLen);
}

export function parseHostToolStatuses(value: unknown): HostToolStatus[] {
  if (!Array.isArray(value)) return [];
  return value
    .map((item) => (isRecord(item) ? item : null))
    .filter(Boolean)
    .map((item) => ({
      tool: typeof item!.tool === "string" ? item!.tool : "",
      bin: typeof item!.bin === "string" ? item!.bin : null,
      ok: Boolean(item!.ok),
      error: typeof item!.error === "string" ? item!.error : null,
      models: Array.isArray(item!.models)
        ? item!.models.filter((v): v is string => typeof v === "string" && Boolean(v.trim()))
        : null,
      default_model: typeof item!.default_model === "string" ? item!.default_model : null,
      models_error: typeof item!.models_error === "string" ? item!.models_error : null,
      models_note: typeof item!.models_note === "string" ? item!.models_note : null,
    }))
    .filter((item) => Boolean(item.tool));
}

export function sanitizeTerminalOutput(input: string): string {
  const s0 = input ?? "";
  if (!s0) return "";
  if (!s0.includes("\x1b") && !s0.includes("\r")) return s0;
  let s = s0.replace(/\r\n/g, "\n");
  if (s.includes("\x1b")) {
    s = s.replace(/\x1b\[[0-?]*[ -/]*[@-~]/g, "");
    s = s.replace(/\x1b\][^\x07]*(?:\x07|\x1b\\)/g, "");
    s = s.replace(/\x1bP[\s\S]*?\x1b\\/g, "");
    s = s.replace(/\x1b[@-Z\\-_]/g, "");
  }
  s = s.replace(/[\x00-\x07\x0B\x0C\x0E-\x1F\x7F]/g, "");
  return s;
}

export function applyTerminalEdits(existing: string, chunk: string): string {
  if (!chunk) return existing;
  if (!chunk.includes("\r") && !chunk.includes("\b")) return `${existing}${chunk}`;
  let out = existing;
  let lineStart = out.lastIndexOf("\n") + 1;
  for (let i = 0; i < chunk.length; i++) {
    const ch = chunk[i]!;
    if (ch === "\n") { out += "\n"; lineStart = out.length; continue; }
    if (ch === "\r") { out = out.slice(0, lineStart); continue; }
    if (ch === "\b") { if (out.length > lineStart) out = out.slice(0, out.length - 1); continue; }
    out += ch;
  }
  return out;
}

export function compareTsDesc(a?: string | null, b?: string | null): number {
  const ta = a ? Date.parse(a) : 0;
  const tb = b ? Date.parse(b) : 0;
  return tb - ta;
}

export function formatRelativeTime(ts?: string | null): string {
  if (!ts) return "";
  const t = Date.parse(ts);
  if (!Number.isFinite(t)) return "";
  const now = Date.now();
  const diffMs = Math.max(0, now - t);
  const sec = Math.floor(diffMs / 1000);
  if (sec < 10) return "刚刚";
  if (sec < 60) return `${sec}秒前`;
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min}分钟前`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr}小时前`;
  const day = Math.floor(hr / 24);
  if (day < 7) return `${day}天前`;
  return new Date(t).toLocaleDateString();
}

export function formatAbsTime(ts: string): string {
  const t = Date.parse(ts);
  if (!Number.isFinite(t)) return ts;
  return new Date(t).toLocaleString();
}

export function statusLabel(r: RunRow): { label: string; kind: "running" | "warning" | "error" | "done" } {
  if (r.status === "awaiting_approval") {
    const reason = (r.pending_reason ?? "").trim().toLowerCase();
    if (reason === "prompt") return { label: "待输入", kind: "warning" };
    return { label: "待审批", kind: "warning" };
  }
  if (r.status === "awaiting_input") return { label: "待输入", kind: "warning" };
  if (r.status === "running") return { label: "运行中", kind: "running" };
  if (r.status === "exited") {
    if (typeof r.exit_code === "number" && r.exit_code !== 0) return { label: "错误", kind: "error" };
    return { label: "已结束", kind: "done" };
  }
  return { label: r.status, kind: "done" };
}

export function connLabel(s: string): string {
  if (s === "connected") return "已连接";
  if (s === "checking") return "检查中";
  if (s === "connecting") return "连接中";
  if (s === "disconnected") return "未连接";
  if (s === "error") return "错误";
  return s;
}

export function sessionTitle(r: RunRow): string {
  return basename(r.cwd) || "";
}

export function basename(path: string): string {
  const trimmed = path.trim();
  if (!trimmed || trimmed === "." || trimmed === "/") return "";
  const parts = trimmed.split(/[\\/]+/g).filter(Boolean);
  return parts[parts.length - 1] ?? "";
}

export function sessionSummary(r: RunRow): string {
  const s = (r.cwd || "").trim();
  if (!s || s === ".") return "";
  return s.length > 60 ? `${s.slice(0, 60)}…` : s;
}

export function looksLikeTuiAnsi(s: string): boolean {
  if (!s) return false;
  if (!s.includes("\x1b[")) return false;
  if (/\x1b\[[0-?]*[ -/]*[HJfKABCDGd]/.test(s)) return true;
  if (/\x1b\[\?[0-9;]*[hl]/.test(s)) return true;
  return false;
}

export function isLikelyTuiToolName(tool: string): boolean {
  return tool === "codex" || tool === "gemini";
}

export function codexStructuredEventText(raw: string): string | null {
  if (!raw || raw[0] !== "{" || !raw.includes('"method":"codex/event"')) return null;
  try {
    const v: unknown = JSON.parse(raw);
    if (!isRecord(v) || v["method"] !== "codex/event") return null;
    const params = v["params"];
    if (!isRecord(params)) return null;
    const msg = params["msg"];
    if (!isRecord(msg)) return null;
    const t = typeof msg["type"] === "string" ? msg["type"] : "";
    const message = typeof msg["message"] === "string" ? msg["message"] : "";
    const details = typeof msg["additional_details"] === "string" ? msg["additional_details"] : "";
    const pieces: string[] = [];
    if (t) pieces.push(`[codex:${t}]`);
    if (message) pieces.push(message);
    if (details) pieces.push(details);
    const out = pieces.join(" ").trim();
    return out ? `${out}\n` : null;
  } catch { return null; }
}

export function computeOutputMatches(lines: string[], q: string, runId: string): OutputMatch[] {
  const query = q.trim().toLowerCase();
  if (!query) return [];
  const safeRunId = runId.replace(/[^a-zA-Z0-9_-]/g, "_");
  const matches: OutputMatch[] = [];
  let matchIndex = 0;
  for (let lineIndex = 0; lineIndex < lines.length; lineIndex++) {
    const line = lines[lineIndex] ?? "";
    const lower = line.toLowerCase();
    let from = 0;
    while (from <= lower.length) {
      const idx = lower.indexOf(query, from);
      if (idx === -1) break;
      matches.push({ id: `out-match-${safeRunId}-${matchIndex}`, line: lineIndex, start: idx, end: idx + query.length });
      matchIndex++;
      from = idx + Math.max(1, query.length);
    }
  }
  return matches;
}

export function renderOutputHtml(lines: string[], matches: OutputMatch[], cursor: number): string {
  if (matches.length === 0) return lines.join("\n").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  const byLine = new Map<number, OutputMatch[]>();
  for (const m of matches) {
    const arr = byLine.get(m.line) ?? [];
    arr.push(m);
    byLine.set(m.line, arr);
  }
  let out = "";
  for (let lineIndex = 0; lineIndex < lines.length; lineIndex++) {
    const line = lines[lineIndex] ?? "";
    const lineMatches = byLine.get(lineIndex) ?? [];
    if (lineMatches.length === 0) { out += esc(line); if (lineIndex !== lines.length - 1) out += "\n"; continue; }
    let cursorPos = 0;
    for (const m of lineMatches) {
      out += esc(line.slice(cursorPos, m.start));
      const cls = matches[cursor]?.id === m.id ? "out-mark current" : "out-mark";
      out += `<mark id="${m.id}" class="${cls}">${esc(line.slice(m.start, m.end))}</mark>`;
      cursorPos = m.end;
    }
    out += esc(line.slice(cursorPos));
    if (lineIndex !== lines.length - 1) out += "\n";
  }
  return out;
}

function esc(s: string): string { return s.replace(/</g, "&lt;").replace(/>/g, "&gt;"); }
