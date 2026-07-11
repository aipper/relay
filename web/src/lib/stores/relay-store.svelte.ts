import {
  type Health, type LoginResponse, type RunRow,
  type ChatMessage, type ChatMessageApi, type HostInfo,
  type HostToolStatus, type WsEnvelope, type TodoItem,
  type SearchMatch, type OutputMatch,
} from "./types";
import {
  fetchWithTimeout, isRecord, dataString, dataBool, dataAny,
  toWsBase, isProbablyInsecureUrl, parseHostToolStatuses,
  sanitizeTerminalOutput, applyTerminalEdits,
  truncateTail, truncateHead, inferDefaultApiBaseUrl,
  uid, compareTsDesc,
} from "./utils";
import { reduceToBlocks } from "../blocks/reduce";
import type { UiBlock } from "../blocks/types";

const START_CWD_STORAGE_KEY = "relay.startCwdByHost.v1";
const DEFAULT_SESSION_LIMIT = 200;

class RelayStore {
  defaultApiBaseUrl = $state(inferDefaultApiBaseUrl());
  apiBaseUrl = $state(inferDefaultApiBaseUrl());
  useCustomServer = $state(false);
  customBaseUrl = $state("");
  username = $state("admin");
  password = $state("");
  token = $state("");
  keepSignedIn = $state(true);
  rememberPassword = $state(false);
  status = $state("disconnected");
  loginBusy = $state(false);
  view = $state("sessions");
  health = $state<Health | null>(null);
  events = $state<WsEnvelope[]>([]);
  runs = $state<RunRow[]>([]);
  hosts = $state<HostInfo[]>([]);
  selectedRunId = $state("");
  messagesByRun = $state<Record<string, ChatMessage[]>>({});
  awaitingByRun = $state<Record<string, any | undefined>>({});
  outputByRun = $state<Record<string, string>>({});
  runReadyByRun = $state<Record<string, boolean>>({});
  lastError = $state("");
  toastText = $state("");
  sessionDetailTab = $state<"messages" | "output">("messages");
  outputAutoScroll = $state(true);
  outputIsAtBottom = $state(true);
  outputBufferLines = $state(400);
  outputSearchText = $state("");
  outputSearchActive = $state("");
  outputSearchCursor = $state(0);
  outputSearchMatches = $state<OutputMatch[]>([]);
  outputHtml = $state("");
  isMobile = $state(false);
  sessionSearch = $state("");
  hostGroupCollapsed = $state<Record<string, boolean>>({});
  approvalModalOpen = $state(false);
  approvalModalShowArgs = $state(false);
  approvalForSession = $state(false);
  approvalAnswersJson = $state("");
  inputModalOpen = $state(false);
  inputModalText = $state("");
  stopConfirmOpen = $state(false);
  chatInputText = $state("");
  startHostId = $state("host-dev");
  startTool = $state("opencode");
  startCwd = $state("");
  startCmd = $state("");
  startError = $state("");
  startCwdByHost = $state<Record<string, string>>({});
  startOpencodeModel = $state("");
  startOpencodeSessionId = $state("");
  startHostToolsById = $state<Record<string, HostToolStatus[] | undefined>>({});
  startHostToolsLoadingById = $state<Record<string, boolean | undefined>>({});
  filePath = $state("README.md");
  fileContent = $state("");
  fileError = $state("");
  searchQuery = $state("TODO");
  searchMatches = $state<SearchMatch[]>([]);
  searchTruncated = $state(false);
  searchError = $state("");
  gitDiffPath = $state("");
  gitStatus = $state("");
  gitDiff = $state("");
  gitError = $state("");
  hostDiagHostId = $state("host-dev");
  hostInfo = $state("");
  hostDoctor = $state("");
  hostCapabilities = $state("");
  hostLogs = $state("");
  hostLogsLines = $state("200");
  hostLogsMaxBytes = $state("200000");
  hostDiagError = $state("");
  serverLogsPath = $state("");
  serverLogs = $state("");
  serverLogsLines = $state("200");
  serverLogsMaxBytes = $state("200000");
  serverLogsTruncated = $state(false);
  serverLogsError = $state("");
  todos = $state<TodoItem[]>([]);
  todoText = $state("");
  todoSuggestions = $state<string[]>([]);
  inlineAwaitingText = $state("");
  outputSearchInputEl: HTMLInputElement | null = $state(null);
  #recentSessions: RunRow[] = [];

  uiBlocks: UiBlock[] = $derived.by(() => {
    const msgs = this.messagesByRun[this.selectedRunId ?? ""] ?? [];
    return reduceToBlocks(msgs, { runTool: this.runToolFor(this.selectedRunId ?? ""), outputMode: "log" });
  });
  selectedRun = $derived(this.runs.find((r) => r.id === this.selectedRunId) ?? null);
  selectedRunReady = $derived(this.selectedRunId ? (this.runReadyByRun[this.selectedRunId] ?? true) : true);
  selectedOutput = $derived(this.selectedRunId ? this.outputByRun[this.selectedRunId] ?? "" : "");
  selectedAwaiting = $derived(this.selectedRunId ? this.awaitingByRun[this.selectedRunId] ?? null : null);
  selectedMessages = $derived(this.selectedRunId ? this.messagesByRun[this.selectedRunId] ?? [] : []);
  hostsById = $derived(Object.fromEntries(this.hosts.map((h) => [h.id, h] as const)) as Record<string, HostInfo>);
  currentStartToolOptions = $derived(this.dynamicStartToolOptions());
  currentStartToolStatuses = $derived(this.startHostToolsById[this.startHostId] ?? null);
  currentStartHostToolsLoading = $derived(Boolean(this.startHostToolsLoadingById[this.startHostId]));
  currentStartOpencodeModels = $derived(this.currentOpencodeToolStatus()?.models?.slice() ?? []);
  currentStartOpencodeDefaultModel = $derived(this.currentOpencodeToolStatus()?.default_model?.trim() ?? "");
  currentStartOpencodeModelsError = $derived(this.currentOpencodeToolStatus()?.models_error?.trim() ?? "");
  currentStartOpencodeModelsNote = $derived(this.currentOpencodeToolStatus()?.models_note?.trim() ?? "");
  isProbablyInsecureUrl = $derived(isProbablyInsecureUrl(this.apiBaseUrl));

  filteredRuns = $derived.by(() => {
    const q = this.sessionSearch.trim().toLowerCase();
    if (!q) return this.runs;
    return this.runs.filter((r) => {
      const title = this.sessionTitle(r);
      const summary = this.cwdShort(r);
      return (title || summary ? `${title} ${summary}` : r.id).toLowerCase().includes(q);
    });
  });
  runGroups = $derived.by(() => {
    const map = new Map<string, RunRow[]>();
    for (const r of this.filteredRuns) {
      const arr = map.get(r.host_id) ?? [];
      arr.push(r);
      map.set(r.host_id, arr);
    }
    const groups = Array.from(map.entries()).map(([hostId, items]) => {
      const h = this.hostsById[hostId] ?? null;
      items.sort((a, b) => compareTsDesc(a.last_active_at ?? a.started_at, b.last_active_at ?? b.started_at));
      return { host_id: hostId, host: h, display_name: this.hostDisplayName(h, hostId), online: Boolean(h?.online), last_seen_at: h?.last_seen_at ?? null, sessions: items };
    });
    groups.sort((a, b) => {
      if (a.online !== b.online) return a.online ? -1 : 1;
      return a.display_name.localeCompare(b.display_name);
    });
    return groups;
  });

  #ws: WebSocket | null = null;
  #wsQueue: WsEnvelope[] = [];
  #wsQueuePos = 0;
  #wsFlushScheduled = false;
  #wsSubscribedRunId = "";
  #pendingRpc = new Map<string, (msg: WsEnvelope) => void>();
  #toastTimer: ReturnType<typeof setTimeout> | null = null;
  #todoSuggestionsTimer: ReturnType<typeof setTimeout> | null = null;
  #lastSeenApprovalRequest: Record<string, string> = {};
  #lastSeenPromptRequest: Record<string, string> = {};
  #approvalDraftKey = "";
  #awaitingDraftKey = "";
  #runsIndex: Map<string, number> | null = null;
  #lastSeenAwaitingKey = "";
  #stdinBuf = "";
  #stdinBufRunId = "";
  #stdinBufTimer: ReturnType<typeof setTimeout> | null = null;
  #mobileTabResetRunId = "";
  #lastStartToolsForceRefreshKey = "";
  #lastSuggestedStartCwd = "";

  constructor() {
    try {
      const savedBaseUrl = localStorage.getItem("relay.baseUrl") ?? "";
      const savedUseCustom = localStorage.getItem("relay.useCustomServer") === "1";
      this.customBaseUrl = savedBaseUrl;
      this.useCustomServer = savedUseCustom || Boolean(savedBaseUrl);

      const parsed = this.#loadPersistedAuth();
      if (parsed) {
        if (typeof parsed.keepSignedIn === "boolean") this.keepSignedIn = parsed.keepSignedIn;
        if (typeof parsed.rememberPassword === "boolean") this.rememberPassword = parsed.rememberPassword;
        if (typeof parsed.username === "string" && parsed.username.trim()) this.username = parsed.username;
        if (typeof parsed.token === "string" && parsed.token.trim() && this.keepSignedIn) this.token = parsed.token;
        if (typeof parsed.password === "string" && this.rememberPassword) this.password = parsed.password;
      }

      const hgc = localStorage.getItem("relay.hostGroupCollapsed.v1");
      if (hgc) { try { this.hostGroupCollapsed = JSON.parse(hgc) as Record<string, boolean>; } catch {} }

      const cwdRaw = localStorage.getItem(START_CWD_STORAGE_KEY);
      if (cwdRaw) { try { this.startCwdByHost = JSON.parse(cwdRaw) as Record<string, string>; } catch {} }
    } catch (e) {
      console.warn("RelayStore constructor (non-fatal):", e);
    }
  }

  #loadPersistedAuth(): Record<string, unknown> | null {
    try {
      const raw = localStorage.getItem("relay.auth.v1");
      if (!raw) return null;
      const parsed = JSON.parse(raw) as unknown;
      if (parsed && typeof parsed === "object") return parsed as Record<string, unknown>;
    } catch {}
    return null;
  }

  setCustomBaseUrl(url: string) { this.customBaseUrl = url; }
  setCredentials(user: string, pass: string) { this.username = user; this.password = pass; }
  navigate(v: string) { this.view = v; }

  persistServerPrefs() {
    try {
      localStorage.setItem("relay.useCustomServer", this.useCustomServer ? "1" : "0");
      if (this.customBaseUrl.trim()) localStorage.setItem("relay.baseUrl", this.customBaseUrl.trim());
      else localStorage.removeItem("relay.baseUrl");
    } catch {}
  }

  persistAuthPrefs() {
    try {
      localStorage.setItem("relay.auth.v1", JSON.stringify({
        username: this.username.trim(), keepSignedIn: this.keepSignedIn,
        rememberPassword: this.rememberPassword,
        ...(this.keepSignedIn && this.token ? { token: this.token } : {}),
        ...(this.rememberPassword && this.password ? { password: this.password } : {}),
      }));
    } catch {}
  }

  setToast(text: string) {
    this.toastText = text;
    if (this.#toastTimer) clearTimeout(this.#toastTimer);
    this.#toastTimer = setTimeout(() => { this.toastText = ""; this.#toastTimer = null; }, 1500);
  }

  #resetWsState() {
    this.events = []; this.outputByRun = {}; this.runReadyByRun = {};
    this.#wsSubscribedRunId = ""; this.awaitingByRun = {}; this.hosts = [];
    if (this.#ws) { try { this.#ws.close(); } catch {} this.#ws = null; }
  }

  #openAppWebSocket(nextToken: string) {
    this.status = "connecting";
    const nextWs = new WebSocket(`${toWsBase(this.apiBaseUrl)}/ws/app?token=${encodeURIComponent(nextToken)}`);
    this.#ws = nextWs;
    nextWs.onopen = () => {
      if (this.#ws === nextWs) {
        this.status = "connected";
        if (this.#wsSubscribedRunId || this.selectedRunId) this.#subscribeToRun(this.#wsSubscribedRunId || this.selectedRunId);
      }
    };
    nextWs.onclose = () => { if (this.#ws === nextWs) this.status = "disconnected"; };
    nextWs.onerror = () => { if (this.#ws === nextWs) this.status = "error"; };
    nextWs.onmessage = (ev) => {
      try {
        if (this.#ws !== nextWs) return;
        const msg = JSON.parse(ev.data) as WsEnvelope;
        if (msg.type === "run.output") { if (!this.selectedRunId || msg.run_id !== this.selectedRunId) return; }
        const last = this.#wsQueue.length > 0 ? this.#wsQueue[this.#wsQueue.length - 1] : null;
        if (msg.type === "run.output" && last && last.type === "run.output" && last.run_id === msg.run_id && isRecord(last.data) && typeof last.data["text"] === "string") {
          last.data["text"] = `${String(last.data["text"])}${dataString(msg, "text") ?? ""}`;
          this.#scheduleWsFlush(); return;
        }
        this.#wsQueue.push(msg);
        this.#scheduleWsFlush();
      } catch {}
    };
  }

  #ensureRunsIndex() {
    if (this.#runsIndex) return this.#runsIndex;
    this.#runsIndex = new Map();
    for (let i = 0; i < this.runs.length; i++) { const r = this.runs[i]; if (r?.id) this.#runsIndex.set(r.id, i); }
    return this.#runsIndex;
  }

  #scheduleWsFlush() {
    if (this.#wsFlushScheduled) return;
    this.#wsFlushScheduled = true;
    requestAnimationFrame(() => this.#flushWsQueue());
  }

  #flushWsQueue() {
    this.#wsFlushScheduled = false;
    if (this.#wsQueue.length === 0) { this.#wsQueuePos = 0; return; }
    if (this.#wsQueuePos > 1000) { this.#wsQueue = this.#wsQueue.slice(this.#wsQueuePos); this.#wsQueuePos = 0; }
    const started = performance.now();
    const selectedId = this.selectedRunId;
    let nextRuns = this.runs; this.#runsIndex = null;
    let runsChanged = false;
    let nextAwaiting = this.awaitingByRun; let awaitingChanged = false;
    let nextOutputByRun = this.outputByRun; let outputChanged = false;
    let nextRunReadyByRun = this.runReadyByRun; let readyChanged = false;
    const newMessages: ChatMessage[] = [];

    while (this.#wsQueuePos < this.#wsQueue.length) {
      const msg = this.#wsQueue[this.#wsQueuePos]!; this.#wsQueuePos++;
      if (msg.type === "rpc.response") {
        const reqId = dataString(msg, "request_id");
        if (reqId) { const cb = this.#pendingRpc.get(reqId); if (cb) { this.#pendingRpc.delete(reqId); cb(msg); } }
      }
      if (msg.run_id && msg.type === "run.ready") {
        if (!readyChanged) { nextRunReadyByRun = { ...nextRunReadyByRun }; readyChanged = true; }
        nextRunReadyByRun[msg.run_id] = true;
      }
      if (msg.run_id && msg.type === "run.output") {
        const runId = msg.run_id;
        const raw = dataString(msg, "text") ?? "";
        const text = sanitizeTerminalOutput(raw);
        if (text && runId === selectedId) {
          if (!outputChanged) { nextOutputByRun = { ...nextOutputByRun }; outputChanged = true; }
          const cur = nextOutputByRun[selectedId] ?? "";
          nextOutputByRun[selectedId] = truncateTail(applyTerminalEdits(cur, text), 200_000);
        }
      }
      if (msg.run_id) {
        const last_active_at = msg.ts;
        const opencodeSessionId = dataString(msg, "opencode_session_id");
        if (msg.type === "run.started") {
          if (!runsChanged) { nextRuns = [...nextRuns]; runsChanged = true; }
          nextRuns.unshift({
            id: msg.run_id, host_id: msg.host_id ?? "unknown", tool: dataString(msg, "tool") ?? "unknown",
            opencode_session_id: opencodeSessionId, cwd: dataString(msg, "cwd") ?? ".", status: "running",
            started_at: msg.ts, last_active_at: msg.ts,
          });
          if (!readyChanged) { nextRunReadyByRun = { ...nextRunReadyByRun }; readyChanged = true; }
          nextRunReadyByRun[msg.run_id] = false;
          this.status = "running";
        } else if (msg.type === "run.exited") {
          if (!readyChanged) { nextRunReadyByRun = { ...nextRunReadyByRun }; readyChanged = true; }
          nextRunReadyByRun[msg.run_id] = false;
          const exit_code = isRecord(msg.data) && typeof msg.data["exit_code"] === "number" ? msg.data["exit_code"] : undefined;
          this.#runsUpdateInPlace(nextRuns, msg.run_id, (cur) => ({ ...cur, last_active_at, status: "exited", ended_at: msg.ts, exit_code: typeof exit_code === "number" ? exit_code : cur.exit_code }));
        } else if (msg.type === "run.awaiting_input" || msg.type === "run.permission_requested") {
          if (!awaitingChanged) { nextAwaiting = { ...nextAwaiting }; awaitingChanged = true; }
          nextAwaiting[msg.run_id] = {
            reason: dataString(msg, "reason"), prompt: dataString(msg, "prompt"),
            request_id: dataString(msg, "request_id"), op_tool: dataString(msg, "op_tool"),
            op_args: dataAny(msg, "op_args"), op_args_summary: dataString(msg, "op_args_summary"),
            approve_text: dataString(msg, "approve_text"), deny_text: dataString(msg, "deny_text"),
            questions: dataAny(msg, "questions"),
          };
        } else if (msg.type === "run.input") {
          this.#runsUpdateInPlace(nextRuns, msg.run_id, (cur) => ({ ...cur, last_active_at, status: "running" }));
          if (!awaitingChanged) { nextAwaiting = { ...nextAwaiting }; awaitingChanged = true; }
          nextAwaiting[msg.run_id] = undefined;
        }
        if (opencodeSessionId) this.#runsUpdateInPlace(nextRuns, msg.run_id, (cur) => ({ ...cur, opencode_session_id: opencodeSessionId }));
      }
      if (this.selectedRunId && msg.run_id === this.selectedRunId && this.view === "sessions") {
        const m = this.#envToMessage(msg);
        if (m) newMessages.push(m);
      }
      if (performance.now() - started > 10) break;
    }
    if (outputChanged) this.outputByRun = nextOutputByRun;
    if (awaitingChanged) this.awaitingByRun = nextAwaiting;
    if (runsChanged) this.runs = nextRuns;
    if (readyChanged) this.runReadyByRun = nextRunReadyByRun;
    if (newMessages.length > 0 && selectedId) {
      const existing = this.messagesByRun[selectedId] ?? [];
      this.messagesByRun = { ...this.messagesByRun, [selectedId]: truncateHead([...existing, ...newMessages], 1000) };
    }
    if (this.#wsQueuePos >= this.#wsQueue.length) { this.#wsQueue = []; this.#wsQueuePos = 0; return; }
    this.#scheduleWsFlush();
  }

  #runsUpdateInPlace(nextRuns: any, runId: string, updater: any) {
    const idx = this.#ensureRunsIndex().get(runId);
    if (idx !== undefined) nextRuns[idx] = updater(nextRuns[idx]!);
  }

  #envToMessage(env: any): ChatMessage | null {
    if (!env.run_id) return null;
    if (env.type === "run.output") {
      return { key: `${env.ts}:run.output:${env.seq ?? uid()}`, ts: env.ts, role: "assistant", kind: env.type, actor: dataString(env, "actor"), text: sanitizeTerminalOutput(dataString(env, "text") ?? ""), data: env.data };
    }
    if (env.type === "run.input") {
      return { key: `${env.ts}:run.input:${dataString(env, "input_id") ?? uid()}`, ts: env.ts, role: "user", kind: env.type, actor: dataString(env, "actor"), text: dataString(env, "text_redacted") ?? "", data: env.data };
    }
    if (env.type === "run.permission_requested") {
      return { key: `${env.ts}:run.permission_requested:${dataString(env, "request_id") ?? uid()}`, ts: env.ts, role: "system", kind: env.type, request_id: dataString(env, "request_id"), text: dataString(env, "prompt") ?? "", data: env.data };
    }
    if (env.type === "run.started" || env.type === "run.exited") {
      return { key: `${env.ts}:${env.type}:${env.seq ?? uid()}`, ts: env.ts, role: "system", kind: env.type, text: env.type === "run.started" ? "run started" : "run exited", data: env.data };
    }
    return null;
  }

  #sendWs(env: any) {
    if (this.#ws && this.#ws.readyState === WebSocket.OPEN) this.#ws.send(JSON.stringify(env));
  }

  #subscribeToRun(runId: string) {
    this.#wsSubscribedRunId = runId;
    if (!runId) return;
    this.#sendWs({ type: "run.subscribe", ts: new Date().toISOString(), run_id: runId, data: { replace: true, include_output: true } });
  }

  subscribeToRun(runId: string) { this.#subscribeToRun(runId); }

  async #rpcCall(rpcType: string, data: any): Promise<any> {
    if (!this.#ws || this.#ws.readyState !== WebSocket.OPEN) throw new Error("ws not connected");
    const requestId = uid();
    const msg: any = { type: rpcType, ts: new Date().toISOString(), data: { ...data, request_id: requestId } };
    if (this.selectedRunId) msg.run_id = this.selectedRunId;
    const p = new Promise<any>((resolve) => { this.#pendingRpc.set(requestId, resolve); });
    this.#sendWs(msg);
    return await p;
  }

  async connect() {
    this.lastError = ""; this.loginBusy = true;
    try {
      this.#resetWsState(); this.status = "checking";
      this.apiBaseUrl = (this.useCustomServer && this.customBaseUrl.trim()) ? this.customBaseUrl.trim() : this.defaultApiBaseUrl;
      const h = await fetchWithTimeout(`${this.apiBaseUrl.replace(/\/$/, "")}/health`, {}, 10_000);
      if (!h.ok) { const b = await h.text().catch(() => ""); throw new Error(`health failed: ${h.status} ${b}`.trim()); }
      this.health = (await h.json()) as Health;
      const l = await fetchWithTimeout(`${this.apiBaseUrl.replace(/\/$/, "")}/auth/login`, {
        method: "POST", headers: { "content-type": "application/json" },
        body: JSON.stringify({ username: this.username, password: this.password }),
      }, 15_000);
      if (!l.ok) {
        const b = await l.text().catch(() => "");
        const hint = b.includes("bad password hash") || l.status === 500 ? "（服务端 ADMIN_PASSWORD_HASH 配置无效）" : "";
        throw new Error(`login failed: ${l.status} ${b} ${hint}`.trim());
      }
      const login = (await l.json()) as LoginResponse;
      this.token = login.access_token; this.view = "sessions";
      this.persistServerPrefs(); this.persistAuthPrefs();
      this.#openAppWebSocket(this.token);
      void Promise.all([this.refreshHosts(), this.refreshRuns()]);
    } catch (e) {
      this.lastError = `${e instanceof Error ? e.message : String(e)}\nserver=${this.apiBaseUrl}`.trim();
      this.status = "error";
    } finally { this.loginBusy = false; }
  }

  async resumeFromStoredToken() {
    if (!this.token) return; this.lastError = "";
    const savedToken = this.token;
    try {
      this.#resetWsState();
      const h = await fetchWithTimeout(`${this.apiBaseUrl.replace(/\/$/, "")}/health`, {}, 10_000);
      if (!h.ok) { const b = await h.text().catch(() => ""); throw new Error(`health failed: ${h.status} ${b}`.trim()); }
      this.health = (await h.json()) as Health; this.view = "sessions";
      this.#openAppWebSocket(savedToken);
      void Promise.all([this.refreshHosts(), this.refreshRuns()]);
    } catch (e) {
      this.lastError = `${e instanceof Error ? e.message : String(e)}\nserver=${this.apiBaseUrl}`.trim();
      this.status = "error";
    }
  }

  disconnect() {
    if (this.#ws) { this.#ws.close(); this.#ws = null; }
    this.token = ""; this.status = "disconnected"; this.persistAuthPrefs();
  }

  async refreshHosts() {
    if (!this.token) return;
    try {
      const r = await fetchWithTimeout(`${this.apiBaseUrl.replace(/\/$/, "")}/hosts`, { headers: { Authorization: `Bearer ${this.token}` } }, 20_000);
      if (r.status === 401) { this.setToast("登录已过期"); this.disconnect(); return; }
      if (r.ok) {
        this.hosts = (await r.json()) as HostInfo[];
        const online = this.hosts.filter((h) => h.online);
        if (online.length > 0 && !online.some((h) => h.id === this.startHostId)) this.startHostId = online[0].id;
      }
    } catch (e) { console.warn("refreshHosts failed", e); }
  }

  async refreshRuns() {
    if (!this.token) return;
    try {
      const r = await fetchWithTimeout(`${this.apiBaseUrl.replace(/\/$/, "")}/sessions/recent?limit=${DEFAULT_SESSION_LIMIT}`, { headers: { Authorization: `Bearer ${this.token}` } }, 20_000);
      if (r.status === 401) { this.setToast("登录已过期"); this.disconnect(); return; }
      if (r.ok) {
        this.runs = (await r.json()) as RunRow[];
        if (this.selectedRunId && !this.runs.some((x) => x.id === this.selectedRunId)) this.selectedRunId = "";
      }
    } catch (e) { console.warn("refreshRuns failed", e); }
  }

  async loadMessages(runId: string) {
    if (!this.token) return;
    try {
      const r = await fetchWithTimeout(`${this.apiBaseUrl.replace(/\/$/, "")}/sessions/${encodeURIComponent(runId)}/messages?limit=200`, { headers: { Authorization: `Bearer ${this.token}` } }, 20_000);
      if (r.status === 401) { this.disconnect(); return; }
      if (!r.ok) return;
      const msgs = (await r.json()) as ChatMessageApi[];
      this.messagesByRun = { ...this.messagesByRun, [runId]: msgs.map((m) => ({
        key: String(m.id), ts: m.ts, role: m.role === "assistant" || m.role === "user" ? m.role : "system",
        kind: m.kind, actor: m.actor, request_id: m.request_id,
        text: m.kind === "run.output" ? sanitizeTerminalOutput(m.text) : m.text, data: m.data,
      })) };
    } catch (e) { console.warn("loadMessages failed", e); }
  }

  async selectSession(runId: string) {
    this.selectedRunId = runId; this.#subscribeToRun(runId);
    this.sessionDetailTab = "messages"; this.outputAutoScroll = true;
    if (!this.messagesByRun[runId]) await this.loadMessages(runId);
  }

  runToolFor(runId: string): string { return this.runs.find((x) => x.id === runId)?.tool ?? ""; }

  sendChatInput(raw?: string) {
    if (!this.selectedRunId || this.status !== "connected") return;
    const text = (raw ?? this.chatInputText).trim();
    if (!text) return;
    this.chatInputText = "";
    const a = this.selectedAwaiting;
    if (a) {
      if (a.request_id || a.op_tool) {
        if (text.toLowerCase() === "y" || text.toLowerCase() === "yes" || text.toLowerCase() === "approve") { this.sendDecision("approve"); return; }
        if (text.toLowerCase() === "n" || text.toLowerCase() === "no" || text.toLowerCase() === "deny") { this.sendDecision("deny"); return; }
      }
      if (a.reason === "prompt" && !a.request_id && !a.op_tool) {
        this.#sendInput(text + "\n"); return;
      }
    }
    this.#sendInput(text + "\n");
  }

  #sendInput(text: string) {
    if (!this.selectedRunId) return;
    this.#sendWs({ type: "run.send_input", ts: new Date().toISOString(), run_id: this.selectedRunId, data: { input_id: uid(), actor: "web", text: text.replace(/\r\n/g, "\r").replace(/\n/g, "\r") } });
  }

  sendDecision(decision: string) {
    if (!this.selectedRunId) return;
    const a = this.selectedAwaiting; const reqId = a?.request_id;
    if (!reqId) { this.#sendInput(decision === "approve" ? "y\n" : "n\n"); return; }
    const data: Record<string, unknown> = { request_id: reqId, actor: "web" };
    if (decision === "approve" && this.approvalForSession) data["decision"] = "approve_for_session";
    this.#sendWs({ type: decision === "approve" ? "run.permission.approve" : "run.permission.deny", ts: new Date().toISOString(), run_id: this.selectedRunId, data });
  }

  sendStop(signal = "term") {
    if (!this.selectedRunId) return;
    this.#sendWs({ type: "run.stop", ts: new Date().toISOString(), run_id: this.selectedRunId, data: { signal } });
  }

  hostDisplayName(h: any, hostId: string): string { return (h?.name ?? "").trim() || hostId; }

  toggleHostGroup(hostId: string) {
    this.hostGroupCollapsed = { ...this.hostGroupCollapsed, [hostId]: !this.hostGroupCollapsed[hostId] };
    localStorage.setItem("relay.hostGroupCollapsed.v1", JSON.stringify(this.hostGroupCollapsed));
  }

  suggestedStartCwdForHost(hostId: string): string {
    const saved = this.startCwdByHost[hostId]?.trim();
    if (saved) return saved;
    const recent = this.runs.find((run) => run.host_id === hostId && run.cwd?.trim());
    return recent?.cwd?.trim() ?? "";
  }

  applySuggestedStartCwd(force = false) {
    const suggested = this.suggestedStartCwdForHost(this.startHostId);
    if (force || !this.startCwd.trim() || this.startCwd === this.#lastSuggestedStartCwd) this.startCwd = suggested;
    this.#lastSuggestedStartCwd = suggested;
  }

  #rememberStartCwd(hostId: string, cwd: string) {
    const normalized = cwd.trim();
    if (!hostId.trim() || !normalized) return;
    this.startCwdByHost = { ...this.startCwdByHost, [hostId.trim()]: normalized };
    localStorage.setItem(START_CWD_STORAGE_KEY, JSON.stringify(this.startCwdByHost));
  }

  currentOpencodeToolStatus(): any {
    const statuses = this.startHostToolsById[this.startHostId] ?? null;
    return statuses?.find((item: any) => item.tool === "opencode") ?? null;
  }

  dynamicStartToolOptions(): Array<{ value: string; label: string }> {
    const statuses = this.startHostToolsById[this.startHostId];
    if (!statuses) return [{ value: "opencode", label: "opencode" }];
    return statuses.filter((item: any) => item.ok).map((item: any) => ({ value: item.tool, label: item.tool }));
  }

  async ensureStartHostTools(hostId: string, force = false) {
    const normalized = hostId.trim();
    if (!normalized || !this.token || this.status !== "connected") return;
    if (!force && normalized in this.startHostToolsById) return;
    this.startHostToolsLoadingById = { ...this.startHostToolsLoadingById, [normalized]: true };
    try {
      const resp = await this.#rpcCall("rpc.host.info", { host_id: normalized });
      const ok = dataBool(resp, "ok");
      if (!ok) throw new Error(dataString(resp, "error") ?? "rpc failed");
      const result = dataAny(resp, "result");
      const tools = parseHostToolStatuses(isRecord(result) ? result["tools"] : []);
      this.startHostToolsById = { ...this.startHostToolsById, [normalized]: tools };
    } catch { if (!(normalized in this.startHostToolsById)) this.startHostToolsById = { ...this.startHostToolsById, [normalized]: [] };
    } finally { this.startHostToolsLoadingById = { ...this.startHostToolsLoadingById, [normalized]: false }; }
  }

  async startRun() {
    this.startError = "";
    try {
      const data: Record<string, unknown> = { host_id: this.startHostId.trim(), tool: this.startTool.trim(), cmd: this.startCmd.trim(), cwd: this.startCwd.trim() || null };
      if (this.startTool === "opencode" && this.startOpencodeModel.trim()) data.model = this.startOpencodeModel.trim();
      if (this.startTool === "opencode" && this.startOpencodeSessionId.trim()) data.opencode_session_id = this.startOpencodeSessionId.trim();
      const resp = await this.#rpcCall("rpc.run.start", data);
      const ok = dataBool(resp, "ok");
      if (!ok) throw new Error(dataString(resp, "error") ?? "rpc failed");
      if (this.startCwd.trim()) this.#rememberStartCwd(this.startHostId, this.startCwd);
      await this.refreshRuns(); this.view = "sessions";
      await this.selectSession(resp.run_id ?? "");
      this.setToast(`已启动 ${this.startTool}`);
    } catch (e) { this.startError = e instanceof Error ? e.message : String(e); }
  }

  async fetchFile() {
    this.fileError = ""; this.fileContent = "";
    try {
      const resp = await this.#rpcCall("rpc.fs.read", { path: this.filePath });
      const ok = dataBool(resp, "ok");
      if (!ok) throw new Error(dataString(resp, "error") ?? "rpc failed");
      const result = dataAny(resp, "result");
      this.fileContent = String(isRecord(result) ? (result.content ?? "") : "");
    } catch (e) { this.fileError = e instanceof Error ? e.message : String(e); }
  }

  async searchFiles() {
    this.searchError = ""; this.searchMatches = []; this.searchTruncated = false;
    try {
      const resp = await this.#rpcCall("rpc.fs.search", { q: this.searchQuery });
      const ok = dataBool(resp, "ok");
      if (!ok) throw new Error(dataString(resp, "error") ?? "rpc failed");
      const result = dataAny(resp, "result");
      if (!isRecord(result)) throw new Error("bad rpc result");
      const matches = result.matches;
      if (!Array.isArray(matches)) throw new Error("bad rpc matches");
      this.searchMatches = matches.filter((m: any): m is SearchMatch => isRecord(m) && Boolean(m.path));
    } catch (e) { this.searchError = e instanceof Error ? e.message : String(e); }
  }

  async fetchGitStatus() { this.gitError = ""; this.gitStatus = "";
    try {
      const resp = await this.#rpcCall("rpc.git.status", {});
      const ok = dataBool(resp, "ok"); if (!ok) throw new Error(dataString(resp, "error") ?? "rpc failed");
      const result = dataAny(resp, "result");
      this.gitStatus = String(isRecord(result) ? (result.stdout ?? "") : "");
    } catch (e) { this.gitError = e instanceof Error ? e.message : String(e); }
  }

  async fetchGitDiff() { this.gitError = ""; this.gitDiff = "";
    try {
      const data: Record<string, unknown> = {};
      if (this.gitDiffPath.trim()) data.path = this.gitDiffPath.trim();
      const resp = await this.#rpcCall("rpc.git.diff", data);
      const ok = dataBool(resp, "ok"); if (!ok) throw new Error(dataString(resp, "error") ?? "rpc failed");
      const result = dataAny(resp, "result");
      this.gitDiff = String(isRecord(result) ? (result.stdout ?? "") : "");
    } catch (e) { this.gitError = e instanceof Error ? e.message : String(e); }
  }

  queueStdin(text: string) {
    const runId = this.selectedRunId; if (!runId) return;
    if (this.#stdinBufRunId && this.#stdinBufRunId !== runId) { this.#stdinBuf = ""; if (this.#stdinBufTimer) { clearTimeout(this.#stdinBufTimer); this.#stdinBufTimer = null; } }
    this.#stdinBufRunId = runId; this.#stdinBuf += text ?? "";
    if (this.#stdinBuf.length >= 4096) { this.#flushStdinBuf(); return; }
    if (!this.#stdinBufTimer) this.#stdinBufTimer = setTimeout(() => this.#flushStdinBuf(), 20);
  }

  #flushStdinBuf() {
    if (this.#stdinBufTimer) { clearTimeout(this.#stdinBufTimer); this.#stdinBufTimer = null; }
    const runId = this.#stdinBufRunId; const chunk = this.#stdinBuf;
    this.#stdinBufRunId = ""; this.#stdinBuf = "";
    if (!chunk || !runId || runId !== this.selectedRunId) return;
    this.#sendWs({ type: "run.send_stdin", ts: new Date().toISOString(), run_id: runId, data: { actor: "web", text: chunk.replace(/\r\n/g, "\r").replace(/\n/g, "\r") } });
  }

  handleChatInputKeydown(ev: KeyboardEvent) { ev.preventDefault(); this.sendChatInput(); }
  handleOutputScroll() {}
  async refreshSelectedSession() { await Promise.all([this.refreshHosts(), this.refreshRuns()]); if (this.selectedRunId) await this.loadMessages(this.selectedRunId); }
  openInputModal(prefill = "") { this.inputModalText = prefill; this.inputModalOpen = true; }
  closeInputModal() { this.inputModalOpen = false; this.inputModalText = ""; }
  sendQuickInput(text: string) { this.#sendInput(text); this.closeInputModal(); }
  runOutputSearch() { this.outputSearchActive = this.outputSearchText.trim(); this.outputSearchCursor = 0; }
  nextOutputMatch() { if (this.outputSearchMatches.length) this.outputSearchCursor = (this.outputSearchCursor + 1) % this.outputSearchMatches.length; }
  prevOutputMatch() { if (this.outputSearchMatches.length) this.outputSearchCursor = (this.outputSearchCursor - 1 + this.outputSearchMatches.length) % this.outputSearchMatches.length; }
  clearOutputSearch() { this.outputSearchText = ""; this.outputSearchActive = ""; this.outputSearchCursor = 0; }
  toggleOutputAutoScroll() { this.outputAutoScroll = !this.outputAutoScroll; }
  jumpToLatest() { this.outputAutoScroll = true; }
  resumeOutputAutoScroll() { this.outputAutoScroll = true; }
  focusOutputSearch() { this.outputSearchInputEl?.focus(); }
  handleOutputSearchKeydown(ev: KeyboardEvent) {
    if (ev.key === "Enter") { ev.preventDefault(); this.runOutputSearch(); }
    else if (ev.key === "ArrowDown") { ev.preventDefault(); this.nextOutputMatch(); }
    else if (ev.key === "ArrowUp") { ev.preventDefault(); this.prevOutputMatch(); }
  }
  async copyOutput() { try { await navigator.clipboard.writeText(this.selectedOutput || ""); this.setToast("已复制"); } catch { this.setToast("复制失败"); } }

  sendInputModalText() {
    let t = this.inputModalText;
    if (t && !t.includes("\n") && !t.endsWith("\r")) t += "\n";
    this.#sendInput(t); this.closeInputModal();
  }

  includeOutputInMessages(runId: string): boolean { return true; }

  async fetchHostInfo() {
    try {
      const resp = await this.#rpcCall("rpc.host.info", { host_id: this.hostDiagHostId });
      this.hostInfo = dataBool(resp, "ok") ? JSON.stringify(dataAny(resp, "result") ?? null, null, 2) : "";
    } catch (e: any) { this.hostDiagError = e.message; }
  }
  async fetchHostDoctor() {
    try {
      const resp = await this.#rpcCall("rpc.host.doctor", { host_id: this.hostDiagHostId });
      this.hostDoctor = dataBool(resp, "ok") ? JSON.stringify(dataAny(resp, "result") ?? null, null, 2) : "";
    } catch (e: any) { this.hostDiagError = e.message; }
  }
  async fetchHostCapabilities() {
    try {
      const resp = await this.#rpcCall("rpc.host.capabilities", { host_id: this.hostDiagHostId });
      this.hostCapabilities = dataBool(resp, "ok") ? JSON.stringify(dataAny(resp, "result") ?? null, null, 2) : "";
    } catch (e: any) { this.hostDiagError = e.message; }
  }
  async fetchHostLogs() {
    try {
      const resp = await this.#rpcCall("rpc.host.logs.tail", { host_id: this.hostDiagHostId, lines: 200, max_bytes: 200000 });
      this.hostLogs = dataBool(resp, "ok") ? JSON.stringify(dataAny(resp, "result") ?? null, null, 2) : "";
    } catch (e: any) { this.hostDiagError = e.message; }
  }
  async fetchServerLogs() {
    this.serverLogsError = ""; if (!this.token) return;
    try {
      const r = await fetchWithTimeout(`${this.apiBaseUrl.replace(/\/$/, "")}/server/logs/tail?lines=200&max_bytes=200000`, { headers: { Authorization: `Bearer ${this.token}` } }, 20_000);
      if (r.status === 401) { this.disconnect(); return; }
      if (!r.ok) throw new Error(`server.logs.tail failed: ${r.status}`);
      const p = JSON.parse(await r.text());
      this.serverLogsPath = typeof p.path === "string" ? p.path : "";
      this.serverLogs = typeof p.text === "string" ? p.text : "";
      this.serverLogsTruncated = Boolean(p.truncated);
    } catch (e: any) { this.serverLogsError = e.message; }
  }

  sessionTitle(r: any): string {
    const s = (r.cwd || "").trim();
    if (!s || s === "." || s === "/") return "";
    return s.split(/[\\/]+/g).filter(Boolean).slice(-1)[0] ?? "";
  }
  sessionSummary(r: any): string { const s = (r.cwd || "").trim(); return (!s || s === ".") ? "" : s; }
  cwdShort(r: any): string { const s = (r.cwd || "").trim(); if (!s || s === ".") return ""; return s.length > 60 ? `${s.slice(0, 60)}…` : s; }

  formatRelativeTime(ts?: string | null): string {
    if (!ts) return "";
    const t = Date.parse(ts);
    if (!Number.isFinite(t)) return "";
    const sec = Math.floor(Math.max(0, Date.now() - t) / 1000);
    if (sec < 10) return "刚刚";
    if (sec < 60) return `${sec}秒前`;
    const min = Math.floor(sec / 60);
    if (min < 60) return `${min}分钟前`;
    const hr = Math.floor(min / 60);
    if (hr < 24) return `${hr}小时前`;
    const day = Math.floor(hr / 24);
    return day < 7 ? `${day}天前` : new Date(t).toLocaleDateString();
  }

  formatAbsTime = (ts: string): string => { const t = Date.parse(ts); return Number.isFinite(t) ? new Date(t).toLocaleString() : ts; };
  renderMarkdownBasic = (src: string): string => {
    const parts = (src ?? "").split("```");
    let out = "";
    for (let i = 0; i < parts.length; i++) {
      const chunk = parts[i] ?? "";
      if (i % 2 === 1) {
        const body = chunk.includes("\n") ? chunk.slice(chunk.indexOf("\n") + 1) : chunk;
        out += `<pre>${this.#escapeHtml(body.trimEnd())}</pre>`;
      } else out += this.#renderMarkdownTextBlock(chunk);
    }
    return out;
  }
  #escapeHtml(s: string): string { return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;"); }
  #renderMarkdownTextBlock(raw: string): string {
    const lines = raw.split("\n"); const blocks: string[] = [];
    for (let i = 0; i < lines.length; i++) {
      const t = lines[i]?.trim(); if (!t) continue;
      const h = t.match(/^(#{1,6})\s+(.*)$/);
      if (h) { blocks.push(`<h${h[1]!.length}>${h[2]}</h${h[1]!.length}>`); continue; }
      blocks.push(`<p>${this.#escapeHtml(t)}</p>`);
    }
    return blocks.join("");
  }

  statusLabel(r: any): { label: string; kind: string } {
    if (r.status === "awaiting_approval") return { label: "待审批", kind: "warning" };
    if (r.status === "awaiting_input") return { label: "待输入", kind: "warning" };
    if (r.status === "running") return { label: "运行中", kind: "running" };
    if (r.status === "exited") return { label: typeof r.exit_code === "number" && r.exit_code !== 0 ? "错误" : "已结束", kind: typeof r.exit_code === "number" && r.exit_code !== 0 ? "error" : "done" };
    return { label: r.status, kind: "done" };
  }

  tailLines(text: string, maxLines: number): string {
    if (!text) return "";
    let idx = text.length; let lines = 0;
    while (idx > 0) { const n = text.lastIndexOf("\n", idx - 1); if (n === -1) break; lines++; if (lines >= maxLines) return text.slice(n + 1); idx = n; }
    return text;
  }

  copyText = (text: string) => {
    if (!text) return;
    try {
      const ta = document.createElement("textarea");
      ta.value = text;
      ta.style.position = "fixed";
      ta.style.opacity = "0";
      document.body.appendChild(ta);
      ta.select();
      document.execCommand("copy");
      document.body.removeChild(ta);
      this.setToast("已复制");
    } catch {
      this.setToast("复制失败");
    }
  };
  riskForOpTool(name?: string | null): any {
    const t = (name ?? "").trim().toLowerCase();
    if (!t) return null;
    if (t.startsWith("fs.read")) return { kind: "read", label: "read" };
    if (t.startsWith("fs.write")) return { kind: "write", label: "write" };
    if (t === "bash" || t.endsWith(".bash")) return { kind: "exec", label: "exec" };
    return { kind: "other", label: "other" };
  }

  awaitingIsApproval(a: any): boolean { if (!a) return false; return Boolean(a.request_id || a.op_tool); }
  awaitingIsPrompt(a: any): boolean { if (!a) return false; return (a.reason ?? "") === "prompt" && !a.request_id && !a.op_tool; }
  awaitingWantsYesNo(a: any): boolean {
    if (!a || a.op_tool) return false;
    const p = (a.prompt ?? "").trim().toLowerCase();
    return p.includes("proceed?") || p.includes("continue?") || p.includes("confirm") || p.includes("(y/n)");
  }

  addTodo(text: string) {
    if (!this.selectedRunId) return;
    const t = text.trim(); if (!t) return;
    this.todos = [{ id: uid(), text: t, done: false, created_at: new Date().toISOString() }, ...this.todos];
    this.#saveTodos(this.selectedRunId, this.todos);
  }
  toggleTodo(id: string) { if (!this.selectedRunId) return; this.todos = this.todos.map((t) => t.id === id ? { ...t, done: !t.done } : t); this.#saveTodos(this.selectedRunId, this.todos); }
  removeTodo(id: string) { if (!this.selectedRunId) return; this.todos = this.todos.filter((t) => t.id !== id); this.#saveTodos(this.selectedRunId, this.todos); }
  #saveTodos(runId: string, items: TodoItem[]) { localStorage.setItem(`relay.todo.${runId}`, JSON.stringify(items)); }
}

export const relay = new RelayStore();
