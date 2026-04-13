<script lang="ts">
  import { onMount, tick } from "svelte";
  import XtermTerminal from "./XtermTerminal.svelte";
  import BlocksRenderer from "./lib/components/BlocksRenderer.svelte";
  import SessionSelector from "./lib/SessionSelector.svelte";
  import { reduceToBlocks } from "./lib/blocks/reduce";
  import { reconcileBlocks } from "./lib/blocks/reconcile";
  import type { UiBlock } from "./lib/blocks/types";
  import uiVersionMeta from "./ui-version.json";
  import TopBar from "./lib/components/TopBar.svelte";
  import NavBar from "./lib/components/NavBar.svelte";
  import Toast from "./lib/components/Toast.svelte";
  import LoginForm from "./lib/components/LoginForm.svelte";
  import SettingsPanel from "./lib/components/SettingsPanel.svelte";
  import SessionList from "./lib/components/SessionList.svelte";
  import SessionDetail from "./lib/components/SessionDetail.svelte";
  import TodoPanel from "./lib/components/TodoPanel.svelte";
  import ToolsPanel from "./lib/components/ToolsPanel.svelte";
  import HostDiagnostics from "./lib/components/HostDiagnostics.svelte";
  import StartRun from "./lib/components/StartRun.svelte";
  import EventLog from "./lib/components/EventLog.svelte";
  import InputModal from "./lib/components/InputModal.svelte";
  import ApprovalModal from "./lib/components/ApprovalModal.svelte";
  import StopConfirmModal from "./lib/components/StopConfirmModal.svelte";

  // Polyfill: crypto.randomUUID is unavailable on non-secure (http://) contexts.
  const uuid: () => string =
    typeof crypto !== "undefined" && typeof crypto.randomUUID === "function"
      ? () => crypto.randomUUID()
      : () => {
          // RFC4122 v4 fallback using crypto.getRandomValues
          const b = new Uint8Array(16);
          (typeof crypto !== "undefined" ? crypto : ({} as Crypto)).getRandomValues(b);
          b[6] = (b[6] & 0x0f) | 0x40;
          b[8] = (b[8] & 0x3f) | 0x80;
          const h = [...b].map((v) => v.toString(16).padStart(2, "0")).join("");
          return `${h.slice(0, 8)}-${h.slice(8, 12)}-${h.slice(12, 16)}-${h.slice(16, 20)}-${h.slice(20)}`;
        };

  type Health = { name: string; version: string };
  type LoginResponse = { access_token: string };
  type RunRow = {
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

  type ChatMessage = {
    key: string;
    ts: string;
    role: "user" | "assistant" | "system";
    kind: string;
    actor?: string | null;
    request_id?: string | null;
    text: string;
    data?: unknown;
  };

  type ChatMessageApi = {
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

  type HostInfo = {
    id: string;
    name?: string | null;
    last_seen_at?: string | null;
    online: boolean;
  };

  type HostToolStatus = {
    tool: string;
    bin?: string | null;
    ok: boolean;
    error?: string | null;
    models?: string[] | null;
    default_model?: string | null;
    models_error?: string | null;
    models_note?: string | null;
  };

  type WsEnvelope = {
    type: string;
    ts: string;
    host_id?: string;
    run_id?: string;
    seq?: number;
    data: unknown;
  };

  const uiVersion = Number((uiVersionMeta as { uiVersion?: number }).uiVersion ?? 0);
  const START_CWD_STORAGE_KEY = "relay.startCwdByHost.v1";

  const DEFAULT_SESSION_LIMIT = 200;
  const WS_BATCH_BUDGET_MS = 10;
  const OUTPUT_TUI_RENDER_THROTTLE_MS = 200;

  function inferDefaultApiBaseUrl(): string {
    if (typeof window === "undefined") return "http://127.0.0.1:8787";
    const host = window.location?.hostname || "127.0.0.1";
    const proto = window.location?.protocol === "https:" ? "https" : "http";
    // If the app is hosted on the server port already, use same-origin.
    if (window.location?.port === "8787") return window.location.origin;
    return `${proto}://${host}:8787`;
  }

  const defaultApiBaseUrl = inferDefaultApiBaseUrl();

  let useCustomServer = false;
  let customBaseUrl = "";
  if (typeof window !== "undefined") {
    const savedBaseUrl = localStorage.getItem("relay.baseUrl") ?? "";
    const savedUseCustom = localStorage.getItem("relay.useCustomServer") === "1";
    customBaseUrl = savedBaseUrl;
    // If user configured a baseUrl, treat it as preferred.
    useCustomServer = savedUseCustom || Boolean(savedBaseUrl);
  }

  $: apiBaseUrl =
    (useCustomServer ? customBaseUrl.trim() : "") || customBaseUrl.trim() || defaultApiBaseUrl;

  let username = "admin";
  let password = "";
  let token = "";
  let keepSignedIn = true;
  let rememberPassword = false;
  type PersistedAuthV1 = {
    username?: string;
    token?: string;
    keepSignedIn?: boolean;
    rememberPassword?: boolean;
    password?: string;
  };
  if (typeof window !== "undefined") {
    try {
      const raw = localStorage.getItem("relay.auth.v1");
      if (raw) {
        const parsed = JSON.parse(raw) as unknown;
        if (parsed && typeof parsed === "object") {
          const v = parsed as PersistedAuthV1;
          if (typeof v.keepSignedIn === "boolean") keepSignedIn = v.keepSignedIn;
          if (typeof v.rememberPassword === "boolean") rememberPassword = v.rememberPassword;
          if (typeof v.username === "string" && v.username.trim()) username = v.username;
          if (typeof v.token === "string" && v.token.trim() && keepSignedIn) token = v.token;
          if (typeof v.password === "string" && rememberPassword) password = v.password;
        }
      }
    } catch {
      // ignore
    }
  }
  let status = "disconnected";
  let loginBusy = false;
  $: loginBusy = status === "checking" || status === "connecting";
  let view: string = "sessions";
  let health: Health | null = null;
  let events: WsEnvelope[] = [];
  let runs: RunRow[] = [];
  let hosts: HostInfo[] = [];
  let ws: WebSocket | null = null;
  let messagesByRun: Record<string, ChatMessage[]> = {};
  let isMobile = false;
  let wsQueue: WsEnvelope[] = [];
  let wsQueuePos = 0;
  let wsFlushScheduled = false;
  let outputTuiLastRenderAt = 0;
  let outputTuiRenderTimer: number | null = null;

  function scheduleTuiRender(runId: string, delayMs: number) {
    if (typeof window === "undefined") return;
    if (!runId) return;
    if (outputTuiRenderTimer) return;
    const safeDelay = Math.max(0, Math.min(2000, delayMs | 0));
    outputTuiRenderTimer = window.setTimeout(() => {
      outputTuiRenderTimer = null;
      if (runId !== selectedRunId) return;
      if (!outputAutoScroll) return;
      if (outputModeByRun[runId] !== "tui") return;
      const t = outputTuiState;
      if (!t || outputTuiRunId !== runId) return;
      outputByRun = { ...outputByRun, [runId]: termToString(t) };
      outputTuiLastRenderAt = performance.now();
    }, safeDelay);
  }

  let selectedRunId = "";
  let showSessionSelector = false;
  let inputModalOpen = false;
  let inputModalText = "";
  let inputModalEl: HTMLTextAreaElement | null = null;
  let lastSeenPromptRequest: Record<string, string> = {};
  let stopConfirmOpen = false;
  let approvalModalOpen = false;
  let approvalModalShowArgs = false;
  let approvalForSession = false;
  let approvalAnswersJson = "";
  let inlineAwaitingText = "";
  let approvalDraftKey = "";
  let awaitingDraftKey = "";
  let lastSeenApprovalRequest: Record<string, string> = {};
  let lastError = "";
  let outputByRun: Record<string, string> = {};
  let runReadyByRun: Record<string, boolean> = {};
  let wsSubscribedRunId = "";
  let wsSubscribedIncludeOutput = true;
  let awaitingByRun: Record<
    string,
    | {
        reason?: string;
        prompt?: string;
        request_id?: string;
        op_tool?: string;
        op_args?: unknown;
        op_args_summary?: string;
        approve_text?: string;
        deny_text?: string;
        questions?: unknown;
      }
    | undefined
  > = {};
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

  let serverLogsPath = "";
  let serverLogs = "";
  let serverLogsLines = "200";
  let serverLogsMaxBytes = "200000";
  let serverLogsTruncated = false;
  let serverLogsError = "";

  type TodoItem = { id: string; text: string; done: boolean; created_at: string };
  let todos: TodoItem[] = [];
  let todoText = "";
  let todoSuggestions: string[] = [];
  let todoSuggestionsTimer: ReturnType<typeof setTimeout> | null = null;

  // Monitor-first: default to structured events view.
  let sessionDetailTab: "output" | "messages" = "messages";
  // Mobile: do not inherit terminal view across sessions.
  // When the selected session changes (including run start), default back to messages (card stream).
  let mobileTabResetRunId = "";
  $: if (isMobile) {
    if (!selectedRunId) {
      mobileTabResetRunId = "";
    } else if (selectedRunId !== mobileTabResetRunId) {
      sessionDetailTab = "messages";
      mobileTabResetRunId = selectedRunId;
    }
  }
  let outputAutoScroll = true;
  let outputIsAtBottom = true;
  let outputBufferLines = 400;
  let outputFeedEl: HTMLDivElement | null = null;
  let outputSearchInputEl: HTMLInputElement | null = null;
  let outputScrollScheduled = false;
  let outputPausedPending = "";
  let outputPausedPendingChars = 0;
  let outputPausedPendingMode: "log" | "tui" = "log";
  let outputPausedPendingMaxSeq = 0;
  let outputModeByRun: Record<string, "log" | "tui"> = {};
  let selectedOutputMode: "log" | "tui" = "log";
  let outputTuiRunId = "";
  let outputTuiState: TerminalState | null = null;
  let xtermRef: {
    write: (data: string) => void;
    reset: () => void;
    focus: () => void;
  } | null = null;
  let xtermRunId = "";
  let xtermAppliedSeq = 0;
  let xtermBackfillRunId = "";
  let xtermBackfillReady = false;
  let xtermBackfillText = "";
  let xtermBackfillMaxSeq = 0;
  let xtermBackfillPending: Array<{ seq?: number; text: string }> = [];
  let xtermPreReady = "";
  let xtermPreReadyMaxSeq = 0;
  let xtermResizeTimer: ReturnType<typeof setTimeout> | null = null;
  let xtermResizePending: { runId: string; cols: number; rows: number } | null = null;
  let xtermLastResizeKey = "";
  let stdinBuf = "";
  let stdinBufRunId = "";
  let stdinBufTimer: ReturnType<typeof setTimeout> | null = null;

  let outputSearchText = "";
  let outputSearchActive = "";
  let outputSearchCursor = 0;
  let chatInputText = "";
  let chatInputEl: HTMLTextAreaElement | null = null;

  let uiBlocks: UiBlock[] = [];

  type OutputMatch = {
    id: string;
    line: number;
    start: number;
    end: number;
  };
  let outputSearchMatches: OutputMatch[] = [];
  let outputHtml = "";

  let toastText = "";
  let toastTimer: ReturnType<typeof setTimeout> | null = null;

  const START_TOOL_OPTIONS = [
    { value: "opencode", label: "opencode" },
  ] as const;
  type StartToolOption = (typeof START_TOOL_OPTIONS)[number];

  let startHostId = "host-dev";
  let startTool: StartToolOption["value"] = "opencode";
  // Leave empty to run the tool's default entrypoint.
  let startCmd = "";
  let startCwd = "";
  let startError = "";
  let startCwdByHost: Record<string, string> = {};
  let startHostToolsById: Record<string, HostToolStatus[] | undefined> = {};
  let startHostToolsLoadingById: Record<string, boolean | undefined> = {};
  let currentStartToolOptions: readonly StartToolOption[] = [...START_TOOL_OPTIONS];
  let currentStartToolStatuses: HostToolStatus[] | null = null;
  let currentStartHostToolsLoading = false;
  let currentStartOpencodeModels: string[] = [];
  let currentStartOpencodeDefaultModel = "";
  let currentStartOpencodeModelsError = "";
  let currentStartOpencodeModelsNote = "";
  let lastStartToolsForceRefreshKey = "";
  let lastSuggestedStartCwd = "";
  let recentSessions: RunRow[] = [];
  let startOpencodeModel = "";
  let startOpencodeSessionId = "";

  if (typeof window !== "undefined") {
    try {
      const raw = localStorage.getItem(START_CWD_STORAGE_KEY);
      if (raw) {
        const parsed = JSON.parse(raw) as unknown;
        if (parsed && typeof parsed === "object") {
          startCwdByHost = parsed as Record<string, string>;
        }
      }
    } catch {
      // ignore
    }
  }

  const pendingRpc = new Map<string, (msg: WsEnvelope) => void>();

  type SearchMatch = { path: string; line: number; column: number; text: string };

  let sessionSearch = "";
  let hostGroupCollapsed: Record<string, boolean> = {};
  if (typeof window !== "undefined") {
    try {
      const raw = localStorage.getItem("relay.hostGroupCollapsed.v1");
      if (raw) {
        const parsed = JSON.parse(raw) as unknown;
        if (parsed && typeof parsed === "object") hostGroupCollapsed = parsed as Record<string, boolean>;
      }
    } catch {
      // ignore
    }
  }

  function persistServerPrefs() {
    if (typeof window === "undefined") return;
    localStorage.setItem("relay.useCustomServer", useCustomServer ? "1" : "0");
    if (customBaseUrl.trim()) localStorage.setItem("relay.baseUrl", customBaseUrl.trim());
    else localStorage.removeItem("relay.baseUrl");
  }

  function persistAuthPrefs() {
    if (typeof window === "undefined") return;
    const payload: PersistedAuthV1 = {
      username: username.trim(),
      keepSignedIn,
      rememberPassword,
    };
    if (keepSignedIn && token) payload.token = token;
    if (rememberPassword && password) payload.password = password;
    try {
      localStorage.setItem("relay.auth.v1", JSON.stringify(payload));
    } catch {
      // ignore
    }
  }

  function persistHostGroupCollapsed() {
    if (typeof window === "undefined") return;
    try {
      localStorage.setItem("relay.hostGroupCollapsed.v1", JSON.stringify(hostGroupCollapsed));
    } catch {
      // ignore
    }
  }

  function persistStartCwdByHost() {
    if (typeof window === "undefined") return;
    try {
      localStorage.setItem(START_CWD_STORAGE_KEY, JSON.stringify(startCwdByHost));
    } catch {
      // ignore
    }
  }

  function suggestedStartCwdForHost(hostId: string): string {
    const saved = startCwdByHost[hostId]?.trim();
    if (saved) return saved;
    const recent = runs.find((run) => run.host_id === hostId && run.cwd?.trim());
    if (recent?.cwd?.trim()) return recent.cwd.trim();
    const recentSession = recentSessions.find((run) => run.host_id === hostId && run.cwd?.trim());
    if (recentSession?.cwd?.trim()) return recentSession.cwd.trim();
    return "";
  }

  function rememberStartCwd(hostId: string, cwd: string) {
    const normalized = cwd.trim();
    if (!hostId.trim() || !normalized) return;
    startCwdByHost = { ...startCwdByHost, [hostId.trim()]: normalized };
    persistStartCwdByHost();
  }

  function humanizeStartError(message: string): string {
    const trimmed = message.trim();
    if (trimmed.startsWith("run cwd does not exist:")) {
      const badPath = trimmed.slice("run cwd does not exist:".length).trim();
      const suggestion = suggestedStartCwdForHost(startHostId) || "/home/ab/test";
      return `启动失败：主机路径不存在：${badPath}。请填写远程主机上的真实目录，例如 ${suggestion}。`;
    }
    if (trimmed.startsWith("run cwd is not a directory:")) {
      const badPath = trimmed.slice("run cwd is not a directory:".length).trim();
      return `启动失败：${badPath} 不是目录，请改成远程主机上的项目目录。`;
    }
    return trimmed;
  }

  function applySuggestedStartCwd(force = false) {
    const suggested = suggestedStartCwdForHost(startHostId);
    const current = startCwd.trim();
    if (force || !current || current === lastSuggestedStartCwd) {
      startCwd = suggested;
    }
    lastSuggestedStartCwd = suggested;
  }

  $: if (startHostId) {
    applySuggestedStartCwd(false);
  }

  function isRecord(v: unknown): v is Record<string, unknown> {
    return Boolean(v) && typeof v === "object";
  }

  function parseHostToolStatuses(value: unknown): HostToolStatus[] {
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

  function currentOpencodeToolStatus(): HostToolStatus | null {
    const found = currentStartToolStatuses?.find((item) => item.tool === "opencode") ?? null;
    return found;
  }

  function syncStartOpencodeModelForHost() {
    const status = currentOpencodeToolStatus();
    const options = status?.models?.filter((item) => item.trim()) ?? [];
    const defaultModel = status?.default_model?.trim() ?? "";
    if (!options.length) {
      startOpencodeModel = defaultModel;
      return;
    }
    if (startOpencodeModel && options.includes(startOpencodeModel)) return;
    startOpencodeModel = defaultModel && options.includes(defaultModel) ? defaultModel : options[0] ?? "";
  }

  function dynamicStartToolOptionsForHost(hostId: string): StartToolOption[] {
    const statuses = startHostToolsById[hostId];
    if (!statuses) return [...START_TOOL_OPTIONS];
    const allowed = new Set(statuses.filter((item) => item.ok).map((item) => item.tool));
    return START_TOOL_OPTIONS.filter((option) => allowed.has(option.value));
  }

  function syncStartToolSelectionForHost(hostId: string) {
    const options = dynamicStartToolOptionsForHost(hostId);
    if (options.length === 0) return;
    if (!options.some((option) => option.value === startTool)) {
      startTool = options[0]!.value;
    }
  }

  function basename(path: string): string {
    const trimmed = path.trim();
    if (!trimmed || trimmed === "." || trimmed === "/") return "";
    const parts = trimmed.split(/[\\/]+/g).filter(Boolean);
    return parts[parts.length - 1] ?? "";
  }

  function sessionTitle(r: RunRow): string {
    return basename(r.cwd) || "";
  }

  function sessionSummary(r: RunRow): string {
    const s = (r.cwd || "").trim();
    if (!s || s === ".") return "";
    return s.length > 60 ? `${s.slice(0, 60)}…` : s;
  }

  function compareTsDesc(a?: string | null, b?: string | null): number {
    const ta = a ? Date.parse(a) : 0;
    const tb = b ? Date.parse(b) : 0;
    return tb - ta;
  }

  function formatRelativeTime(ts?: string | null): string {
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

  function formatAbsTime(ts: string): string {
    const t = Date.parse(ts);
    if (!Number.isFinite(t)) return ts;
    return new Date(t).toLocaleString();
  }

  function setToast(text: string) {
    toastText = text;
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
      toastText = "";
      toastTimer = null;
    }, 1500);
  }

  async function copyText(text: string) {
    try {
      await navigator.clipboard.writeText(text || "");
      setToast("已复制");
    } catch (e) {
      setToast(e instanceof Error ? e.message : "复制失败");
    }
  }

  function connLabel(s: string): string {
    if (s === "connected") return "已连接";
    if (s === "checking") return "检查中";
    if (s === "connecting") return "连接中";
    if (s === "disconnected") return "未连接";
    if (s === "error") return "错误";
    return s;
  }

  function statusLabel(r: RunRow): { label: string; kind: "running" | "warning" | "error" | "done" } {
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

  function awaitingFromRunRow(r: RunRow | null): {
    reason?: string;
    prompt?: string;
    request_id?: string;
    op_tool?: string;
    op_args?: unknown;
    op_args_summary?: string;
    approve_text?: string;
    deny_text?: string;
    questions?: unknown;
  } | null {
    if (!r) return null;
    if (!r.pending_request_id && !r.pending_prompt && !r.pending_op_tool && !r.pending_op_args_summary) return null;
    return {
      reason: r.pending_reason ?? undefined,
      prompt: r.pending_prompt ?? undefined,
      request_id: r.pending_request_id ?? undefined,
      op_tool: r.pending_op_tool ?? undefined,
      op_args_summary: r.pending_op_args_summary ?? undefined,
      op_args: undefined,
      approve_text: undefined,
      deny_text: undefined,
    };
  }

  type RiskKind = "read" | "write" | "exec" | "other";

  function normalizeOpToolName(name: string): string {
    const raw = name.trim().toLowerCase();
    if (raw.startsWith("rpc.")) return raw.slice(4);
    return raw;
  }

  function riskForOpTool(name?: string | null): { kind: RiskKind; label: string } | null {
    const t = (name ?? "").trim();
    if (!t) return null;
    const n = normalizeOpToolName(t);

    if (n === "fs.read" || n.startsWith("fs.read.")) return { kind: "read", label: "read" };
    if (n === "fs.write" || n.startsWith("fs.write.")) return { kind: "write", label: "write" };
    if (n === "bash" || n === "host.bash" || n.endsWith(".bash")) return { kind: "exec", label: "exec" };

    return { kind: "other", label: "other" };
  }

  function awaitingIsPrompt(a: {
    reason?: string;
    request_id?: string;
    op_tool?: string;
  }): boolean {
    const reason = (a.reason ?? "").trim().toLowerCase();
    const reqId = (a.request_id ?? "").trim();
    const opTool = (a.op_tool ?? "").trim();
    return reason === "prompt" && !opTool && !reqId;
  }

  function promptWantsYesNo(prompt?: string | null): boolean {
    const p = (prompt ?? "").trim().toLowerCase();
    if (!p) return false;
    if (p.includes("proceed?")) return true;
    if (p.includes("continue?")) return true;
    if (p.includes("confirm")) return true;
    if (p.includes("(y/n)")) return true;
    if (p.includes("[y/n]")) return true;
    if (p.includes("[y/n]")) return true;
    return /\by\s*\/\s*n\b/i.test(p);
  }

  function awaitingWantsYesNo(a: { reason?: string; op_tool?: string; request_id?: string; prompt?: string }): boolean {
    if ((a.op_tool ?? "").trim()) return false;
    const reason = (a.reason ?? "").trim().toLowerCase();
    if (reason !== "prompt") return false;
    // If it's a prompt that looks like a yes/no confirmation (e.g. "Proceed?"), treat it like an approval.
    return promptWantsYesNo((a as unknown as { prompt?: string }).prompt ?? "");
  }

  function awaitingIsApproval(a: {
    reason?: string;
    request_id?: string;
    op_tool?: string;
  }): boolean {
    if (awaitingIsPrompt(a)) return false;
    const reqId = (a.request_id ?? "").trim();
    const opTool = (a.op_tool ?? "").trim();
    return Boolean(reqId || opTool);
  }

  function hostDisplayName(h: HostInfo | null | undefined, hostId: string): string {
    const name = (h?.name ?? "").trim();
    return name || hostId;
  }

  function toggleHostGroup(hostId: string) {
    hostGroupCollapsed = { ...hostGroupCollapsed, [hostId]: !hostGroupCollapsed[hostId] };
    persistHostGroupCollapsed();
  }

  function scheduleWsFlush() {
    if (wsFlushScheduled) return;
    wsFlushScheduled = true;
    requestAnimationFrame(flushWsQueue);
  }

  function flushWsQueue() {
    wsFlushScheduled = false;
    if (wsQueue.length === 0) {
      wsQueuePos = 0;
      return;
    }

    // Periodically compact processed messages to avoid unbounded growth when the
    // websocket is producing faster than we can render.
    if (wsQueuePos > 1000) {
      wsQueue = wsQueue.slice(wsQueuePos);
      wsQueuePos = 0;
    }

    const started = performance.now();

    const shouldCaptureEvents = token && view === "settings";
    const shouldAppendMessages = token && view === "sessions" && Boolean(selectedRunId);
    const selectedId = selectedRunId;

    let nextRuns = runs;
    let nextRunsIndex: Map<string, number> | null = null;
    let runsChanged = false;

    let nextAwaiting = awaitingByRun;
    let awaitingChanged = false;

    let nextOutputByRun = outputByRun;
    let selectedOutput = selectedId ? outputByRun[selectedId] ?? "" : "";
    let outputChanged = false;
    let outputByRunChanged = false;

    let nextRunReadyByRun = runReadyByRun;
    let readyChanged = false;
    const pendingOutputChunks: string[] = [];
    const pendingPausedChunks: string[] = [];
    const pendingPausedTuiChunks: string[] = [];
    const pendingTuiWriteChunks: string[] = [];
    let pendingTuiWriteMaxSeq = 0;

    let nextOutputModeByRun = outputModeByRun;
    let outputModeChanged = false;
    let selectedMode: "log" | "tui" = selectedId ? nextOutputModeByRun[selectedId] ?? "log" : "log";

    let tuiState = outputTuiState;
    let tuiRunId = outputTuiRunId;
    let tuiChanged = false;
    if (selectedId && selectedMode === "tui" && tuiRunId !== selectedId) {
      tuiState = newTerminalState();
      tuiRunId = selectedId;
    }

    const newEvents: WsEnvelope[] = [];
    const newMessages: ChatMessage[] = [];

    const ensureRunsIndex = () => {
      if (nextRunsIndex) return nextRunsIndex;
      nextRunsIndex = new Map<string, number>();
      for (let i = 0; i < nextRuns.length; i++) {
        const r = nextRuns[i];
        if (r?.id) nextRunsIndex.set(r.id, i);
      }
      return nextRunsIndex;
    };

    const upsertRun = (runId: string, updater: (cur: RunRow) => RunRow) => {
      const idx = ensureRunsIndex().get(runId);
      if (idx === undefined) return;
      if (!runsChanged) {
        nextRuns = [...nextRuns];
        runsChanged = true;
      }
      nextRuns[idx] = updater(nextRuns[idx]!);
    };

    while (wsQueuePos < wsQueue.length) {
      const msg = wsQueue[wsQueuePos]!;
      wsQueuePos++;

      if (shouldCaptureEvents) newEvents.push(msg);

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

      if (msg.run_id && msg.type === "run.ready") {
        if (!readyChanged) {
          nextRunReadyByRun = { ...nextRunReadyByRun };
          readyChanged = true;
        }
        nextRunReadyByRun[msg.run_id] = true;
      }

      if (msg.run_id && msg.type === "run.output") {
        const runId = msg.run_id;
        const raw0 = dataString(msg, "text") ?? "";
        const seq = typeof msg.seq === "number" ? msg.seq : undefined;
        const raw = codexStructuredEventText(raw0) ?? raw0;
        if (raw) {
          if (!readyChanged) {
            nextRunReadyByRun = { ...nextRunReadyByRun };
            readyChanged = true;
          }
          nextRunReadyByRun[runId] = true;
          if (runId === selectedId) {
            // Auto-detect: if we see cursor movement / clear-screen ANSI, switch to TUI snapshot mode.
            if (selectedId && selectedMode === "log" && looksLikeTuiAnsi(raw)) {
              if (!outputModeChanged) {
                nextOutputModeByRun = { ...nextOutputModeByRun };
                outputModeChanged = true;
              }
              nextOutputModeByRun[selectedId] = "tui";
              selectedMode = "tui";
              tuiState = newTerminalState();
              tuiRunId = selectedId;
              selectedOutput = "";
            }

            if (selectedMode === "tui") {
              if (typeof seq === "number" && seq > 0 && seq <= xtermAppliedSeq) {
                // already applied (e.g. after backfill)
              } else if (xtermBackfillRunId === selectedId) {
                xtermBackfillPending.push({ seq, text: raw });
              } else if (!outputAutoScroll) {
                pendingPausedTuiChunks.push(raw);
                if (typeof seq === "number") outputPausedPendingMaxSeq = Math.max(outputPausedPendingMaxSeq, seq);
              } else if (xtermRef && xtermRunId === selectedId) {
                pendingTuiWriteChunks.push(raw);
                if (typeof seq === "number") pendingTuiWriteMaxSeq = Math.max(pendingTuiWriteMaxSeq, seq);
              } else {
                xtermPreReady = truncateTail(`${xtermPreReady}${raw}`, 500_000);
                if (typeof seq === "number") xtermPreReadyMaxSeq = Math.max(xtermPreReadyMaxSeq, seq);
              }
            } else {
              const text = sanitizeTerminalOutput(raw);
              if (text) {
                if (outputAutoScroll) {
                  pendingOutputChunks.push(text);
                  outputChanged = true;
                } else {
                  pendingPausedChunks.push(text);
                }
              }
            }
          } else {
            // Keep a small tail buffer for non-selected sessions so switching between
            // runs doesn't require a full HTTP backfill to see recent output.
            const mode: "log" | "tui" = nextOutputModeByRun[runId] ?? "log";
            if (mode === "log") {
              const text = sanitizeTerminalOutput(raw);
              if (text) {
                if (!outputByRunChanged) {
                  nextOutputByRun = { ...nextOutputByRun };
                  outputByRunChanged = true;
                }
                const cur = nextOutputByRun[runId] ?? "";
                nextOutputByRun[runId] = truncateTail(`${cur}${text}`, 200_000);
              }
            }
          }
        }
      }

      if (msg.run_id && msg.type === "run.awaiting_input") {
        const reqId = dataString(msg, "request_id");
        if (!awaitingChanged) {
          nextAwaiting = { ...nextAwaiting };
          awaitingChanged = true;
        }
        nextAwaiting[msg.run_id] = {
          reason: dataString(msg, "reason"),
          prompt: dataString(msg, "prompt"),
          request_id: reqId,
          approve_text: dataString(msg, "approve_text"),
          deny_text: dataString(msg, "deny_text"),
        };
      }
      if (msg.run_id && msg.type === "run.permission_requested") {
        if (!awaitingChanged) {
          nextAwaiting = { ...nextAwaiting };
          awaitingChanged = true;
        }
        nextAwaiting[msg.run_id] = {
          reason: dataString(msg, "reason"),
          prompt: dataString(msg, "prompt"),
          request_id: dataString(msg, "request_id"),
          op_tool: dataString(msg, "op_tool"),
          op_args: dataAny(msg, "op_args"),
          op_args_summary: dataString(msg, "op_args_summary"),
          approve_text: dataString(msg, "approve_text"),
          deny_text: dataString(msg, "deny_text"),
          questions: dataAny(msg, "questions"),
        };
      }
      if (msg.run_id && msg.type === "run.input") {
        if (!awaitingChanged) {
          nextAwaiting = { ...nextAwaiting };
          awaitingChanged = true;
        }
        nextAwaiting[msg.run_id] = undefined;
      }
      if (msg.run_id && msg.type === "run.exited") {
        if (!awaitingChanged) {
          nextAwaiting = { ...nextAwaiting };
          awaitingChanged = true;
        }
        nextAwaiting[msg.run_id] = undefined;
      }

      // Best-effort run status updates (skip high-volume events to keep UI responsive).
      if (msg.run_id) {
        const hasReqId = dataString(msg, "request_id");
        const last_active_at = msg.ts;
        const opencodeSessionId = dataString(msg, "opencode_session_id");

        if (opencodeSessionId) {
          upsertRun(msg.run_id, (cur) => ({
            ...cur,
            opencode_session_id: opencodeSessionId,
            last_active_at,
          }));
        }

        if (msg.type === "run.started") {
          if (!readyChanged) {
            nextRunReadyByRun = { ...nextRunReadyByRun };
            readyChanged = true;
          }
          nextRunReadyByRun[msg.run_id] = false;

          const tool = dataString(msg, "tool") ?? "unknown";
          const runnerMode = dataString(msg, "runner_mode") ?? dataString(msg, "mode") ?? "";
          const isLikelyTuiTool = tool === "codex" || tool === "gemini";
          const mode: "log" | "tui" = runnerMode === "structured" ? "log" : isLikelyTuiTool ? "tui" : "log";
          if (nextOutputModeByRun[msg.run_id] !== mode) {
            if (!outputModeChanged) {
              nextOutputModeByRun = { ...nextOutputModeByRun };
              outputModeChanged = true;
            }
            nextOutputModeByRun[msg.run_id] = mode;
            if (msg.run_id === selectedId) {
              selectedMode = mode;
              if (mode === "tui") {
                tuiState = newTerminalState();
                tuiRunId = selectedId;
                selectedOutput = "";
              }
            }
          }

          const idx = ensureRunsIndex().get(msg.run_id);
          if (idx === undefined) {
            if (!runsChanged) {
              nextRuns = [...nextRuns];
              runsChanged = true;
            }
            nextRuns.unshift({
              id: msg.run_id,
              host_id: msg.host_id ?? "unknown",
              tool,
              opencode_session_id: opencodeSessionId,
              cwd: dataString(msg, "cwd") ?? ".",
              status: "running",
              started_at: msg.ts,
              last_active_at: msg.ts,
            });
            nextRunsIndex = null;
          } else {
            upsertRun(msg.run_id, (cur) => ({
              ...cur,
              host_id: msg.host_id ?? cur.host_id,
              tool,
              opencode_session_id: opencodeSessionId ?? cur.opencode_session_id,
              cwd: dataString(msg, "cwd") ?? cur.cwd,
              status: "running",
              started_at: msg.ts,
              last_active_at,
              ended_at: null,
              exit_code: null,
            }));
          }
        } else if (msg.type === "run.permission_requested") {
          upsertRun(msg.run_id, (cur) => ({
            ...cur,
            last_active_at,
            status: "awaiting_approval",
            pending_request_id: dataString(msg, "request_id"),
            pending_reason: dataString(msg, "reason"),
            pending_prompt: dataString(msg, "prompt"),
            pending_op_tool: dataString(msg, "op_tool"),
            pending_op_args_summary: dataString(msg, "op_args_summary"),
          }));
        } else if (msg.type === "run.awaiting_input") {
          upsertRun(msg.run_id, (cur) => ({ ...cur, last_active_at, status: hasReqId ? "awaiting_approval" : "awaiting_input" }));
        } else if (msg.type === "run.input") {
          upsertRun(msg.run_id, (cur) => ({
            ...cur,
            last_active_at,
            status: "running",
            pending_request_id: null,
            pending_reason: null,
            pending_prompt: null,
            pending_op_tool: null,
            pending_op_args_summary: null,
          }));
        } else if (msg.type === "run.exited") {
          if (!readyChanged) {
            nextRunReadyByRun = { ...nextRunReadyByRun };
            readyChanged = true;
          }
          nextRunReadyByRun[msg.run_id] = false;

          const exit_code =
            isRecord(msg.data) && typeof msg.data["exit_code"] === "number" ? msg.data["exit_code"] : undefined;
          upsertRun(msg.run_id, (cur) => ({
            ...cur,
            last_active_at,
            status: "exited",
            ended_at: msg.ts,
            exit_code: typeof exit_code === "number" ? exit_code : cur.exit_code,
            pending_request_id: null,
            pending_reason: null,
            pending_prompt: null,
            pending_op_tool: null,
            pending_op_args_summary: null,
          }));
        }
      }

      if (shouldAppendMessages && msg.run_id && msg.run_id === selectedId) {
        // Only append full message stream when the Messages tab is active.
        // For Codex structured mode, also surface `codex/event` notifications even
        // when viewing Output to avoid "I sent hello but got nothing" confusion.
        const allow =
          sessionDetailTab === "messages" ||
          (msg.type === "run.output" && Boolean(codexStructuredEventText(dataString(msg, "text") ?? "")));
        if (allow) {
          const m = envToMessage(msg);
          if (m) newMessages.push(m);
        }
      }

      if (performance.now() - started > WS_BATCH_BUDGET_MS) break;
    }

    if (selectedId && selectedMode === "tui") {
      if (pendingTuiWriteChunks.length > 0 && xtermRef && xtermRunId === selectedId && outputAutoScroll) {
        const chunk = pendingTuiWriteChunks.join("");
        if (chunk) {
          xtermRef.write(chunk);
          xtermAppliedSeq = Math.max(xtermAppliedSeq, pendingTuiWriteMaxSeq);
        }
      }
      if (pendingPausedTuiChunks.length > 0) {
        const append = pendingPausedTuiChunks.join("");
        if (append) {
          outputPausedPendingMode = "tui";
          outputPausedPending = truncateTail(outputPausedPending + append, 400_000);
          outputPausedPendingChars = Math.min(2_000_000_000, outputPausedPendingChars + append.length);
        }
      }
    } else {
      if (outputChanged && selectedId) {
        const append = pendingOutputChunks.length > 0 ? pendingOutputChunks.join("") : "";
        if (append) selectedOutput = truncateTail(applyTerminalEdits(selectedOutput, append), 200_000);
        nextOutputByRun = { ...nextOutputByRun, [selectedId]: selectedOutput };
        outputByRunChanged = true;
      }
      if (pendingPausedChunks.length > 0 && selectedId) {
        const append = pendingPausedChunks.join("");
        if (append) {
          outputPausedPendingMode = "log";
          outputPausedPending = truncateTail(applyTerminalEdits(outputPausedPending, append), 200_000);
          outputPausedPendingChars = Math.min(2_000_000_000, outputPausedPendingChars + append.length);
        }
      }
    }

    if (outputByRunChanged) outputByRun = nextOutputByRun;
    if (outputModeChanged) outputModeByRun = nextOutputModeByRun;
    if (awaitingChanged) awaitingByRun = nextAwaiting;
    if (runsChanged) runs = nextRuns;
    if (readyChanged) runReadyByRun = nextRunReadyByRun;
    if (shouldCaptureEvents && newEvents.length > 0) {
      events = [...newEvents.reverse(), ...events].slice(0, 500);
    }
    if (newMessages.length > 0 && selectedId) {
      const existing = messagesByRun[selectedId] ?? [];
      messagesByRun = { ...messagesByRun, [selectedId]: truncateHead([...existing, ...newMessages], 1000) };
    }

    if (wsQueuePos >= wsQueue.length) {
      wsQueue = [];
      wsQueuePos = 0;
      return;
    }

    scheduleWsFlush();
  }

  async function focusOutputSearch() {
    await tick();
    updateOutputBufferLines();
    if (outputFeedEl && outputAutoScroll) {
      outputFeedEl.scrollTop = outputFeedEl.scrollHeight;
      outputIsAtBottom = true;
    }
    outputSearchInputEl?.focus();
  }

  async function selectSession(runId: string) {
    selectedRunId = runId;
    subscribeToRun(runId);
    const tool = runToolFor(runId);
    const isLikelyTuiTool = isLikelyTuiToolName(tool);
    sessionDetailTab = isLikelyTuiTool ? "output" : "messages";
    outputAutoScroll = true;
    outputPausedPending = "";
    outputPausedPendingChars = 0;
    outputPausedPendingMode = "log";
    outputPausedPendingMaxSeq = 0;
    xtermRunId = "";
    xtermAppliedSeq = 0;
    xtermBackfillRunId = "";
    xtermBackfillReady = false;
    xtermBackfillText = "";
    xtermBackfillMaxSeq = 0;
    xtermBackfillPending = [];
    xtermPreReady = "";
    xtermPreReadyMaxSeq = 0;
    stdinBuf = "";
    stdinBufRunId = "";
    if (stdinBufTimer) {
      clearTimeout(stdinBufTimer);
      stdinBufTimer = null;
    }
    outputTuiRunId = "";
    outputTuiState = null;
    outputTuiLastRenderAt = 0;
    if (outputTuiRenderTimer) {
      clearTimeout(outputTuiRenderTimer);
      outputTuiRenderTimer = null;
    }
    outputSearchText = "";
    outputSearchActive = "";
    outputSearchCursor = 0;
    chatInputText = "";
    inlineAwaitingText = "";
    if (!messagesByRun[runId]) void loadMessages(runId);
    else void loadMessages(runId);
    await tick();
    chatInputEl?.focus();
  }

  function isLikelyTuiToolName(tool: string): boolean {
    return tool === "codex" || tool === "gemini";
  }

  function runToolFor(runId: string): string {
    return runs.find((x) => x.id === runId)?.tool ?? "";
  }

  function currentOutputModeForRun(runId: string): "log" | "tui" {
    const tool = runToolFor(runId);
    return outputModeByRun[runId] ?? (isLikelyTuiToolName(tool) ? "tui" : "log");
  }

  function includeOutputInMessages(runId: string): boolean {
    const tool = runToolFor(runId);
    return tool === "opencode" || currentOutputModeForRun(runId) === "log";
  }

  function shouldSubscribeOutput(runId: string): boolean {
    return sessionDetailTab === "output" || includeOutputInMessages(runId);
  }

  function tailLines(text: string, maxLines: number): string {
    const max = Math.max(1, maxLines | 0);
    if (!text) return "";
    // Avoid `split()` to reduce allocations while streaming output.
    let idx = text.length;
    let lines = 0;
    while (idx > 0) {
      const next = text.lastIndexOf("\n", idx - 1);
      if (next === -1) break;
      lines++;
      if (lines >= max) {
        return text.slice(next + 1);
      }
      idx = next;
    }
    return text;
  }

  function updateOutputBufferLines() {
    const viewHeight = outputFeedEl?.clientHeight ?? 520;
    const lineHeight = 18;
    const visibleLines = Math.max(1, Math.floor(viewHeight / lineHeight));
    outputBufferLines = Math.min(2000, Math.max(200, visibleLines * 4));
  }

  function scheduleOutputScrollToBottom() {
    if (outputScrollScheduled) return;
    outputScrollScheduled = true;
    requestAnimationFrame(() => {
      outputScrollScheduled = false;
      if (!outputFeedEl) return;
      if (sessionDetailTab !== "output" || !outputAutoScroll) return;
      outputFeedEl.scrollTop = outputFeedEl.scrollHeight;
      outputIsAtBottom = true;
    });
  }

  function outputAtBottom(el: HTMLDivElement): boolean {
    const threshold = 8;
    return el.scrollTop + el.clientHeight >= el.scrollHeight - threshold;
  }

  async function resumeOutputAutoScroll() {
    const runId = selectedRunId;
    if (runId && outputPausedPending) {
      if (outputPausedPendingMode === "tui") {
        if (xtermRef && xtermRunId === runId) {
          xtermRef.write(outputPausedPending);
          xtermAppliedSeq = Math.max(xtermAppliedSeq, outputPausedPendingMaxSeq);
        } else {
          xtermPreReady = truncateTail(`${xtermPreReady}${outputPausedPending}`, 500_000);
          xtermPreReadyMaxSeq = Math.max(xtermPreReadyMaxSeq, outputPausedPendingMaxSeq);
        }
      } else {
        const maxChars = 200_000;
        const base = outputByRun[runId] ?? "";
        const merged = truncateTail(applyTerminalEdits(base, outputPausedPending), maxChars);
        outputByRun = { ...outputByRun, [runId]: merged };
      }
      outputPausedPending = "";
      outputPausedPendingChars = 0;
      outputPausedPendingMode = "log";
      outputPausedPendingMaxSeq = 0;
    }
    outputAutoScroll = true;
    await tick();
    if (outputFeedEl) {
      outputFeedEl.scrollTop = outputFeedEl.scrollHeight;
      outputIsAtBottom = true;
    }
  }

  function pauseOutputAutoScroll() {
    outputAutoScroll = false;
  }

  function handleOutputScroll() {
    if (!outputFeedEl) return;
    const atBottom = outputAtBottom(outputFeedEl);
    outputIsAtBottom = atBottom;
    if (!atBottom) {
      if (outputAutoScroll) pauseOutputAutoScroll();
    }
  }

  async function toggleOutputAutoScroll() {
    if (outputAutoScroll) pauseOutputAutoScroll();
    else await resumeOutputAutoScroll();
  }

  async function jumpToLatest() {
    await resumeOutputAutoScroll();
  }

  function computeOutputMatches(lines: string[], q: string, runId: string): OutputMatch[] {
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
        matches.push({
          id: `out-match-${safeRunId}-${matchIndex}`,
          line: lineIndex,
          start: idx,
          end: idx + query.length,
        });
        matchIndex++;
        from = idx + Math.max(1, query.length);
      }
    }
    return matches;
  }

  function renderOutputHtml(lines: string[], matches: OutputMatch[], cursor: number): string {
    if (matches.length === 0) return escapeHtml(lines.join("\n"));
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
      if (lineMatches.length === 0) {
        out += escapeHtml(line);
        if (lineIndex !== lines.length - 1) out += "\n";
        continue;
      }
      let cursorPos = 0;
      for (const m of lineMatches) {
        out += escapeHtml(line.slice(cursorPos, m.start));
        const cls = matches[cursor]?.id === m.id ? "out-mark current" : "out-mark";
        out += `<mark id="${m.id}" class="${cls}">${escapeHtml(line.slice(m.start, m.end))}</mark>`;
        cursorPos = m.end;
      }
      out += escapeHtml(line.slice(cursorPos));
      if (lineIndex !== lines.length - 1) out += "\n";
    }
    return out;
  }

  async function scrollToCurrentOutputMatch() {
    const m = outputSearchMatches[outputSearchCursor];
    if (!m) return;
    await tick();
    const el = document.getElementById(m.id);
    if (el) el.scrollIntoView({ block: "center" });
  }

  async function runOutputSearch() {
    outputSearchActive = outputSearchText.trim();
    outputSearchCursor = 0;
    if (!outputSearchActive) return;
    pauseOutputAutoScroll();
    await scrollToCurrentOutputMatch();
  }

  async function nextOutputMatch() {
    if (outputSearchMatches.length === 0) return;
    outputSearchCursor = (outputSearchCursor + 1) % outputSearchMatches.length;
    pauseOutputAutoScroll();
    await scrollToCurrentOutputMatch();
  }

  async function prevOutputMatch() {
    if (outputSearchMatches.length === 0) return;
    outputSearchCursor = (outputSearchCursor - 1 + outputSearchMatches.length) % outputSearchMatches.length;
    pauseOutputAutoScroll();
    await scrollToCurrentOutputMatch();
  }

  function clearOutputSearch() {
    outputSearchText = "";
    outputSearchActive = "";
    outputSearchCursor = 0;
  }

  function handleOutputSearchKeydown(ev: KeyboardEvent) {
    if (ev.key === "Enter") {
      ev.preventDefault();
      runOutputSearch();
      return;
    }
    if (ev.key === "ArrowDown") {
      ev.preventDefault();
      nextOutputMatch();
      return;
    }
    if (ev.key === "ArrowUp") {
      ev.preventDefault();
      prevOutputMatch();
      return;
    }
  }

  async function copyOutput() {
    try {
      await navigator.clipboard.writeText(selectedOutput || "");
      setToast("已复制");
    } catch (e) {
      setToast(e instanceof Error ? e.message : "复制失败");
    }
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

    return blocks.join("");
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

    return out;
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

  function sanitizeTerminalOutput(input: string): string {
    const s0 = input ?? "";
    if (!s0) return "";
    // Fast-path: most output is plain text.
    if (!s0.includes("\x1b") && !s0.includes("\r")) return s0;

    // Keep carriage returns: we apply CR semantics when merging into the output buffer.
    let s = s0.replace(/\r\n/g, "\n");

    if (s.includes("\x1b")) {
      // CSI: ESC [ ... <final>
      s = s.replace(/\x1b\[[0-?]*[ -/]*[@-~]/g, "");
      // OSC: ESC ] ... BEL or ESC \
      s = s.replace(/\x1b\][^\x07]*(?:\x07|\x1b\\)/g, "");
      // DCS: ESC P ... ESC \
      s = s.replace(/\x1bP[\s\S]*?\x1b\\/g, "");
      // Single-character escapes.
      s = s.replace(/\x1b[@-Z\\-_]/g, "");
    }

    // Strip remaining control chars (keep \n, \r, \t, and \b).
    s = s.replace(/[\x00-\x07\x0B\x0C\x0E-\x1F\x7F]/g, "");
    return s;
  }

  function applyTerminalEdits(existing: string, chunk: string): string {
    if (!chunk) return existing;

    // Fast-path: append-only.
    if (!chunk.includes("\r") && !chunk.includes("\b")) return `${existing}${chunk}`;

    let out = existing;
    let lineStart = out.lastIndexOf("\n") + 1;
    for (let i = 0; i < chunk.length; i++) {
      const ch = chunk[i]!;
      if (ch === "\n") {
        out += "\n";
        lineStart = out.length;
        continue;
      }
      if (ch === "\r") {
        out = out.slice(0, lineStart);
        continue;
      }
      if (ch === "\b") {
        if (out.length > lineStart) out = out.slice(0, out.length - 1);
        continue;
      }
      out += ch;
    }
    return out;
  }

  type TerminalState = {
    rows: number;
    cols: number;
    cursorRow: number;
    cursorCol: number;
    savedRow: number;
    savedCol: number;
    grid: string[][];
    escMode: "none" | "esc" | "csi" | "osc" | "dcs";
    escBuf: string;
    oscEsc: boolean;
    dcsEsc: boolean;
  };

  function looksLikeTuiAnsi(s: string): boolean {
    if (!s) return false;
    if (!s.includes("\x1b[")) return false;
    // Treat cursor movement / clear screen / clear line as "TUI-ish".
    if (/\x1b\[[0-?]*[ -/]*[HJfKABCDGd]/.test(s)) return true;
    // alt screen / private modes (e.g. ?1049h)
    if (/\x1b\[\?[0-9;]*[hl]/.test(s)) return true;
    return false;
  }

  function newTerminalState(rows = 24, cols = 80): TerminalState {
    const grid = Array.from({ length: rows }, () => Array.from({ length: cols }, () => " "));
    return {
      rows,
      cols,
      cursorRow: 0,
      cursorCol: 0,
      savedRow: 0,
      savedCol: 0,
      grid,
      escMode: "none",
      escBuf: "",
      oscEsc: false,
      dcsEsc: false,
    };
  }

  function termClampCursor(t: TerminalState) {
    if (t.cursorRow < 0) t.cursorRow = 0;
    if (t.cursorCol < 0) t.cursorCol = 0;
    if (t.cursorRow >= t.rows) t.cursorRow = t.rows - 1;
    if (t.cursorCol >= t.cols) t.cursorCol = t.cols - 1;
  }

  function termBlankLine(cols: number): string[] {
    return Array.from({ length: cols }, () => " ");
  }

  function termScrollUp(t: TerminalState, n = 1) {
    const count = Math.max(1, n | 0);
    for (let i = 0; i < count; i++) {
      t.grid.shift();
      t.grid.push(termBlankLine(t.cols));
    }
    t.cursorRow = t.rows - 1;
    termClampCursor(t);
  }

  function termClearAll(t: TerminalState) {
    t.grid = Array.from({ length: t.rows }, () => termBlankLine(t.cols));
    t.cursorRow = 0;
    t.cursorCol = 0;
    t.savedRow = 0;
    t.savedCol = 0;
    termClampCursor(t);
  }

  function termEraseInLine(t: TerminalState, mode: number) {
    const line = t.grid[t.cursorRow];
    if (!line) return;
    const m = mode | 0;
    let start = 0;
    let end = t.cols;
    if (m === 0) {
      start = t.cursorCol;
      end = t.cols;
    } else if (m === 1) {
      start = 0;
      end = Math.min(t.cols, t.cursorCol + 1);
    } else if (m === 2) {
      start = 0;
      end = t.cols;
    } else {
      return;
    }
    for (let i = start; i < end; i++) line[i] = " ";
  }

  function termEraseInDisplay(t: TerminalState, mode: number) {
    const m = mode | 0;
    if (m === 2) {
      termClearAll(t);
      return;
    }
    if (m === 0) {
      // cursor -> end
      termEraseInLine(t, 0);
      for (let r = t.cursorRow + 1; r < t.rows; r++) {
        const line = t.grid[r];
        if (!line) continue;
        for (let c = 0; c < t.cols; c++) line[c] = " ";
      }
      return;
    }
    if (m === 1) {
      // start -> cursor
      for (let r = 0; r < t.cursorRow; r++) {
        const line = t.grid[r];
        if (!line) continue;
        for (let c = 0; c < t.cols; c++) line[c] = " ";
      }
      termEraseInLine(t, 1);
    }
  }

  function termDeleteChars(t: TerminalState, n: number) {
    const line = t.grid[t.cursorRow];
    if (!line) return;
    const count = Math.max(1, n | 0);
    for (let i = 0; i < count; i++) {
      if (t.cursorCol >= t.cols) break;
      line.splice(t.cursorCol, 1);
      line.push(" ");
    }
  }

  function termInsertBlanks(t: TerminalState, n: number) {
    const line = t.grid[t.cursorRow];
    if (!line) return;
    const count = Math.max(1, n | 0);
    for (let i = 0; i < count; i++) {
      if (t.cursorCol >= t.cols) break;
      line.splice(t.cursorCol, 0, " ");
      line.pop();
    }
  }

  function termEraseChars(t: TerminalState, n: number) {
    const line = t.grid[t.cursorRow];
    if (!line) return;
    const count = Math.max(1, n | 0);
    for (let i = 0; i < count; i++) {
      const c = t.cursorCol + i;
      if (c >= t.cols) break;
      line[c] = " ";
    }
  }

  function termHandleCsi(t: TerminalState, buf: string) {
    if (!buf) return;
    const final = buf[buf.length - 1] ?? "";
    let body = buf.slice(0, -1);
    let isPrivate = false;
    if (body.startsWith("?")) {
      isPrivate = true;
      body = body.slice(1);
    }
    const params = body
      .split(";")
      .map((p) => (p.trim() === "" ? NaN : Number.parseInt(p, 10)))
      .map((n) => (Number.isFinite(n) ? n : NaN));
    const p0 = params[0];
    const p1 = params[1];

    const n1 = Number.isFinite(p0) && p0 ? p0 : 1;

    switch (final) {
      case "H":
      case "f": {
        const row = (Number.isFinite(p0) && p0 ? p0 : 1) - 1;
        const col = (Number.isFinite(p1) && p1 ? p1 : 1) - 1;
        t.cursorRow = Math.max(0, Math.min(t.rows - 1, row));
        t.cursorCol = Math.max(0, Math.min(t.cols - 1, col));
        return;
      }
      case "A":
        t.cursorRow -= n1;
        termClampCursor(t);
        return;
      case "B":
        t.cursorRow += n1;
        termClampCursor(t);
        return;
      case "C":
        t.cursorCol += n1;
        termClampCursor(t);
        return;
      case "D":
        t.cursorCol -= n1;
        termClampCursor(t);
        return;
      case "G": {
        const col = (Number.isFinite(p0) && p0 ? p0 : 1) - 1;
        t.cursorCol = Math.max(0, Math.min(t.cols - 1, col));
        return;
      }
      case "d": {
        const row = (Number.isFinite(p0) && p0 ? p0 : 1) - 1;
        t.cursorRow = Math.max(0, Math.min(t.rows - 1, row));
        return;
      }
      case "J":
        termEraseInDisplay(t, Number.isFinite(p0) ? (p0 as number) : 0);
        return;
      case "K":
        termEraseInLine(t, Number.isFinite(p0) ? (p0 as number) : 0);
        return;
      case "s":
        t.savedRow = t.cursorRow;
        t.savedCol = t.cursorCol;
        return;
      case "u":
        t.cursorRow = t.savedRow;
        t.cursorCol = t.savedCol;
        termClampCursor(t);
        return;
      case "P":
        termDeleteChars(t, n1);
        return;
      case "@":
        termInsertBlanks(t, n1);
        return;
      case "X":
        termEraseChars(t, n1);
        return;
      case "S":
        termScrollUp(t, n1);
        return;
      case "m":
        // styles/colors: ignore
        return;
      case "h":
      case "l":
        if (isPrivate) {
          const ps = params.filter((n) => Number.isFinite(n)) as number[];
          if (ps.includes(1049) || ps.includes(1047) || ps.includes(47)) {
            termClearAll(t);
          }
        }
        return;
      default:
        return;
    }
  }

  function termWrite(t: TerminalState, input: string) {
    if (!input) return;
    for (let i = 0; i < input.length; i++) {
      const ch = input[i]!;

      if (t.escMode === "none") {
        if (ch === "\x1b") {
          t.escMode = "esc";
          continue;
        }
        if (ch === "\n") {
          t.cursorRow += 1;
          t.cursorCol = 0;
          if (t.cursorRow >= t.rows) termScrollUp(t, 1);
          continue;
        }
        if (ch === "\r") {
          t.cursorCol = 0;
          continue;
        }
        if (ch === "\b") {
          if (t.cursorCol > 0) t.cursorCol -= 1;
          continue;
        }
        if (ch === "\t") {
          const next = ((Math.floor(t.cursorCol / 8) + 1) * 8) | 0;
          while (t.cursorCol < Math.min(t.cols, next)) {
            const line = t.grid[t.cursorRow];
            if (line) line[t.cursorCol] = " ";
            t.cursorCol += 1;
          }
          termClampCursor(t);
          continue;
        }

        const line = t.grid[t.cursorRow];
        if (line && t.cursorCol >= 0 && t.cursorCol < t.cols) line[t.cursorCol] = ch;
        t.cursorCol += 1;
        if (t.cursorCol >= t.cols) {
          t.cursorCol = 0;
          t.cursorRow += 1;
          if (t.cursorRow >= t.rows) termScrollUp(t, 1);
        }
        continue;
      }

      if (t.escMode === "esc") {
        if (ch === "[") {
          t.escMode = "csi";
          t.escBuf = "";
          continue;
        }
        if (ch === "]") {
          t.escMode = "osc";
          t.escBuf = "";
          t.oscEsc = false;
          continue;
        }
        if (ch === "P") {
          t.escMode = "dcs";
          t.escBuf = "";
          t.dcsEsc = false;
          continue;
        }
        if (ch === "7") {
          t.savedRow = t.cursorRow;
          t.savedCol = t.cursorCol;
          t.escMode = "none";
          continue;
        }
        if (ch === "8") {
          t.cursorRow = t.savedRow;
          t.cursorCol = t.savedCol;
          termClampCursor(t);
          t.escMode = "none";
          continue;
        }
        t.escMode = "none";
        continue;
      }

      if (t.escMode === "csi") {
        t.escBuf += ch;
        if (ch >= "@" && ch <= "~") {
          termHandleCsi(t, t.escBuf);
          t.escMode = "none";
          t.escBuf = "";
        } else if (t.escBuf.length > 64) {
          t.escMode = "none";
          t.escBuf = "";
        }
        continue;
      }

      if (t.escMode === "osc") {
        if (ch === "\x07") {
          t.escMode = "none";
          t.escBuf = "";
          t.oscEsc = false;
          continue;
        }
        if (t.oscEsc) {
          if (ch === "\\") {
            t.escMode = "none";
            t.escBuf = "";
            t.oscEsc = false;
            continue;
          }
          t.oscEsc = false;
        }
        if (ch === "\x1b") {
          t.oscEsc = true;
          continue;
        }
        continue;
      }

      if (t.escMode === "dcs") {
        if (t.dcsEsc) {
          if (ch === "\\") {
            t.escMode = "none";
            t.escBuf = "";
            t.dcsEsc = false;
            continue;
          }
          t.dcsEsc = false;
        }
        if (ch === "\x1b") {
          t.dcsEsc = true;
          continue;
        }
        continue;
      }
    }
  }

  function termToString(t: TerminalState): string {
    return t.grid
      .map((line) => line.join("").replace(/\s+$/g, ""))
      .join("\n");
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

  async function fetchWithTimeout(url: string, init: RequestInit = {}, timeoutMs = 15_000): Promise<Response> {
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
      const raw0 = dataString(env, "text") ?? "";
      const raw = codexStructuredEventText(raw0) ?? raw0;
      return {
        key: `${env.ts}:run.output:${env.seq ?? uuid()}`,
        ts: env.ts,
        role: "assistant",
        kind: env.type,
        actor: dataString(env, "actor"),
        text: sanitizeTerminalOutput(raw),
        data: env.data,
      };
    }
    if (env.type === "run.input") {
      return {
        key: `${env.ts}:run.input:${dataString(env, "input_id") ?? uuid()}`,
        ts: env.ts,
        role: "user",
        kind: env.type,
        actor: dataString(env, "actor"),
        text: dataString(env, "text_redacted") ?? "",
        data: env.data,
      };
    }
    if (env.type === "run.permission_requested") {
      return {
        key: `${env.ts}:run.permission_requested:${dataString(env, "request_id") ?? uuid()}`,
        ts: env.ts,
        role: "system",
        kind: env.type,
        request_id: dataString(env, "request_id"),
        text: dataString(env, "prompt") ?? "",
        data: env.data,
      };
    }
    if (env.type === "run.started" || env.type === "run.exited") {
      return {
        key: `${env.ts}:${env.type}:${env.seq ?? uuid()}`,
        ts: env.ts,
        role: "system",
        kind: env.type,
        text: env.type === "run.started" ? "run started" : "run exited",
        data: env.data,
      };
    }
    if (env.type === "tool.call") {
      return {
        key: `${env.ts}:tool.call:${dataString(env, "request_id") ?? uuid()}`,
        ts: env.ts,
        role: "system",
        kind: env.type,
        request_id: dataString(env, "request_id"),
        actor: dataString(env, "actor"),
        text: `tool.call ${dataString(env, "tool") ?? "unknown"} ${jsonTrunc(dataAny(env, "args"), 2000)}`,
        data: env.data,
      };
    }
    if (env.type === "tool.result") {
      const ok = dataBool(env, "ok") ?? false;
      const dur = isRecord(env.data) && typeof env.data["duration_ms"] === "number" ? env.data["duration_ms"] : 0;
      const base = `tool.result ${dataString(env, "tool") ?? "unknown"} ok=${ok} duration_ms=${dur}`;
      const extra = ok ? jsonTrunc(dataAny(env, "result"), 2000) : String(dataAny(env, "error") ?? "");
      return {
        key: `${env.ts}:tool.result:${dataString(env, "request_id") ?? uuid()}`,
        ts: env.ts,
        role: "system",
        kind: env.type,
        request_id: dataString(env, "request_id"),
        actor: dataString(env, "actor"),
        text: `${base} ${extra}`.trim(),
        data: env.data,
      };
    }
    return null;
  }

  function resetConnectionState() {
    status = "checking";
    events = [];
    health = null;
    outputByRun = {};
    runReadyByRun = {};
    wsSubscribedRunId = "";
    awaitingByRun = {};
    hosts = [];
    messagesByRun = {};
    outputTuiLastRenderAt = 0;
    if (outputTuiRenderTimer) {
      clearTimeout(outputTuiRenderTimer);
      outputTuiRenderTimer = null;
    }

    if (ws) {
      try {
        ws.close();
      } catch {
        // ignore
      }
      ws = null;
    }
  }

  function openAppWebSocket(nextToken: string) {
    status = "connecting";
    const nextWs = new WebSocket(`${toWsBase(apiBaseUrl)}/ws/app?token=${encodeURIComponent(nextToken)}`);
    ws = nextWs;
    nextWs.onopen = () => {
      if (ws === nextWs) {
        status = "connected";
        const runId = wsSubscribedRunId || selectedRunId;
        if (runId) subscribeToRun(runId);
      }
    };
    nextWs.onclose = () => {
      if (ws === nextWs) status = "disconnected";
    };
    nextWs.onerror = () => {
      if (ws === nextWs) status = "error";
    };
    nextWs.onmessage = (ev) => {
      try {
        if (ws !== nextWs) return;
        const msg = JSON.parse(ev.data) as WsEnvelope;

        // Drop high-volume events early to keep the UI thread responsive.
        // - Only keep `run.output` for the currently selected session.
        // - Only keep tool call/result when actively viewing messages or settings.
        if (msg.type === "run.output") {
          const sel = selectedRunId;
          if (!sel || msg.run_id !== sel) return;
        }
        if (msg.type === "tool.call" || msg.type === "tool.result") {
          const sel = selectedRunId;
          const wantsMessages =
            token && view === "sessions" && sessionDetailTab === "messages" && Boolean(sel) && msg.run_id === sel;
          const wantsSettings = token && view === "settings";
          if (!wantsMessages && !wantsSettings) return;
        }

        // Coalesce consecutive output chunks to avoid growing `wsQueue` too fast on heavy streams.
        if (msg.type === "run.output") {
          const text = dataString(msg, "text") ?? "";
          const last = wsQueue.length > 0 ? wsQueue[wsQueue.length - 1] : null;
          if (
            text &&
            last &&
            last.type === "run.output" &&
            last.run_id &&
            last.run_id === msg.run_id &&
            isRecord(last.data) &&
            typeof last.data["text"] === "string"
          ) {
            last.data["text"] = `${String(last.data["text"])}${text}`;
            scheduleWsFlush();
            return;
          }
        }

        wsQueue.push(msg);
        scheduleWsFlush();
      } catch {
        // ignore
      }
    };
  }

  async function connect() {
    lastError = "";
    try {
      resetConnectionState();

      const h = await fetchWithTimeout(`${apiBaseUrl.replace(/\/$/, "")}/health`, {}, 10_000);
      if (!h.ok) {
        const body = await h.text().catch(() => "");
        throw new Error(`health failed: ${h.status} ${body}`.trim());
      }
      health = (await h.json()) as Health;

      const l = await fetchWithTimeout(
        `${apiBaseUrl.replace(/\/$/, "")}/auth/login`,
        {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ username, password }),
        },
        15_000,
      );
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
      persistAuthPrefs();

      openAppWebSocket(token);
      void Promise.all([refreshHosts(), refreshRuns()]);
    } catch (e) {
      lastError = `${e instanceof Error ? e.message : String(e)}\nserver=${apiBaseUrl}`.trim();
      status = "error";
    }
  }

  async function resumeFromStoredToken() {
    if (!token) return;
    lastError = "";
    const savedToken = token;
    try {
      resetConnectionState();

      const h = await fetchWithTimeout(`${apiBaseUrl.replace(/\/$/, "")}/health`, {}, 10_000);
      if (!h.ok) {
        const body = await h.text().catch(() => "");
        throw new Error(`health failed: ${h.status} ${body}`.trim());
      }
      health = (await h.json()) as Health;
      view = "sessions";

      openAppWebSocket(savedToken);
      void Promise.all([refreshHosts(), refreshRuns()]);
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
    persistAuthPrefs();
  }

  async function refreshHosts() {
    if (!token) return;
    try {
      const r = await fetchWithTimeout(
        `${apiBaseUrl.replace(/\/$/, "")}/hosts`,
        {
          headers: { Authorization: `Bearer ${token}` },
        },
        20_000,
      );
      if (r.status === 401) {
        lastError = "登录已过期，请重新登录";
        setToast("登录已过期");
        disconnect();
        return;
      }
      if (r.ok) {
        hosts = (await r.json()) as HostInfo[];
        const online = hosts.filter((h) => h.online);
        if (online.length > 0 && !online.some((h) => h.id === startHostId)) startHostId = online[0].id;
        if (online.length > 0 && !online.some((h) => h.id === hostDiagHostId)) hostDiagHostId = online[0].id;
        if (startHostId.trim()) void ensureStartHostTools(startHostId, true);
      }
    } catch (e) {
      // Non-fatal: keep previous hosts to avoid "everything offline" flicker.
      const msg = e instanceof Error ? e.message : String(e);
      console.warn("refreshHosts failed", msg);
      setToast("主机列表刷新失败");
    }
  }

  async function refreshRuns() {
    if (!token) return;
    try {
      const r = await fetchWithTimeout(
        `${apiBaseUrl.replace(/\/$/, "")}/sessions/recent?limit=${DEFAULT_SESSION_LIMIT}`,
        {
          headers: { Authorization: `Bearer ${token}` },
        },
        20_000,
      );
      if (r.status === 401) {
        lastError = "登录已过期，请重新登录";
        setToast("登录已过期");
        disconnect();
        return;
      }
      if (r.ok) {
        runs = (await r.json()) as RunRow[];
        if (selectedRunId && !runs.some((x) => x.id === selectedRunId)) selectedRunId = "";
      }
    } catch (e) {
      // Non-fatal: keep previous runs; WS may still deliver incremental updates.
      const msg = e instanceof Error ? e.message : String(e);
      console.warn("refreshRuns failed", msg);
      setToast("会话列表刷新失败");
    }
  }

  async function refreshRecentSessions() {
    if (!token) return;
    const r = await fetch(`${apiBaseUrl.replace(/\/$/, "")}/sessions/recent?limit=50`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    if (r.status === 401) {
      lastError = "登录已过期，请重新登录";
      setToast("登录已过期");
      disconnect();
      return;
    }
    if (r.ok) {
      recentSessions = (await r.json()) as RunRow[];
    } else {
      recentSessions = [];
    }
  }

  async function refreshSelectedSession() {
    if (!token) return;
    await Promise.all([refreshHosts(), refreshRuns()]);
    if (selectedRunId) await loadMessages(selectedRunId);
  }

  async function loadMessages(runId: string, options?: { includeOutput?: boolean }) {
    if (!token) return;
    try {
      const expectedMode0 = currentOutputModeForRun(runId);
      const includeOutput = options?.includeOutput ?? (runId === selectedRunId ? shouldSubscribeOutput(runId) : includeOutputInMessages(runId));
      if (runId === selectedRunId && expectedMode0 === "tui") {
        xtermBackfillRunId = runId;
        xtermBackfillReady = false;
        xtermBackfillText = "";
        xtermBackfillMaxSeq = 0;
        xtermBackfillPending = [];
      }
      const query = new URLSearchParams({ limit: "200", include_output: includeOutput ? "true" : "false" });
      const r = await fetchWithTimeout(
        `${apiBaseUrl.replace(/\/$/, "")}/sessions/${encodeURIComponent(runId)}/messages?${query.toString()}`,
        {
          headers: { Authorization: `Bearer ${token}` },
        },
        20_000,
      );
      if (r.status === 401) {
        lastError = "登录已过期，请重新登录";
        setToast("登录已过期");
        disconnect();
        return;
      }
      if (!r.ok) return;
      const msgs = (await r.json()) as ChatMessageApi[];
      const mapped: ChatMessage[] = msgs.map((m) => ({
        key: String(m.id),
        ts: m.ts,
        role: m.role === "assistant" || m.role === "user" ? m.role : "system",
        kind: m.kind,
        actor: m.actor,
        request_id: m.request_id,
        text:
          m.kind === "run.output"
            ? sanitizeTerminalOutput(codexStructuredEventText(m.text) ?? m.text)
            : m.text,
        data: m.data,
      }));
      messagesByRun = { ...messagesByRun, [runId]: mapped };

      if (!includeOutput) return;

      // Populate output view from persisted messages (useful when opening a run after it started).
      // For TUI tools (Codex/Claude/etc), reconstruct a small screen snapshot instead of a giant log.
      const maxChars = 200_000;
      const rawPartsRev: string[] = [];
      let rawTotal = 0;
      for (const m of msgs) {
        // API returns newest-first.
        if (m.kind !== "run.output" || !m.text) continue;
        rawPartsRev.push(m.text);
        rawTotal += m.text.length;
        if (rawTotal >= 1_000_000) break;
      }
      const rawParts = rawPartsRev.reverse();
      const tool = runToolFor(runId);
      const isLikelyTuiTool = isLikelyTuiToolName(tool);
      const mode: "log" | "tui" =
        rawParts.some((s) => looksLikeTuiAnsi(s)) || (isLikelyTuiTool && rawParts.some((s) => s.includes("\x1b[")))
          ? "tui"
          : "log";
      if (outputModeByRun[runId] !== mode) outputModeByRun = { ...outputModeByRun, [runId]: mode };

      if (mode === "tui") {
        let maxSeq = 0;
        const parts: string[] = [];
        // API returns newest-first, so replay from oldest-first for xterm.
        for (let i = msgs.length - 1; i >= 0; i--) {
          const m = msgs[i];
          if (!m || m.kind !== "run.output" || !m.text) continue;
          parts.push(m.text);
          if (typeof m.seq === "number") maxSeq = Math.max(maxSeq, m.seq);
        }

        // Mark as "loaded" for this run so we don't refetch on every click.
        outputByRun = { ...outputByRun, [runId]: " " };

        if (runId === selectedRunId) {
          xtermBackfillRunId = runId;
          xtermBackfillText = parts.join("");
          xtermBackfillMaxSeq = maxSeq;
          xtermBackfillReady = true;
          applyXtermBackfillIfReady();
        }

        if (runId === selectedRunId) {
          outputTuiState = null;
          outputTuiRunId = "";
        }
      } else {
        let out = "";
        for (const p of rawParts) {
          out = applyTerminalEdits(out, sanitizeTerminalOutput(p));
          if (out.length > maxChars * 2) out = truncateTail(out, maxChars);
        }
        outputByRun = { ...outputByRun, [runId]: truncateTail(out, maxChars) };
        if (runId === selectedRunId) {
          outputTuiState = null;
          outputTuiRunId = "";
        }
        if (runId === selectedRunId && xtermBackfillRunId === runId) {
          xtermBackfillRunId = "";
          xtermBackfillReady = false;
          xtermBackfillText = "";
          xtermBackfillMaxSeq = 0;
          xtermBackfillPending = [];
        }
      }
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      console.warn("loadMessages failed", msg);
      setToast("消息加载失败");
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

  function subscribeToRun(runId: string) {
    wsSubscribedRunId = runId;
    wsSubscribedIncludeOutput = runId ? shouldSubscribeOutput(runId) : false;
    if (!runId) return;
    sendWs({
      type: "run.subscribe",
      ts: new Date().toISOString(),
      run_id: runId,
      data: { replace: true, include_output: wsSubscribedIncludeOutput },
    });
  }

  $: if (selectedRunId && ws && ws.readyState === WebSocket.OPEN) {
    const wantsOutput = shouldSubscribeOutput(selectedRunId);
    if (wsSubscribedRunId !== selectedRunId || wsSubscribedIncludeOutput !== wantsOutput) {
      subscribeToRun(selectedRunId);
    }
  }

  async function rpcCall(rpcType: string, data: Record<string, unknown>): Promise<WsEnvelope> {
    if (!ws || ws.readyState !== WebSocket.OPEN) throw new Error("ws not connected");
    if (!selectedRunId) throw new Error("missing run_id");
    const requestId = uuid();
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
    const requestId = uuid();
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

  async function fetchHostInfoResult(hostId: string): Promise<Record<string, unknown>> {
    const resp = await rpcCallNoRun("rpc.host.info", { host_id: hostId });
    const ok = dataBool(resp, "ok");
    if (!ok) throw new Error(dataString(resp, "error") ?? "rpc failed");
    const result = dataAny(resp, "result");
    if (!isRecord(result)) throw new Error("bad rpc result");
    return result;
  }

  async function ensureStartHostTools(hostId: string, force = false) {
    const normalized = hostId.trim();
    if (!normalized || !token || status !== "connected") return;
    if (!force && normalized in startHostToolsById) {
      syncStartToolSelectionForHost(normalized);
      return;
    }
    startHostToolsLoadingById = { ...startHostToolsLoadingById, [normalized]: true };
    try {
      const result = await fetchHostInfoResult(normalized);
      const tools = parseHostToolStatuses(result["tools"]);
      startHostToolsById = { ...startHostToolsById, [normalized]: tools };
      syncStartToolSelectionForHost(normalized);
    } catch (e) {
      console.warn("ensureStartHostTools failed", normalized, e instanceof Error ? e.message : String(e));
      if (!(normalized in startHostToolsById)) {
        startHostToolsById = { ...startHostToolsById, [normalized]: [] };
      }
    } finally {
      startHostToolsLoadingById = { ...startHostToolsLoadingById, [normalized]: false };
    }
  }

  async function startRun() {
    startError = "";
    try {
      const tool = startTool.trim();
      const availableOptions = dynamicStartToolOptionsForHost(startHostId);
      if (!availableOptions.some((option) => option.value === tool)) {
        throw new Error(`unsupported tool: ${tool}`);
      }
      const data: Record<string, unknown> = {
        host_id: startHostId.trim(),
        tool,
        cmd: startCmd.trim(),
        cwd: startCwd.trim() ? startCwd.trim() : null,
      };
      if (tool === "opencode" && startOpencodeModel.trim()) {
        data.model = startOpencodeModel.trim();
      }
      if (tool === "opencode" && startOpencodeSessionId && startOpencodeSessionId.trim()) {
        data.opencode_session_id = startOpencodeSessionId.trim();
      }
      const resp = await rpcCallNoRun("rpc.run.start", data);
      const ok = dataBool(resp, "ok");
      if (!ok) throw new Error(dataString(resp, "error") ?? "rpc failed");
      const result = dataAny(resp, "result");
      const runIdFromResult = isRecord(result) && typeof result.run_id === "string" ? result.run_id : "";
      const runId = resp.run_id ?? runIdFromResult;
      if (!runId) {
        throw new Error("rpc.run.start succeeded but run_id missing");
      }
      if (startCwd.trim()) rememberStartCwd(startHostId, startCwd);
      await refreshRuns();
      view = "sessions";
      await selectSession(runId);
      setToast(`已启动 ${tool}`);
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      startError = humanizeStartError(message);
    }
  }

  async function handleSessionSelect(event: CustomEvent<string>) {
    const sessionId = event.detail;
    if (sessionId) {
      showSessionSelector = false;
      await selectSession(sessionId);
      setToast(`已切换到会话 ${sessionId}`);
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
    await ensureStartHostTools(hostDiagHostId, true);
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

  async function fetchServerLogs() {
    serverLogsError = "";
    serverLogsPath = "";
    serverLogs = "";
    serverLogsTruncated = false;
    if (!token) return;
    try {
      const lines = Number(serverLogsLines);
      const maxBytes = Number(serverLogsMaxBytes);
      const lines2 = (Number.isFinite(lines) ? lines : 200).toString();
      const maxBytes2 = (Number.isFinite(maxBytes) ? maxBytes : 200000).toString();
      const url = `${apiBaseUrl.replace(/\/$/, "")}/server/logs/tail?lines=${encodeURIComponent(lines2)}&max_bytes=${encodeURIComponent(maxBytes2)}`;
      const r = await fetchWithTimeout(
        url,
        {
          headers: { Authorization: `Bearer ${token}` },
        },
        20_000,
      );
      if (r.status === 401) {
        lastError = "登录已过期，请重新登录";
        setToast("登录已过期");
        disconnect();
        return;
      }
      const bodyText = await r.text().catch(() => "");
      if (!r.ok) throw new Error(`server.logs.tail failed: ${r.status} ${bodyText}`.trim());
      const parsed = JSON.parse(bodyText) as { path?: unknown; text?: unknown; truncated?: unknown };
      serverLogsPath = typeof parsed.path === "string" ? parsed.path : "";
      serverLogs = typeof parsed.text === "string" ? parsed.text : "";
      serverLogsTruncated = Boolean(parsed.truncated);
    } catch (e) {
      serverLogsError = e instanceof Error ? e.message : String(e);
    }
  }

  function normalizeTerminalInput(text: string): string {
    // For PTY-based interactive tools, the Enter key is typically '\r' (carriage return).
    // Browser inputs naturally use '\n', so normalize to '\r' to match terminal behavior.
    // Also collapse CRLF to CR to avoid sending two line breaks.
    return (text ?? "").replace(/\r\n/g, "\r").replace(/\n/g, "\r");
  }

  function normalizeRunInput(text: string, runId: string): string {
    const raw = text ?? "";
    const run = runs.find((item) => item.id === runId) ?? null;
    const mode = outputModeByRun[runId] ?? "log";
    const isOpencodeStructured = run?.tool === "opencode" && mode === "log";
    if (isOpencodeStructured) {
      return raw.replace(/\r\n/g, "\n").replace(/\r/g, "\n");
    }
    return normalizeTerminalInput(raw);
  }

  function codexStructuredEventText(raw: string): string | null {
    // Codex structured mode (mcp-server) emits notifications like:
    // {"jsonrpc":"2.0","method":"codex/event","params":{...}}
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
    } catch {
      return null;
    }
  }

  function sendStdin(text: string) {
    if (!selectedRunId) return;
    const chunk = normalizeTerminalInput(text);
    if (!chunk) return;
    sendWs({
      type: "run.send_stdin",
      ts: new Date().toISOString(),
      run_id: selectedRunId,
      data: { actor: "web", text: chunk },
    });
  }

  function flushStdinBuf() {
    if (stdinBufTimer) {
      clearTimeout(stdinBufTimer);
      stdinBufTimer = null;
    }
    const runId = stdinBufRunId;
    const chunk = stdinBuf;
    stdinBufRunId = "";
    stdinBuf = "";
    if (!chunk || !runId) return;
    if (runId !== selectedRunId) return;
    sendStdin(chunk);
  }

  function queueStdin(text: string) {
    const runId = selectedRunId;
    if (!runId) return;
    if (stdinBufRunId && stdinBufRunId !== runId) {
      stdinBuf = "";
      stdinBufTimer && clearTimeout(stdinBufTimer);
      stdinBufTimer = null;
    }
    stdinBufRunId = runId;
    stdinBuf += text ?? "";
    if (stdinBuf.length >= 4096) {
      flushStdinBuf();
      return;
    }
    if (!stdinBufTimer) {
      stdinBufTimer = setTimeout(flushStdinBuf, 20);
    }
  }

  function scheduleResize(runId: string, cols: number, rows: number) {
    const c = Math.max(2, Math.min(500, cols | 0));
    const r = Math.max(1, Math.min(200, rows | 0));
    const key = `${runId}:${c}x${r}`;
    if (key === xtermLastResizeKey) return;
    xtermResizePending = { runId, cols: c, rows: r };
    if (xtermResizeTimer) return;
    xtermResizeTimer = setTimeout(() => {
      xtermResizeTimer = null;
      const p = xtermResizePending;
      xtermResizePending = null;
      if (!p) return;
      if (!selectedRunId || p.runId !== selectedRunId) return;
      xtermLastResizeKey = `${p.runId}:${p.cols}x${p.rows}`;
      sendWs({
        type: "run.resize",
        ts: new Date().toISOString(),
        run_id: p.runId,
        data: { actor: "web", cols: p.cols, rows: p.rows },
      });
    }, 200);
  }

  function applyXtermBackfillIfReady() {
    const runId = selectedRunId;
    if (!runId) return;
    if (!xtermRef || xtermRunId !== runId) return;

    // Apply backfill snapshot (from API) once per selection.
    if (xtermBackfillReady && xtermBackfillRunId === runId) {
      xtermRef.reset();
      if (xtermBackfillText) xtermRef.write(xtermBackfillText);
      xtermAppliedSeq = Math.max(xtermAppliedSeq, xtermBackfillMaxSeq);

      const pending = xtermBackfillPending;
      xtermBackfillPending = [];
      xtermBackfillText = "";
      xtermBackfillMaxSeq = 0;
      xtermBackfillRunId = "";
      xtermBackfillReady = false;

      for (const p of pending) {
        if (typeof p.seq === "number" && p.seq <= xtermAppliedSeq) continue;
        xtermRef.write(p.text);
        if (typeof p.seq === "number") xtermAppliedSeq = Math.max(xtermAppliedSeq, p.seq);
      }
    }

    // Flush any output received before xterm finished mounting.
    if (xtermPreReady) {
      xtermRef.write(xtermPreReady);
      xtermAppliedSeq = Math.max(xtermAppliedSeq, xtermPreReadyMaxSeq);
      xtermPreReady = "";
      xtermPreReadyMaxSeq = 0;
    }
  }

  function sendInput(text: string) {
    if (!selectedRunId) return;
    const normalized = normalizeRunInput(text, selectedRunId);
    sendWs({
      type: "run.send_input",
      ts: new Date().toISOString(),
      run_id: selectedRunId,
      data: { input_id: uuid(), actor: "web", text: normalized },
    });
  }
  function sendChatInput() {
    if (!selectedRunId || status !== "connected") return;
    const raw = chatInputText;
    if (!raw.trim()) return;

    const a = selectedAwaiting;
    if (a) {
      // If the run is awaiting permission, don't blindly send arbitrary text.
      if (awaitingIsApproval(a) || awaitingWantsYesNo(a)) {
        const t = raw.trim().toLowerCase();
        const yes = t === "y" || t === "yes" || t === "approve" || t === "ok" || t === "同意" || t === "继续";
        const no = t === "n" || t === "no" || t === "deny" || t === "拒绝";
        if (yes) {
          sendDecision("approve");
          chatInputText = "";
          void tick().then(() => chatInputEl?.focus());
          return;
        }
        if (no) {
          sendDecision("deny");
          chatInputText = "";
          void tick().then(() => chatInputEl?.focus());
          return;
        }

        setToast("当前需要先确认（Proceed?），请在置顶卡片点“同意/拒绝”或输入 y/n");
        // Surface the approval modal on desktop to reduce confusion.
        approvalModalOpen = true;
        return;
      }

      // If the run is awaiting a prompt, route the main input to that prompt.
      if (awaitingIsPrompt(a)) {
        let text = raw;
        if (!text.includes("\n") && !text.endsWith("\r") && !text.endsWith("\n")) text += "\n";
        sendInput(text);
        chatInputText = "";
        void tick().then(() => chatInputEl?.focus());
        return;
      }
    }

    let text = raw;
    if (!text.includes("\n") && !text.endsWith("\r") && !text.endsWith("\n")) text += "\n";
    sendInput(text);
    chatInputText = "";
    void tick().then(() => chatInputEl?.focus());
  }

  function handleChatInputKeydown(ev: KeyboardEvent) {
    if (ev.key !== "Enter") return;
    if ((ev as unknown as { isComposing?: boolean }).isComposing) return;
    if (ev.shiftKey) return;
    ev.preventDefault();
    sendChatInput();
  }

  async function openInputModal(prefill = "") {
    inputModalText = prefill;
    inputModalOpen = true;
    await tick();
    inputModalEl?.focus();
  }

  function closeInputModal() {
    inputModalOpen = false;
    inputModalText = "";
  }

  function sendQuickInput(text: string) {
    sendInput(text);
    closeInputModal();
  }

  function sendInputModalText() {
    let text = inputModalText;
    if (text && !text.includes("\n") && !text.endsWith("\r") && !text.endsWith("\n")) text += "\n";
    sendInput(text);
    closeInputModal();
  }

  function sendDecision(decision: string) {
    if (!selectedRunId) return;
    const reqId = selectedAwaiting?.request_id;
    if (!reqId) {
      sendInput(decision === "approve" ? "y\n" : "n\n");
      return;
    }

    const data: Record<string, unknown> = { request_id: reqId, actor: "web" };

    if (decision === "approve") {
      if (approvalForSession) {
        data["decision"] = "approve_for_session";
        if (selectedAwaiting?.op_tool) {
          data["allow_tools"] = [selectedAwaiting.op_tool];
        }
      }

      if (selectedAwaiting?.questions !== undefined && selectedAwaiting?.questions !== null) {
        const raw = (approvalAnswersJson ?? "").trim();
        if (!raw) {
          setToast(`需要填写 answers（JSON）`);
          return;
        }
        try {
          data["answers"] = JSON.parse(raw) as unknown;
        } catch {
          setToast(`answers JSON 解析失败`);
          return;
        }
      }
    }

    sendWs({
      type: decision === "approve" ? "run.permission.approve" : "run.permission.deny",
      ts: new Date().toISOString(),
      run_id: selectedRunId,
      data,
    });
  }

  function sendStop(signal: string = "term") {
    if (!selectedRunId) return;
    sendWs({
      type: "run.stop",
      ts: new Date().toISOString(),
      run_id: selectedRunId,
      data: { signal },
    });
  }

  $: selectedRun = runs.find((r) => r.id === selectedRunId) ?? null;
  $: awaitingRuns = runs.filter((r) => r.status === "awaiting_input" || r.status === "awaiting_approval");
  $: selectedRunReady = selectedRunId ? runReadyByRun[selectedRunId] ?? true : true;
  $: selectedOutput = selectedRunId ? outputByRun[selectedRunId] ?? "" : "";
  $: selectedAwaiting = selectedRunId ? awaitingByRun[selectedRunId] ?? awaitingFromRunRow(selectedRun) : null;
  $: selectedMessages = selectedRunId ? messagesByRun[selectedRunId] ?? [] : [];
  $: hostsById = Object.fromEntries(hosts.map((h) => [h.id, h] as const)) as Record<string, HostInfo>;
  $: currentStartToolOptions = dynamicStartToolOptionsForHost(startHostId);
  $: currentStartToolStatuses = startHostToolsById[startHostId] ?? null;
  $: currentStartHostToolsLoading = Boolean(startHostToolsLoadingById[startHostId]);
  $: currentStartOpencodeModels = currentOpencodeToolStatus()?.models?.slice() ?? [];
  $: currentStartOpencodeDefaultModel = currentOpencodeToolStatus()?.default_model?.trim() ?? "";
  $: currentStartOpencodeModelsError = currentOpencodeToolStatus()?.models_error?.trim() ?? "";
  $: currentStartOpencodeModelsNote = currentOpencodeToolStatus()?.models_note?.trim() ?? "";
  $: if (startHostId && currentStartToolStatuses) {
    syncStartOpencodeModelForHost();
  }
  $: if (token && status === "connected" && startHostId.trim()) {
    void ensureStartHostTools(startHostId);
  }
  $: if (token && status === "connected" && view === "start" && startHostId.trim()) {
    const refreshKey = `${view}:${startHostId}:${hostsById[startHostId]?.online ? "online" : "offline"}`;
    if (refreshKey !== lastStartToolsForceRefreshKey) {
      lastStartToolsForceRefreshKey = refreshKey;
      void ensureStartHostTools(startHostId, true);
    }
  }

  // Keep card footer + modal drafts consistent for the same request.
  $: {
    const a = selectedAwaiting;
    if (selectedRunId && a && awaitingIsApproval(a)) {
      const key = `${selectedRunId}:${a.request_id ?? ""}`;
      if (key !== approvalDraftKey) {
        approvalForSession = false;
        approvalAnswersJson = "";
        approvalDraftKey = key;
      }
    } else {
      approvalForSession = false;
      approvalAnswersJson = "";
      approvalDraftKey = "";
    }
  }

  $: {
    const a = selectedAwaiting;
    if (selectedRunId && a && awaitingIsPrompt(a)) {
      const key = `${selectedRunId}:${a.request_id ?? ""}`;
      if (key !== awaitingDraftKey) {
        inlineAwaitingText = "";
        awaitingDraftKey = key;
      }
    } else {
      inlineAwaitingText = "";
      awaitingDraftKey = "";
    }
  }

  $: {
    const a = selectedAwaiting;
    if (selectedRunId && a && awaitingIsApproval(a) && !approvalModalOpen) {
      const key = (a.request_id ?? a.op_tool ?? "").trim();
      if (key && lastSeenApprovalRequest[selectedRunId] !== key) {
        lastSeenApprovalRequest = { ...lastSeenApprovalRequest, [selectedRunId]: key };
        approvalModalShowArgs = false;
        approvalModalOpen = true;
      }
    }
  }

  $: {
    const a = selectedAwaiting;
    if (selectedRunId && a && awaitingIsPrompt(a) && !inputModalOpen) {
      const key = (a.request_id ?? "").trim();
      if (key && lastSeenPromptRequest[selectedRunId] !== key) {
        lastSeenPromptRequest = { ...lastSeenPromptRequest, [selectedRunId]: key };
        openInputModal("");
      }
    }
  }

  $: if (approvalModalOpen && (!selectedAwaiting || !awaitingIsApproval(selectedAwaiting))) {
    approvalModalOpen = false;
    approvalModalShowArgs = false;
  }
  $: filteredRuns = (() => {
    const q = sessionSearch.trim().toLowerCase();
    if (!q) return runs;
    return runs.filter((r) => {
      const title = sessionTitle(r);
      const summary = sessionSummary(r);
      const haystack = (title || summary ? `${title} ${summary}` : r.id).toLowerCase();
      return haystack.includes(q);
    });
  })();
  $: runGroups = (() => {
    const map = new Map<string, RunRow[]>();
    for (const r of filteredRuns) {
      const arr = map.get(r.host_id) ?? [];
      arr.push(r);
      map.set(r.host_id, arr);
    }
    const groups = Array.from(map.entries()).map(([hostId, items]) => {
      const h = hostsById[hostId] ?? null;
      items.sort((a, b) => compareTsDesc(a.last_active_at ?? a.started_at, b.last_active_at ?? b.started_at));
      return {
        host_id: hostId,
        host: h,
        display_name: hostDisplayName(h, hostId),
        online: Boolean(h?.online),
        last_seen_at: h?.last_seen_at ?? null,
        sessions: items,
      };
    });
    groups.sort((a, b) => {
      if (a.online !== b.online) return a.online ? -1 : 1;
      return a.display_name.localeCompare(b.display_name);
    });
    return groups;
  })();

  $: selectedOutputMode = selectedRunId ? outputModeByRun[selectedRunId] ?? "log" : "log";
  $: outputDisplayText =
    sessionDetailTab === "output" && selectedRunId
      ? selectedOutputMode === "tui"
        ? selectedOutput
        : tailLines(selectedOutput, outputBufferLines)
      : "";
  $: outputLines = outputSearchActive ? outputDisplayText.split(/\r?\n/) : [];
  $: outputSearchMatches = outputSearchActive ? computeOutputMatches(outputLines, outputSearchActive, selectedRunId) : [];
  $: if (outputSearchMatches.length === 0) outputSearchCursor = 0;
  $: if (outputSearchCursor >= outputSearchMatches.length) outputSearchCursor = 0;
  $: outputHtml = outputSearchActive ? renderOutputHtml(outputLines, outputSearchMatches, outputSearchCursor) : "";
  $: if (sessionDetailTab === "output" && outputAutoScroll) scheduleOutputScrollToBottom();

  $: uiBlocks = (() => {
    const msgs = selectedMessages ?? [];
    const tool = selectedRunId ? (runs.find((r) => r.id === selectedRunId)?.tool ?? "") : "";
    const pinnedRequestId = selectedAwaiting?.request_id ?? null;
    return reduceToBlocks(msgs, { runTool: tool, pinnedRequestId, outputMode: selectedOutputMode });
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
          id: typeof x!.id === "string" ? x!.id : uuid(),
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
    const next = [{ id: uuid(), text: t, done: false, created_at: new Date().toISOString() }, ...todos];
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

  $: if (!selectedRunId) {
    todoSuggestions = [];
  } else {
    if (todoSuggestionsTimer) clearTimeout(todoSuggestionsTimer);
    const text = outputDisplayText;
    const snapshot = todos;
    todoSuggestionsTimer = setTimeout(() => {
      todoSuggestions = extractTodoSuggestions(text).filter((s) => !snapshot.some((t) => t.text === s));
    }, 800);
  }

  $: if (selectedRunId) {
    todos = loadTodos(selectedRunId);
  } else {
    todos = [];
  }

  onMount(() => {
    let stopped = false;
    (async () => {
      if (!token) return;
      await resumeFromStoredToken();
      if (stopped) return;
      if (!token && rememberPassword && password) await connect();
    })();
    return () => {
      stopped = true;
    };
  });

  // Best-effort polling while disconnected (helps mobile background/resume).
  onMount(() => {
    let stopped = false;
    let inFlight = false;
    const timer = setInterval(async () => {
      if (stopped) return;
      if (!token) return;
      if (status === "connected") return;
      if (status === "checking" || status === "connecting") return;
      if (inFlight) return;
      inFlight = true;
      try {
        await Promise.all([refreshHosts(), refreshRuns()]);
      } finally {
        inFlight = false;
      }
    }, 10_000);
    return () => {
      stopped = true;
      clearInterval(timer);
    };
  });

  onMount(() => {
    if (typeof window !== "undefined") {
      const mq = window.matchMedia("(max-width: 640px)");
      const apply = () => {
        isMobile = mq.matches;
      };
      apply();
      mq.addEventListener("change", apply);
      return () => mq.removeEventListener("change", apply);
    }
    return;
  });

  onMount(() => {
    const onResize = () => updateOutputBufferLines();
    window.addEventListener("resize", onResize);
    return () => {
      window.removeEventListener("resize", onResize);
      if (toastTimer) clearTimeout(toastTimer);
      if (todoSuggestionsTimer) clearTimeout(todoSuggestionsTimer);
    };
  });
</script>

<main>
  <TopBar {health} {apiBaseUrl} {uiVersion} {status} {connLabel} {token} {username} />

  <NavBar {view} {token} onSetView={(v) => (view = v)} />

  {#if !token}
  <LoginForm
    {useCustomServer}
    bind:customBaseUrl={customBaseUrl}
    {apiBaseUrl}
    bind:username={username}
    bind:password={password}
    bind:keepSignedIn={keepSignedIn}
    bind:rememberPassword={rememberPassword}
    {loginBusy}
    {lastError}
    {isProbablyInsecureUrl}
    onConnect={connect}
    onPersistServerPrefs={persistServerPrefs}
    onPersistAuthPrefs={persistAuthPrefs}
  />
  {/if}

  <SettingsPanel
    {apiBaseUrl}
    {health}
    {uiVersion}
    {useCustomServer}
    bind:customBaseUrl={customBaseUrl}
    {isProbablyInsecureUrl}
    {username}
    {keepSignedIn}
    {rememberPassword}
    bind:password={password}
    {status}
    {token}
    bind:serverLogsLines={serverLogsLines}
    bind:serverLogsMaxBytes={serverLogsMaxBytes}
    {serverLogs}
    {serverLogsPath}
    {serverLogsTruncated}
    {serverLogsError}
    onDisconnect={disconnect}
    onPersistServerPrefs={persistServerPrefs}
    onPersistAuthPrefs={persistAuthPrefs}
    onFetchServerLogs={fetchServerLogs}
  />

  {#if token && view === "sessions"}
  <div class="sessions-layout" class:mobile-detail-open={isMobile && Boolean(selectedRunId)}>
  <SessionList
    {token}
    {selectedRunId}
    {sessionSearch}
    apiBaseUrl={apiBaseUrl}
    {runGroups}
    {statusLabel}
    {sessionTitle}
    {sessionSummary}
    {formatRelativeTime}
    onSelectSession={selectSession}
    onRefresh={async () => { await Promise.all([refreshHosts(), refreshRuns()]); }}
    onSessionSelect={handleSessionSelect}
  />

  <SessionDetail
    {selectedRun}
    {selectedRunId}
    {isMobile}
    {token}
    {status}
    {sessionDetailTab}
    {statusLabel}
    hostsById={hostsById}
    {sessionTitle}
    {formatRelativeTime}
    {selectedAwaiting}
    {approvalForSession}
    bind:approvalAnswersJson={approvalAnswersJson}
    {awaitingIsApproval}
    {awaitingIsPrompt}
    {awaitingWantsYesNo}
    bind:chatInputText={chatInputText}
    {uiBlocks}
    {renderMarkdownBasic}
    {formatAbsTime}
    {copyText}
    {selectedOutputMode}
    {selectedOutput}
    {tailLines}
    {selectedRunReady}
    {outputAutoScroll}
    bind:outputSearchText={outputSearchText}
    {outputSearchMatches}
    {outputSearchCursor}
    {outputSearchActive}
    {outputHtml}
    {outputDisplayText}
    {outputIsAtBottom}
    {xtermRef}
    {xtermRunId}
    onBack={() => selectedRunId = ""}
    onOpenApprovalModal={() => { approvalModalShowArgs = false; approvalModalOpen = true; }}
    onOpenStopConfirm={() => stopConfirmOpen = true}
    onSendStop={sendStop}
    onSwitchToOutputTabAction={async () => {
      sessionDetailTab = "output";
      if (selectedRunId) {
        subscribeToRun(selectedRunId);
        void loadMessages(selectedRunId, { includeOutput: true });
      }
      await focusOutputSearch();
    }}
    onSwitchToMessagesTab={() => {
      sessionDetailTab = "messages";
      if (selectedRunId) {
        subscribeToRun(selectedRunId);
        void loadMessages(selectedRunId, { includeOutput: includeOutputInMessages(selectedRunId) });
      }
    }}
    onRefreshMessages={() => selectedRunId && loadMessages(selectedRunId)}
    onFocusOutputSearch={focusOutputSearch}
    onSendChatInput={sendChatInput}
    onOpenInputModal={openInputModal}
    onHandleChatInputKeydown={handleChatInputKeydown}
    onSendDecision={sendDecision}
    onToggleApprovalForSession={() => approvalForSession = !approvalForSession}
    onToggleOutputAutoScroll={toggleOutputAutoScroll}
    onQueueStdin={queueStdin}
    onRunOutputSearch={runOutputSearch}
    onPrevOutputMatch={prevOutputMatch}
    onNextOutputMatch={nextOutputMatch}
    onClearOutputSearch={clearOutputSearch}
    onCopyOutput={copyOutput}
    onJumpToLatest={jumpToLatest}
    onResumeOutputAutoScroll={resumeOutputAutoScroll}
    onOutputScroll={handleOutputScroll}
    onXtermReady={() => {
      const runId = selectedRunId;
      if (!runId) return;
      xtermRunId = runId;
      applyXtermBackfillIfReady();
    }}
    onXtermData={(e) => {
      const runId = selectedRunId;
      if (!runId || status !== "connected") return;
      queueStdin(e.detail.data);
    }}
    onXtermResize={(e) => {
      const runId = selectedRunId;
      if (!runId || status !== "connected") return;
      scheduleResize(runId, e.detail.cols, e.detail.rows);
    }}
    onSearchKeydown={handleOutputSearchKeydown}
    onResumeFromStoredToken={resumeFromStoredToken}
    onRefreshSelectedSession={refreshSelectedSession}
  />
  <TodoPanel
    {todos}
    bind:todoText={todoText}
    {todoSuggestions}
    {selectedRunId}
    onAddTodo={(text: string) => { addTodo(text); todoText = ""; }}
    onToggleTodo={toggleTodo}
    onRemoveTodo={removeTodo}
  />
  </div>
  {/if}

  <ToolsPanel
    bind:filePath={filePath}
    {fileContent}
    {fileError}
    bind:searchQuery={searchQuery}
    {searchMatches}
    {searchTruncated}
    {searchError}
    bind:gitDiffPath={gitDiffPath}
    {gitStatus}
    {gitDiff}
    {gitError}
    {selectedRunId}
    {status}
    onFetchFile={fetchFile}
    onSearchFiles={searchFiles}
    onFetchGitStatus={fetchGitStatus}
    onFetchGitDiff={fetchGitDiff}
  />
  <HostDiagnostics
    {hosts}
    bind:hostDiagHostId={hostDiagHostId}
    {hostDiagError}
    {hostInfo}
    {hostDoctor}
    {hostCapabilities}
    {hostLogs}
    bind:hostLogsLines={hostLogsLines}
    bind:hostLogsMaxBytes={hostLogsMaxBytes}
    {status}
    {token}
    onRefreshHosts={refreshHosts}
    onFetchHostInfo={fetchHostInfo}
    onFetchHostDoctor={fetchHostDoctor}
    onFetchHostCapabilities={fetchHostCapabilities}
    onFetchHostLogs={fetchHostLogs}
  />
  <StartRun
    {hosts}
    bind:startHostId={startHostId}
    {startTool}
    {currentStartToolOptions}
    {currentStartHostToolsLoading}
    {currentStartToolStatuses}
    bind:startOpencodeModel={startOpencodeModel}
    {currentStartOpencodeModels}
    {currentStartOpencodeModelsError}
    {currentStartOpencodeDefaultModel}
    {currentStartOpencodeModelsNote}
    bind:startOpencodeSessionId={startOpencodeSessionId}
    bind:startCwd={startCwd}
    {lastSuggestedStartCwd}
    bind:startCmd={startCmd}
    {startError}
    {status}
    {token}
    onRefreshHosts={refreshHosts}
    onStartRun={startRun}
    onApplySuggestedStartCwd={() => applySuggestedStartCwd(true)}
  />

  {#if token && view === "settings"}
    <EventLog {events} {token} {view} />
  {/if}

  <InputModal
    show={inputModalOpen}
    bind:text={inputModalText}
    {selectedRunId}
    {status}
    onClose={closeInputModal}
    onSend={sendInputModalText}
    onQuickInput={sendQuickInput}
  />

  <ApprovalModal
    show={approvalModalOpen}
    {selectedRunId}
    {status}
    awaiting={selectedAwaiting}
    runTool={selectedRun?.tool ?? ""}
    {approvalForSession}
    bind:approvalAnswersJson={approvalAnswersJson}
    showArgs={approvalModalShowArgs}
    {riskForOpTool}
    onClose={() => { approvalModalOpen = false; approvalModalShowArgs = false; }}
    onSendDecision={(d) => { sendDecision(d); approvalModalOpen = false; approvalModalShowArgs = false; }}
    onToggleApprovalForSession={(v) => (approvalForSession = v)}
  />

  <StopConfirmModal
    show={stopConfirmOpen}
    runId={selectedRunId}
    {status}
    onClose={() => (stopConfirmOpen = false)}
    onStop={(signal) => { sendStop(signal); stopConfirmOpen = false; }}
  />

  <Toast text={toastText} />
</main>

<style>
  :global(:root) {
    --bg: #eef4ff;
    --surface: rgba(255, 255, 255, 0.9);
    --surface-2: rgba(246, 250, 255, 0.82);
    --surface-muted: rgba(241, 247, 255, 0.94);
    --text: #132744;
    --text-strong: #071326;
    --muted: #59708f;
    --border: rgba(15, 23, 42, 0.12);
    --border-strong: rgba(14, 165, 233, 0.34);
    --shadow-sm: 0 2px 12px rgba(2, 6, 23, 0.08);
    --shadow-md: 0 20px 48px rgba(2, 6, 23, 0.14);
    --primary: #0ea5e9;
    --primary-2: #22d3ee;
    --success: #22c55e;
    --warning: #f97316;
    --danger: #ef4444;
    --radius-lg: 18px;
    --radius-md: 13px;
    --radius-sm: 10px;
  }

  :global(body) {
    margin: 0;
    font-family: "Fira Sans", "IBM Plex Sans", "SF Pro Text", Segoe UI, sans-serif;
    background:
      radial-gradient(circle at 0% 0%, rgba(56, 189, 248, 0.18), transparent 40%),
      radial-gradient(circle at 100% 100%, rgba(34, 197, 94, 0.14), transparent 36%),
      linear-gradient(180deg, #f5f9ff 0%, var(--bg) 48%, #ecf3ff 100%);
    color: var(--text);
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
  }

  :global(code) {
    font-family: "Fira Code", ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
    font-size: 12px;
  }

  main {
    max-width: 1120px;
    margin: 0 auto;
    padding: 16px;
    padding-top: calc(16px + env(safe-area-inset-top));
    padding-right: calc(16px + env(safe-area-inset-right));
    padding-bottom: calc(16px + env(safe-area-inset-bottom));
    padding-left: calc(16px + env(safe-area-inset-left));
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  /* Shared utilities used across components */
  :global(.subtle) {
    font-size: 12px;
    color: var(--muted);
    margin-top: 2px;
  }

  :global(.secondary) {
    background: linear-gradient(145deg, rgba(255, 255, 255, 0.92), rgba(238, 246, 255, 0.86));
  }

  :global(.session-status) {
    display: inline-flex;
    align-items: center;
    border-radius: 999px;
    padding: 4px 8px;
    font-size: 12px;
    font-weight: 800;
    border: 1px solid var(--border);
    background: var(--surface-2);
    flex: 0 0 auto;
  }

  :global(.session-status[data-kind="running"]) {
    background: rgba(34, 197, 94, 0.12);
    border-color: rgba(34, 197, 94, 0.28);
    color: #065f46;
  }

  :global(.session-status[data-kind="warning"]) {
    background: rgba(249, 115, 22, 0.12);
    border-color: rgba(249, 115, 22, 0.28);
    color: #92400e;
  }

  :global(.session-status[data-kind="error"]) {
    background: rgba(239, 68, 68, 0.1);
    border-color: rgba(239, 68, 68, 0.22);
    color: #991b1b;
  }

  :global(.session-status[data-kind="done"]) {
    background: rgba(107, 114, 128, 0.12);
    border-color: rgba(107, 114, 128, 0.22);
    color: #374151;
  }

  /* Session layout grid */
  .sessions-layout {
    display: grid;
    grid-template-columns: clamp(320px, 34vw, 360px) 1fr;
    gap: 12px;
    align-items: start;
  }

  @media (max-width: 1024px) {
    .sessions-layout {
      grid-template-columns: 320px 1fr;
    }
  }

  @media (max-width: 640px) {
    .sessions-layout {
      grid-template-columns: 1fr;
    }
    .sessions-layout.mobile-detail-open :global(.sessions-list) {
      display: none;
    }
    .sessions-layout:not(.mobile-detail-open) :global(.sessions-detail),
    .sessions-layout:not(.mobile-detail-open) :global(.sessions-todo) {
      display: none;
    }
  }

  /* BlocksRenderer shared global styles (used via @html rendering) */
  :global(.chat-row) { display: flex; flex-direction: column; gap: 4px; margin: 10px 0; }
  :global(.chat-row[data-role="assistant"]) { align-items: flex-start; }
  :global(.chat-row[data-role="user"]) { align-items: flex-end; }
  :global(.chat-row[data-role="system"]) { align-items: center; text-align: center; }

  :global(.chat-bubble) {
    max-width: 70%;
    padding: 10px 12px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: var(--surface);
    word-break: break-word;
    white-space: pre-wrap;
  }
  :global(.chat-bubble[data-role="assistant"]) { background: rgba(248, 250, 255, 0.98); }
  :global(.chat-bubble[data-role="user"]) { background: rgba(14, 165, 233, 0.16); border-color: rgba(14, 165, 233, 0.3); }

  :global(.chat-system) { max-width: 70%; font-size: 12px; color: var(--muted); word-break: break-word; }
  :global(.chat-ts) { font-size: 11px; color: var(--muted); }

  :global(.tool-card) {
    width: 100%;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: linear-gradient(155deg, rgba(255, 255, 255, 0.92), rgba(241, 245, 255, 0.92));
    padding: 10px 12px;
  }
  :global(.tool-card summary) { cursor: pointer; display: flex; gap: 8px; align-items: center; flex-wrap: wrap; list-style: none; }
  :global(.tool-card summary::-webkit-details-marker) { display: none; }
  :global(.tool-card-body) { margin-top: 8px; text-align: left; }
  :global(.tool-card-actions) { display: flex; gap: 8px; flex-wrap: wrap; margin-bottom: 10px; }
  :global(.tool-card-label) { font-size: 12px; color: var(--muted); margin-bottom: 6px; }
  :global(.tool-json) {
    margin: 0; padding: 10px 12px; border-radius: var(--radius-lg);
    border: 1px solid var(--border); background: rgba(241, 245, 255, 0.78);
    font-size: 12px; white-space: pre-wrap; word-break: break-word; overflow: auto; max-height: 240px;
  }

  :global(.out-mark) { background: rgba(245, 158, 11, 0.35); color: inherit; border-radius: 4px; padding: 0 1px; }
  :global(.out-mark.current) { background: rgba(245, 158, 11, 0.7); }

  @media (prefers-reduced-motion: reduce) {
    * {
      transition: none !important;
      scroll-behavior: auto !important;
    }
  }
</style>
