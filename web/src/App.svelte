<script lang="ts">
  import { onMount, tick } from "svelte";

  type Health = { name: string; version: string };
  type LoginResponse = { access_token: string };
  type RunRow = {
    id: string;
    host_id: string;
    tool: string;
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
  let view: "sessions" | "hosts" | "start" | "tools" | "settings" = "sessions";
  let health: Health | null = null;
  let events: WsEnvelope[] = [];
  let runs: RunRow[] = [];
  let hosts: HostInfo[] = [];
  let ws: WebSocket | null = null;
  let messagesByRun: Record<string, ChatMessage[]> = {};
  let isMobile = false;

  let selectedRunId = "";
  let inputModalOpen = false;
  let inputModalText = "";
  let inputModalEl: HTMLTextAreaElement | null = null;
  let lastSeenPromptRequest: Record<string, string> = {};
  let stopConfirmOpen = false;
  let approvalModalOpen = false;
  let approvalModalShowArgs = false;
  let lastSeenApprovalRequest: Record<string, string> = {};
  let lastError = "";
  let outputByRun: Record<string, string> = {};
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

  type TodoItem = { id: string; text: string; done: boolean; created_at: string };
  let todos: TodoItem[] = [];
  let todoText = "";

  let sessionDetailTab: "output" | "messages" = "output";
  let outputAutoScroll = true;
  let outputIsAtBottom = true;
  let outputBufferLines = 400;
  let outputFeedEl: HTMLDivElement | null = null;
  let outputSearchInputEl: HTMLInputElement | null = null;

  let outputSearchText = "";
  let outputSearchActive = "";
  let outputSearchCursor = 0;

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
  let outputAutoResumeTimer: ReturnType<typeof setTimeout> | null = null;

  let startHostId = "host-dev";
  let startTool = "codex";
  let startCmd = "echo Proceed?; cat";
  let startCwd = "";
  let startError = "";
  let recentSessions: RunRow[] = [];

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

  function isRecord(v: unknown): v is Record<string, unknown> {
    return Boolean(v) && typeof v === "object";
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
    op_tool?: string;
  }): boolean {
    const reason = (a.reason ?? "").trim().toLowerCase();
    const opTool = (a.op_tool ?? "").trim();
    return reason === "prompt" && !opTool;
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
    sessionDetailTab = "output";
    outputAutoScroll = true;
    outputSearchText = "";
    outputSearchActive = "";
    outputSearchCursor = 0;
    await loadMessages(runId);
    await focusOutputSearch();
  }

  function tailLines(text: string, maxLines: number): string {
    const max = Math.max(1, maxLines | 0);
    const lines = text.split(/\r?\n/);
    if (lines.length <= max) return text;
    return lines.slice(lines.length - max).join("\n");
  }

  function updateOutputBufferLines() {
    const viewHeight = outputFeedEl?.clientHeight ?? 520;
    const lineHeight = 18;
    const visibleLines = Math.max(1, Math.floor(viewHeight / lineHeight));
    outputBufferLines = Math.min(2000, Math.max(200, visibleLines * 4));
  }

  function outputAtBottom(el: HTMLDivElement): boolean {
    const threshold = 8;
    return el.scrollTop + el.clientHeight >= el.scrollHeight - threshold;
  }

  function scheduleOutputAutoResume() {
    if (outputAutoResumeTimer) clearTimeout(outputAutoResumeTimer);
    outputAutoResumeTimer = setTimeout(async () => {
      outputAutoScroll = true;
      await tick();
      if (outputFeedEl) {
        outputFeedEl.scrollTop = outputFeedEl.scrollHeight;
        outputIsAtBottom = true;
      }
    }, 10_000);
  }

  async function resumeOutputAutoScroll() {
    outputAutoScroll = true;
    if (outputAutoResumeTimer) {
      clearTimeout(outputAutoResumeTimer);
      outputAutoResumeTimer = null;
    }
    await tick();
    if (outputFeedEl) {
      outputFeedEl.scrollTop = outputFeedEl.scrollHeight;
      outputIsAtBottom = true;
    }
  }

  function pauseOutputAutoScroll() {
    outputAutoScroll = false;
    scheduleOutputAutoResume();
  }

  function handleOutputScroll() {
    if (!outputFeedEl) return;
    const atBottom = outputAtBottom(outputFeedEl);
    outputIsAtBottom = atBottom;
    if (!atBottom) {
      if (outputAutoScroll) pauseOutputAutoScroll();
      else scheduleOutputAutoResume();
    } else if (!outputAutoScroll) {
      scheduleOutputAutoResume();
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

  function resetConnectionState() {
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
  }

  function openAppWebSocket(nextToken: string) {
    status = "connecting";
    const nextWs = new WebSocket(`${toWsBase(apiBaseUrl)}/ws/app?token=${encodeURIComponent(nextToken)}`);
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
          const reqId = dataString(msg, "request_id");
          awaitingByRun = {
            ...awaitingByRun,
            [msg.run_id]: {
              reason: dataString(msg, "reason"),
              prompt: dataString(msg, "prompt"),
              request_id: reqId,
              approve_text: dataString(msg, "approve_text"),
              deny_text: dataString(msg, "deny_text"),
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
              op_tool: dataString(msg, "op_tool"),
              op_args: dataAny(msg, "op_args"),
              op_args_summary: dataString(msg, "op_args_summary"),
              approve_text: dataString(msg, "approve_text"),
              deny_text: dataString(msg, "deny_text"),
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
          const last_active_at = msg.ts;
          const hasReqId = dataString(msg, "request_id");

          if (i === -1 && msg.type === "run.started") {
            runs = [
              {
                id: msg.run_id,
                host_id: msg.host_id ?? "unknown",
                tool: dataString(msg, "tool") ?? "unknown",
                cwd: dataString(msg, "cwd") ?? ".",
                status: "running",
                started_at: msg.ts,
                last_active_at: msg.ts,
              },
              ...runs,
            ];
            return;
          }

          if (i !== -1) {
            const base: RunRow = { ...runs[i], last_active_at };
            if (msg.type === "run.permission_requested") {
              runs[i] = {
                ...base,
                status: "awaiting_approval",
                pending_request_id: dataString(msg, "request_id"),
                pending_reason: dataString(msg, "reason"),
                pending_prompt: dataString(msg, "prompt"),
                pending_op_tool: dataString(msg, "op_tool"),
                pending_op_args_summary: dataString(msg, "op_args_summary"),
              };
            } else if (msg.type === "run.awaiting_input") {
              runs[i] = { ...base, status: hasReqId ? "awaiting_approval" : "awaiting_input" };
            } else if (msg.type === "run.input") {
              runs[i] = {
                ...base,
                status: "running",
                pending_request_id: null,
                pending_reason: null,
                pending_prompt: null,
                pending_op_tool: null,
                pending_op_args_summary: null,
              };
            } else if (msg.type === "run.exited") {
              const exit_code =
                isRecord(msg.data) && typeof msg.data["exit_code"] === "number" ? msg.data["exit_code"] : base.exit_code;
              runs[i] = {
                ...base,
                status: "exited",
                ended_at: msg.ts,
                exit_code,
                pending_request_id: null,
                pending_reason: null,
                pending_prompt: null,
                pending_op_tool: null,
                pending_op_args_summary: null,
              };
            } else if (msg.type === "run.started") {
              runs[i] = { ...base, status: "running", started_at: msg.ts, ended_at: null, exit_code: null };
            } else if (msg.type === "run.output" || msg.type === "tool.call" || msg.type === "tool.result") {
              runs[i] = base;
            }
            runs = [...runs];
          }
        }
      } catch {
        // ignore
      }
    };
  }

  async function connect() {
    lastError = "";
    try {
      resetConnectionState();

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
      persistAuthPrefs();

      await refreshHosts();
      await refreshRuns();
      if (!token) return;
      if (selectedRunId) await loadMessages(selectedRunId);

      openAppWebSocket(token);
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

      const h = await fetch(`${apiBaseUrl.replace(/\/$/, "")}/health`);
      if (!h.ok) {
        const body = await h.text().catch(() => "");
        throw new Error(`health failed: ${h.status} ${body}`.trim());
      }
      health = (await h.json()) as Health;
      view = "sessions";

      await refreshHosts();
      await refreshRuns();
      if (!token) return;
      if (selectedRunId) await loadMessages(selectedRunId);

      openAppWebSocket(savedToken);
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
    const r = await fetch(`${apiBaseUrl.replace(/\/$/, "")}/hosts`, {
      headers: { Authorization: `Bearer ${token}` },
    });
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
    } else {
      hosts = [];
    }
  }

  async function refreshRuns() {
    if (!token) return;
    const r = await fetch(`${apiBaseUrl.replace(/\/$/, "")}/sessions`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    if (r.status === 401) {
      lastError = "登录已过期，请重新登录";
      setToast("登录已过期");
      disconnect();
      return;
    }
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

  async function loadMessages(runId: string) {
    if (!token) return;
    const r = await fetch(
      `${apiBaseUrl.replace(/\/$/, "")}/sessions/${encodeURIComponent(runId)}/messages?limit=200`,
      {
        headers: { Authorization: `Bearer ${token}` },
      },
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
    sendInput(inputModalText);
    closeInputModal();
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
  $: awaitingRuns = runs.filter((r) => r.status === "awaiting_input" || r.status === "awaiting_approval");
  $: selectedOutput = selectedRunId ? outputByRun[selectedRunId] ?? "" : "";
  $: selectedAwaiting = selectedRunId ? awaitingByRun[selectedRunId] ?? awaitingFromRunRow(selectedRun) : null;
  $: selectedMessages = selectedRunId ? messagesByRun[selectedRunId] ?? [] : [];
  $: hostsById = Object.fromEntries(hosts.map((h) => [h.id, h] as const)) as Record<string, HostInfo>;

  $: if (selectedRunId && selectedAwaiting && awaitingIsApproval(selectedAwaiting) && !approvalModalOpen) {
    const key = (selectedAwaiting.request_id ?? selectedAwaiting.op_tool ?? "").trim();
    if (key && lastSeenApprovalRequest[selectedRunId] !== key) {
      lastSeenApprovalRequest = { ...lastSeenApprovalRequest, [selectedRunId]: key };
      approvalModalShowArgs = false;
      approvalModalOpen = true;
    }
  }

  $: if (selectedRunId && selectedAwaiting && awaitingIsPrompt(selectedAwaiting) && !inputModalOpen) {
    const key = (selectedAwaiting.request_id ?? "").trim();
    if (key && lastSeenPromptRequest[selectedRunId] !== key) {
      lastSeenPromptRequest = { ...lastSeenPromptRequest, [selectedRunId]: key };
      openInputModal("");
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

  $: outputDisplayText = tailLines(selectedOutput, outputBufferLines);
  $: outputLines = outputDisplayText.split(/\r?\n/);
  $: outputSearchMatches = outputSearchActive ? computeOutputMatches(outputLines, outputSearchActive, selectedRunId) : [];
  $: if (outputSearchMatches.length === 0) outputSearchCursor = 0;
  $: if (outputSearchCursor >= outputSearchMatches.length) outputSearchCursor = 0;
  $: outputHtml = renderOutputHtml(outputLines, outputSearchMatches, outputSearchCursor);
  $: if (sessionDetailTab === "output" && outputAutoScroll) {
    tick().then(() => {
      if (!outputFeedEl) return;
      outputFeedEl.scrollTop = outputFeedEl.scrollHeight;
      outputIsAtBottom = true;
    });
  }

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
      if (outputAutoResumeTimer) clearTimeout(outputAutoResumeTimer);
      if (toastTimer) clearTimeout(toastTimer);
    };
  });
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
      <span class="conn-status" data-kind={status}>
        <span class="conn-dot" aria-hidden="true"></span>
        <span>{connLabel(status)}</span>
      </span>
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
      <input bind:value={username} autocomplete="username" on:change={persistAuthPrefs} />
    </label>
    <label>
      密码
      <input type="password" bind:value={password} autocomplete="current-password" on:change={persistAuthPrefs} />
    </label>
    <div class="login-prefs">
      <label class="checkbox">
        <input
          type="checkbox"
          bind:checked={keepSignedIn}
          on:change={() => {
            persistAuthPrefs();
          }}
        />
        刷新后保持登录
      </label>
      <label class="checkbox">
        <input
          type="checkbox"
          bind:checked={rememberPassword}
          on:change={() => {
            if (!rememberPassword) password = "";
            persistAuthPrefs();
          }}
        />
        记住密码（本机）
      </label>
    </div>
    <div style="display:flex;gap:8px;flex-wrap:wrap">
      <button on:click={connect} disabled={loginBusy || !username.trim() || !password}>{loginBusy ? "登录中…" : "登录"}</button>
    </div>
    {#if isProbablyInsecureUrl(apiBaseUrl)}
      <div style="margin-top:8px;padding:8px;border:1px solid #f59e0b;background:#fffbeb">
        注意：当前是 <code>http://</code>，密码会明文传输。建议通过 HTTPS 访问（例如用 Caddy 反代）。
      </div>
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

    <div style="margin-top:14px">
      <div style="font-weight:600">登录</div>
      <div class="subtle">当前用户：<code>{username}</code></div>
      <div class="login-prefs" style="margin-top:8px">
        <label class="checkbox">
          <input
            type="checkbox"
            bind:checked={keepSignedIn}
            on:change={() => {
              persistAuthPrefs();
            }}
          />
          刷新后保持登录
        </label>
        <label class="checkbox">
          <input
            type="checkbox"
            bind:checked={rememberPassword}
            on:change={() => {
              if (!rememberPassword) password = "";
              persistAuthPrefs();
            }}
          />
          记住密码（本机）
        </label>
      </div>
      {#if rememberPassword}
        <label style="margin-top:10px">
          密码
          <input type="password" bind:value={password} autocomplete="current-password" on:change={persistAuthPrefs} />
        </label>
      {/if}
    </div>
  </section>
  {/if}

  {#if token && view === "sessions"}
  <div class="sessions-layout" class:mobile-detail-open={isMobile && Boolean(selectedRunId)}>
  <section class="sessions-list">
    <div class="list-head">
      <h2 style="margin:0">会话</h2>
      <button
        class="secondary"
        on:click={async () => {
          await Promise.all([refreshHosts(), refreshRuns()]);
        }}
        disabled={!token}
      >
        刷新
      </button>
    </div>
    <div class="list-search">
      <input bind:value={sessionSearch} placeholder="搜索" />
    </div>

  {#each runGroups as g (g.host_id)}
      <div class="host-group">
        <button class="host-group-header" on:click={() => toggleHostGroup(g.host_id)} aria-expanded={!hostGroupCollapsed[g.host_id]}>
          <span class="chevron">{hostGroupCollapsed[g.host_id] ? "▸" : "▾"}</span>
          <span class="dot" data-online={g.online ? "1" : "0"} aria-hidden="true"></span>
          <span class="host-name">{g.display_name}</span>
          <span class="host-last-seen">{formatRelativeTime(g.last_seen_at)}</span>
        </button>
        {#if !hostGroupCollapsed[g.host_id]}
          <div class="session-items">
            {#each g.sessions as r (r.id)}
              {@const st = statusLabel(r)}
              {@const title = sessionTitle(r)}
              {@const summary = sessionSummary(r)}
              <button class="session-item" class:selected={selectedRunId === r.id} on:click={() => selectSession(r.id)}>
                <div class="session-item-top">
                  {#if title}
                    <div class="session-title">{title}</div>
                  {/if}
                  <span class="session-status" data-kind={st.kind}>{st.label}</span>
                </div>
                {#if r.status === "awaiting_approval"}
                  <div class="session-meta">
                    <span class="session-tool">{r.tool}</span>
                    {#if r.pending_op_tool}<span class="session-op">{r.pending_op_tool}</span>{/if}
                    {#if r.pending_op_args_summary}<span class="session-op-args">{r.pending_op_args_summary}</span>{/if}
                  </div>
                {:else}
                  {#if summary}
                    <div class="session-summary">{summary}</div>
                  {/if}
                {/if}
                <div class="session-time">{formatRelativeTime(r.last_active_at ?? r.started_at)}</div>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/each}
  </section>

  <section class="sessions-detail">
    {#if !selectedRun}
      <div class="subtle"></div>
    {:else}
      {@const st = statusLabel(selectedRun)}
      {@const host = hostsById[selectedRun.host_id] ?? null}
      {@const title = sessionTitle(selectedRun)}
      <div class="detail-head">
        <div class="detail-title">
          {#if title}
            <div class="detail-title-main">{title}</div>
          {/if}
          <div class="detail-title-sub">
            <span class="dot" data-online={host?.online ? "1" : "0"} aria-hidden="true"></span>
            <span class="detail-host"><code>{selectedRun.host_id}</code></span>
            <span class="subtle">最近 {formatRelativeTime(host?.last_seen_at ?? null)}</span>
            <span class="subtle">活跃 {formatRelativeTime(selectedRun.last_active_at ?? selectedRun.started_at)}</span>
          </div>
          <div class="detail-meta">
            <span class="meta-pill">
              <span class="meta-k">tool</span>
              <span class="meta-v">{selectedRun.tool}</span>
            </span>
            <span class="meta-pill">
              <span class="meta-k">run</span>
              <span class="meta-v"><code>{selectedRun.id}</code></span>
            </span>
            <span class="meta-pill meta-pill-cwd">
              <span class="meta-k">cwd</span>
              <span class="meta-v"><code>{selectedRun.cwd}</code></span>
            </span>
          </div>
        </div>
        <div class="detail-actions">
          {#if isMobile}
            <button class="secondary" on:click={() => (selectedRunId = "")} type="button">返回</button>
          {/if}
          <span class="session-status" data-kind={st.kind}>{st.label}</span>
          {#if selectedAwaiting && awaitingIsApproval(selectedAwaiting)}
            <button
              on:click={() => {
                approvalModalShowArgs = false;
                approvalModalOpen = true;
              }}
              disabled={!selectedRunId}
              type="button"
            >
              审批
            </button>
          {/if}
          <button on:click={() => (stopConfirmOpen = true)} disabled={!selectedRunId || status !== "connected"}>停止</button>
        </div>
      </div>

      <div class="detail-tabs" role="tablist" aria-label="session detail tabs">
        <button
          class:active={sessionDetailTab === "output"}
          role="tab"
          on:click={async () => {
            sessionDetailTab = "output";
            await focusOutputSearch();
          }}
        >
          输出
        </button>
        <button class:active={sessionDetailTab === "messages"} role="tab" on:click={() => (sessionDetailTab = "messages")}>消息</button>
        <button on:click={() => selectedRunId && loadMessages(selectedRunId)} disabled={!selectedRunId || !token} style="margin-left:auto">
          刷新
        </button>
      </div>

      {#if token && status !== "connected"}
        <div class="offline-banner">
          <span class="dot" data-online="0" aria-hidden="true"></span>
          <span>离线</span>
          <button class="secondary" on:click={resumeFromStoredToken} type="button">重连</button>
          <button class="secondary" on:click={refreshSelectedSession} disabled={!selectedRunId} type="button">刷新</button>
        </div>
      {/if}

      {#if selectedAwaiting && (selectedAwaiting.op_tool || selectedAwaiting.op_args_summary || selectedAwaiting.prompt)}
        <div class="awaiting-banner">
          <div class="awaiting-banner-top">
            {#if selectedAwaiting.op_tool}<span class="session-op">{selectedAwaiting.op_tool}</span>{/if}
            {#if selectedAwaiting.op_args_summary}<span class="session-op-args">{selectedAwaiting.op_args_summary}</span>{/if}
          </div>
          {#if selectedAwaiting.prompt}
            <div class="awaiting-banner-prompt">{selectedAwaiting.prompt}</div>
          {/if}
        </div>
      {/if}

      {#if sessionDetailTab === "messages"}
        <div class="chat-feed">
          {#if displayMessages.length === 0}
            <div class="subtle"></div>
          {:else}
            {#each displayMessages as m, i (m.key)}
              {#if m.kind === "tool.call" && displayMessages[i + 1]?.kind === "tool.result" && displayMessages[i + 1]?.request_id && displayMessages[i + 1]?.request_id === m.request_id}
                {@const callMeta = toolMetaFromText(m.kind, m.text || "")}
                {@const res = displayMessages[i + 1]}
                {@const resMeta = toolMetaFromText(res.kind, res.text || "")}
                <div class="chat-row" data-role="system">
                  <details open class="tool-card">
                    <summary>
                      <code>{callMeta.label}</code>
                      {#if m.actor}<code>actor={m.actor}</code>{/if}
                      {#if (res.text || "").includes(" ok=true")}
                        <span style="color:#065f46">ok</span>
                      {:else if (res.text || "").includes(" ok=false")}
                        <span style="color:#b91c1c">error</span>
                      {/if}
                    </summary>
                    <div class="tool-card-body">
                      <div class="tool-card-label">call</div>
                      {@html renderMarkdownBasic(callMeta.details || "")}
                      <div class="tool-card-label" style="margin-top:10px">result</div>
                      {@html renderMarkdownBasic(resMeta.details || "")}
                    </div>
                  </details>
                </div>
              {:else if m.kind === "tool.result" && displayMessages[i - 1]?.kind === "tool.call" && displayMessages[i - 1]?.request_id && displayMessages[i - 1]?.request_id === m.request_id}
                <!-- paired with previous tool.call; skip rendering -->
              {:else if m.kind === "run.permission_requested"}
                {@const isCurrent = Boolean(selectedAwaiting?.request_id) && selectedAwaiting?.request_id === m.request_id}
                <div class="chat-row" data-role="system">
                  <div class="approval-card">
                    <div class="approval-card-top">
                      <span class="meta-pill">
                        <span class="meta-k">tool</span>
                        <span class="meta-v">{selectedRun.tool}</span>
                      </span>
                      {#if isCurrent && selectedAwaiting?.op_tool}
                        <span class="session-op">{selectedAwaiting.op_tool}</span>
                      {/if}
                      {#if isCurrent && selectedAwaiting?.op_args_summary}
                        <span class="session-op-args">{selectedAwaiting.op_args_summary}</span>
                      {/if}
                    </div>
                    {#if m.text}
                      <div class="approval-card-prompt">{m.text}</div>
                    {/if}
                  </div>
                  <div class="chat-ts">{formatAbsTime(m.ts)}</div>
                </div>
              {:else}
                <div class="chat-row" data-role={m.role}>
                  {#if m.role === "system"}
                    <div class="chat-system">
                      {@html renderMarkdownBasic(m.text || "")}
                    </div>
                  {:else}
                    <div class="chat-bubble" data-role={m.role}>
                      {@html renderMarkdownBasic(m.text || "")}
                    </div>
                  {/if}
                  <div class="chat-ts">{formatAbsTime(m.ts)}</div>
                </div>
              {/if}
            {/each}
          {/if}
        </div>
      {:else}
        <div class="output-toolbar">
          <div class="output-searchbar">
            <input
              bind:this={outputSearchInputEl}
              bind:value={outputSearchText}
              on:keydown={handleOutputSearchKeydown}
              placeholder=""
            />
            <button on:click={runOutputSearch} disabled={!outputSearchText.trim()}>搜索</button>
            <button on:click={prevOutputMatch} disabled={outputSearchMatches.length === 0}>↑</button>
            <button on:click={nextOutputMatch} disabled={outputSearchMatches.length === 0}>↓</button>
            {#if outputSearchActive}
              <div class="output-count">
                {outputSearchMatches.length === 0 ? "0/0" : `${outputSearchCursor + 1}/${outputSearchMatches.length}`}
              </div>
            {/if}
            <button on:click={clearOutputSearch} disabled={!outputSearchText && !outputSearchActive}>清空</button>
          </div>
          <div class="output-actions">
            <button on:click={toggleOutputAutoScroll} disabled={!selectedRunId}>
              {outputAutoScroll ? "暂停" : "继续"}
            </button>
            {#if !outputAutoScroll && !outputIsAtBottom}
              <button on:click={jumpToLatest} disabled={!selectedRunId}>跳到最新</button>
            {/if}
            <button on:click={copyOutput} disabled={!selectedOutput}>复制输出</button>
          </div>
        </div>
        <div class="output-feed" bind:this={outputFeedEl} on:scroll={handleOutputScroll}>
          {#if outputHtml}
            <pre class="output-pre">{@html outputHtml}</pre>
          {/if}
          {#if !outputAutoScroll}
            <button class="paused-badge" on:click={resumeOutputAutoScroll} type="button">已暂停</button>
          {/if}
        </div>
      {/if}

      <div class="detail-input">
        <button class="secondary" on:click={() => openInputModal("")} disabled={!selectedRunId || status !== "connected"} type="button">
          输入
        </button>
      </div>
    {/if}
  </section>

  <section class="sessions-todo">
    <h2>待办</h2>
    <div style="display:flex;gap:8px;flex-wrap:wrap;align-items:center;margin-bottom:8px">
      <input bind:value={todoText} placeholder="新增待办…" />
      <button
        on:click={() => {
          addTodo(todoText);
          todoText = "";
        }}
        disabled={!selectedRunId}
      >
        添加
      </button>
    </div>

    {#if todoSuggestions.length > 0}
      <div style="margin:8px 0;padding:8px;border:1px solid #e5e7eb;background:#f8fafc">
        <strong>建议（来自输出）</strong>
        <ul>
          {#each todoSuggestions as s (s)}
            <li>
              <button on:click={() => addTodo(s)} disabled={!selectedRunId} style="margin-right:8px">添加</button>
              {s}
            </li>
          {/each}
        </ul>
      </div>
    {/if}

    {#if todos.length > 0}
      <ul>
        {#each todos as t (t.id)}
          <li>
            <label style="display:flex;gap:8px;align-items:center">
              <input type="checkbox" checked={t.done} on:change={() => toggleTodo(t.id)} />
              <span style={t.done ? "text-decoration:line-through;color:#6b7280" : ""}>{t.text}</span>
            </label>
            <button on:click={() => removeTodo(t.id)} style="margin-left:8px">移除</button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
  </div>
  {/if}

  <section class:hidden={!token || view !== "tools"}>
    <h2>文件（run cwd）</h2>
    <label>
      路径（相对）
      <input bind:value={filePath} placeholder="README.md" />
    </label>
    <button on:click={fetchFile} disabled={!selectedRunId || status !== "connected"}>读取</button>
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
        <div style="color:#92400e">结果已截断</div>
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

  <section class:hidden={!token || view !== "tools"}>
    <h2>Git（run cwd）</h2>
    <div style="display:flex;gap:8px;flex-wrap:wrap">
      <button on:click={fetchGitStatus} disabled={!selectedRunId || status !== "connected"}>状态</button>
      <button on:click={fetchGitDiff} disabled={!selectedRunId || status !== "connected"}>差异</button>
    </div>
    <label>
      Diff 路径（可选，相对）
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

  <section class:hidden={!token || view !== "hosts"}>
    <h2>主机诊断（WS-RPC）</h2>
    <div style="display:flex;gap:8px;flex-wrap:wrap;margin-bottom:8px">
      <button on:click={refreshHosts} disabled={!token}>刷新主机</button>
    </div>
    <label>
      主机 ID
      {#if hosts.length > 0}
        <select bind:value={hostDiagHostId} style="width:100%;padding:10px;box-sizing:border-box">
          {#each hosts as h (h.id)}
            <option value={h.id}>{h.id}{h.online ? "（在线）" : "（离线）"}</option>
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
    <button on:click={refreshHosts} disabled={!token} style="margin-bottom:8px">刷新主机</button>
    <label>
      主机 ID
      {#if hosts.length > 0}
        <select bind:value={startHostId} style="width:100%;padding:10px;box-sizing:border-box">
          {#each hosts as h (h.id)}
            <option value={h.id}>{h.id}{h.online ? "（在线）" : "（离线）"}</option>
          {/each}
        </select>
      {:else}
        <input bind:value={startHostId} placeholder="host-dev" />
      {/if}
    </label>
    <label>
      工具
      <input bind:value={startTool} placeholder="codex" />
    </label>
    <label>
      CWD（可选，主机路径）
      <input bind:value={startCwd} placeholder="/path/to/project" />
    </label>
    <label>
      命令
      <input bind:value={startCmd} placeholder="echo hi; cat" />
    </label>
    <button on:click={startRun} disabled={status !== "connected"}>启动</button>
    {#if startError}
      <div style="color:#b91c1c">{startError}</div>
    {/if}
  </section>

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

  {#if inputModalOpen}
    <div class="modal-overlay" role="dialog" aria-modal="true">
      <div class="modal">
        <div class="modal-head">
          <div class="modal-title">输入</div>
          <button class="secondary" on:click={closeInputModal} type="button">关闭</button>
        </div>
        <div class="modal-body">
          <textarea
            class="input-textarea"
            bind:this={inputModalEl}
            bind:value={inputModalText}
            placeholder=""
            rows="3"
          ></textarea>
          <div class="quick-inputs">
            <button class="secondary" on:click={() => sendQuickInput("y\n")} type="button">y</button>
            <button class="secondary" on:click={() => sendQuickInput("n\n")} type="button">n</button>
            <button class="secondary" on:click={() => sendQuickInput("continue\n")} type="button">continue</button>
          </div>
        </div>
        <div class="modal-actions">
          <button class="secondary" on:click={closeInputModal} type="button">取消</button>
          <button on:click={sendInputModalText} disabled={!selectedRunId || status !== "connected"} type="button">发送</button>
        </div>
      </div>
    </div>
  {/if}

  {#if approvalModalOpen && selectedRun && selectedAwaiting && awaitingIsApproval(selectedAwaiting)}
    {@const risk = riskForOpTool(selectedAwaiting.op_tool)}
    <div class="modal-overlay" role="dialog" aria-modal="true">
      <div class="modal">
        <div class="modal-head">
          <div class="modal-title">待审批</div>
          <button
            class="secondary"
            on:click={() => {
              approvalModalOpen = false;
              approvalModalShowArgs = false;
            }}
            type="button"
          >
            关闭
          </button>
        </div>
        <div class="modal-body">
          <div class="approval-meta">
            <span class="meta-pill">
              <span class="meta-k">tool</span>
              <span class="meta-v">{selectedRun.tool}</span>
            </span>
            {#if selectedAwaiting.op_tool}
              <span class="meta-pill">
                <span class="meta-k">op</span>
                <span class="meta-v"><code>{selectedAwaiting.op_tool}</code></span>
              </span>
            {/if}
            {#if risk}
              <span class="risk-pill" data-kind={risk.kind}>{risk.label}</span>
            {/if}
          </div>

          {#if selectedAwaiting.op_args_summary}
            <div class="approval-summary"><code>{selectedAwaiting.op_args_summary}</code></div>
          {/if}

          {#if selectedAwaiting.prompt}
            <div class="approval-prompt">{selectedAwaiting.prompt}</div>
          {/if}

          {#if selectedAwaiting.op_args !== undefined && selectedAwaiting.op_args !== null}
            <details class="approval-details" bind:open={approvalModalShowArgs}>
              <summary>参数</summary>
              <pre>{JSON.stringify(selectedAwaiting.op_args, null, 2)}</pre>
            </details>
          {/if}
        </div>
        <div class="modal-actions">
          <button
            class="secondary"
            on:click={() => {
              approvalModalOpen = false;
              approvalModalShowArgs = false;
            }}
            type="button"
          >
            取消
          </button>
          <button
            on:click={() => {
              sendDecision("deny");
              approvalModalOpen = false;
              approvalModalShowArgs = false;
            }}
            disabled={!selectedRunId || status !== "connected"}
            type="button"
          >
            拒绝
          </button>
          <button
            on:click={() => {
              sendDecision("approve");
              approvalModalOpen = false;
              approvalModalShowArgs = false;
            }}
            disabled={!selectedRunId || status !== "connected"}
            type="button"
          >
            同意
          </button>
        </div>
      </div>
    </div>
  {/if}

  {#if stopConfirmOpen}
    <div class="modal-overlay" role="dialog" aria-modal="true">
      <div class="modal">
        <div class="modal-head">
          <div class="modal-title">停止会话</div>
          <button class="secondary" on:click={() => (stopConfirmOpen = false)} type="button">关闭</button>
        </div>
        <div class="modal-body">
          <code>{selectedRunId}</code>
        </div>
        <div class="modal-actions">
          <button class="secondary" on:click={() => (stopConfirmOpen = false)} type="button">取消</button>
          <button
            on:click={() => {
              sendStop("term");
              stopConfirmOpen = false;
            }}
            disabled={!selectedRunId || status !== "connected"}
            type="button"
          >
            停止
          </button>
          <button
            class="danger"
            on:click={() => {
              sendStop("kill");
              stopConfirmOpen = false;
            }}
            disabled={!selectedRunId || status !== "connected"}
            type="button"
          >
            强制停止
          </button>
        </div>
      </div>
    </div>
  {/if}

  {#if toastText}
    <div class="toast" role="status" aria-live="polite">{toastText}</div>
  {/if}
</main>

<style>
  :global(:root) {
    --bg: #f8fafc;
    --surface: rgba(255, 255, 255, 0.92);
    --surface-2: rgba(255, 255, 255, 0.78);
    --surface-muted: rgba(248, 250, 252, 0.96);
    --text: #1e293b;
    --text-strong: #0f172a;
    --muted: #64748b;
    --border: rgba(2, 6, 23, 0.12);
    --border-strong: rgba(2, 6, 23, 0.18);
    --shadow-sm: 0 1px 2px rgba(2, 6, 23, 0.06);
    --shadow-md: 0 10px 30px rgba(2, 6, 23, 0.1);
    --primary: #2563eb;
    --primary-2: #3b82f6;
    --success: #22c55e;
    --warning: #f97316;
    --danger: #ef4444;
    --radius-lg: 16px;
    --radius-md: 12px;
    --radius-sm: 10px;
  }

  :global(body) {
    margin: 0;
    font-family: system-ui, -apple-system, BlinkMacSystemFont, "SF Pro Display", "SF Pro Text", Segoe UI, Roboto, sans-serif;
    background: var(--bg);
    color: var(--text);
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
  }

  :global(code) {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
    font-size: 12px;
  }

  main {
    max-width: 1040px;
    margin: 0 auto;
    padding: 14px;
    padding-top: calc(14px + env(safe-area-inset-top));
    padding-right: calc(14px + env(safe-area-inset-right));
    padding-bottom: calc(14px + env(safe-area-inset-bottom));
    padding-left: calc(14px + env(safe-area-inset-left));
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
    color: var(--muted);
    margin-top: 2px;
  }

  .toast {
    position: fixed;
    left: 50%;
    bottom: 18px;
    transform: translateX(-50%);
    padding: 10px 12px;
    border-radius: 999px;
    border: 1px solid rgba(255, 255, 255, 0.18);
    background: rgba(17, 24, 39, 0.86);
    color: rgba(255, 255, 255, 0.95);
    font-size: 13px;
    font-weight: 800;
    box-shadow:
      0 1px 2px rgba(0, 0, 0, 0.12),
      0 10px 30px rgba(0, 0, 0, 0.22);
    z-index: 50;
  }

  .segmented {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 6px;
    background: rgba(2, 6, 23, 0.06);
    border-radius: var(--radius-lg);
    padding: 6px;
  }

  .segmented button {
    border: none;
    background: transparent;
    border-radius: var(--radius-md);
    padding: 10px 8px;
    font-weight: 800;
    font-size: 13px;
  }

  .segmented button.active {
    background: var(--surface);
    box-shadow: var(--shadow-sm);
    border: 1px solid var(--border);
  }

  .conn-status {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    border-radius: 999px;
    padding: 6px 10px;
    font-size: 12px;
    border: 1px solid var(--border);
    background: var(--surface-2);
  }

  .conn-dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: rgba(100, 116, 139, 0.8);
  }

  .conn-status[data-kind="connected"] {
    background: rgba(34, 197, 94, 0.12);
    border-color: rgba(34, 197, 94, 0.28);
    color: #065f46;
  }

  .conn-status[data-kind="connected"] .conn-dot {
    background: rgba(34, 197, 94, 0.95);
  }

  .conn-status[data-kind="checking"],
  .conn-status[data-kind="connecting"] {
    background: rgba(37, 99, 235, 0.12);
    border-color: rgba(37, 99, 235, 0.28);
    color: #1d4ed8;
  }

  .conn-status[data-kind="checking"] .conn-dot,
  .conn-status[data-kind="connecting"] .conn-dot {
    background: rgba(37, 99, 235, 0.95);
  }

  .conn-status[data-kind="error"] {
    background: rgba(239, 68, 68, 0.1);
    border-color: rgba(239, 68, 68, 0.22);
    color: #991b1b;
  }

  .conn-status[data-kind="error"] .conn-dot {
    background: rgba(239, 68, 68, 0.9);
  }

  .hidden {
    display: none;
  }

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

  .sessions-list {
    grid-column: 1;
    grid-row: 1 / span 2;
  }

  @media (max-width: 640px) {
    .sessions-list {
      grid-row: auto;
    }
  }

  @media (max-width: 640px) {
    .sessions-layout {
      grid-template-columns: 1fr;
    }

    .sessions-layout.mobile-detail-open .sessions-list {
      display: none;
    }

    .sessions-layout:not(.mobile-detail-open) .sessions-detail,
    .sessions-layout:not(.mobile-detail-open) .sessions-todo {
      display: none;
    }
  }

  .list-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
  }

  .secondary {
    background: var(--surface-2);
  }

  .list-search {
    margin: 10px 0 12px 0;
  }

  .list-search input {
    border-radius: 999px;
    padding-left: 14px;
    padding-right: 14px;
  }

  .host-group {
    margin: 10px 0;
  }

  .host-group-header {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: var(--surface-2);
    text-align: left;
  }

  .host-group-header:hover {
    border-color: var(--border-strong);
  }

  .chevron {
    width: 16px;
    color: var(--muted);
    flex: 0 0 auto;
  }

  .dot {
    width: 10px;
    height: 10px;
    border-radius: 999px;
    background: rgba(100, 116, 139, 0.7);
    flex: 0 0 auto;
  }

  .dot[data-online="1"] {
    background: var(--success);
  }

  .host-name {
    font-weight: 800;
    font-size: 13px;
    flex: 1;
  }

  .host-last-seen {
    font-size: 12px;
    color: var(--muted);
    flex: 0 0 auto;
  }

  .session-items {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 8px;
  }

  .session-item {
    width: 100%;
    text-align: left;
    position: relative;
    padding: 10px 12px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: var(--surface);
    display: flex;
    flex-direction: column;
    gap: 6px;
    box-shadow: var(--shadow-sm);
    transition:
      border-color 160ms ease,
      background-color 160ms ease,
      box-shadow 160ms ease;
  }

  .session-item:hover {
    border-color: var(--border-strong);
    box-shadow: 0 2px 10px rgba(2, 6, 23, 0.09);
  }

  .session-item.selected {
    border-color: rgba(37, 99, 235, 0.35);
    background: rgba(37, 99, 235, 0.08);
  }

  .session-item.selected::before {
    content: "";
    position: absolute;
    left: 0;
    top: 10px;
    bottom: 10px;
    width: 3px;
    border-radius: 999px;
    background: rgba(37, 99, 235, 0.9);
  }

  .session-item-top {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 10px;
  }

  .session-title {
    font-weight: 900;
    font-size: 13px;
    line-height: 1.2;
    color: var(--text-strong);
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .session-status {
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

  .session-status[data-kind="running"] {
    background: rgba(34, 197, 94, 0.12);
    border-color: rgba(34, 197, 94, 0.28);
    color: #065f46;
  }

  .session-status[data-kind="warning"] {
    background: rgba(249, 115, 22, 0.12);
    border-color: rgba(249, 115, 22, 0.28);
    color: #92400e;
  }

  .session-status[data-kind="error"] {
    background: rgba(239, 68, 68, 0.1);
    border-color: rgba(239, 68, 68, 0.22);
    color: #991b1b;
  }

  .session-status[data-kind="done"] {
    background: rgba(107, 114, 128, 0.12);
    border-color: rgba(107, 114, 128, 0.22);
    color: #374151;
  }

  .session-meta {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    align-items: center;
    font-size: 12px;
    color: #334155;
  }

  .session-tool {
    font-weight: 900;
    color: var(--text-strong);
  }

  .session-op,
  .session-op-args {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
    font-size: 12px;
    background: rgba(241, 245, 249, 0.96);
    border: 1px solid rgba(2, 6, 23, 0.08);
    padding: 2px 6px;
    border-radius: 8px;
  }

  .session-summary {
    font-size: 12px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  @media (max-width: 1024px) {
    .session-summary {
      display: none;
    }
  }

  .session-time {
    font-size: 12px;
    color: var(--muted);
  }

  .sessions-detail {
    grid-column: 2;
    grid-row: 1;
  }

  .detail-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 12px;
    flex-wrap: wrap;
  }

  .detail-title {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .detail-title-main {
    font-weight: 900;
    font-size: 15px;
    line-height: 1.2;
  }

  .detail-title-sub {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: center;
    font-size: 12px;
    color: var(--muted);
  }

  .detail-host code {
    font-weight: 900;
    color: var(--text-strong);
  }

  .detail-meta {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    align-items: center;
    font-size: 12px;
  }

  .meta-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    max-width: 100%;
    min-width: 0;
  }

  .meta-k {
    color: var(--muted);
    font-weight: 900;
  }

  .meta-v {
    color: var(--text-strong);
    font-weight: 900;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta-pill-cwd {
    flex: 1 1 360px;
  }

  .detail-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: center;
    justify-content: flex-end;
  }

  .offline-banner {
    margin-top: 10px;
    padding: 10px 12px;
    border-radius: var(--radius-lg);
    border: 1px solid rgba(239, 68, 68, 0.18);
    background: rgba(239, 68, 68, 0.06);
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    align-items: center;
    font-size: 12px;
    font-weight: 900;
    color: #991b1b;
  }

  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(17, 24, 39, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
    z-index: 100;
  }

  .modal {
    width: 100%;
    max-width: 520px;
    border-radius: 18px;
    border: 1px solid var(--border);
    background: rgba(255, 255, 255, 0.98);
    padding: 14px;
    box-shadow:
      0 2px 12px rgba(0, 0, 0, 0.16),
      0 18px 50px rgba(0, 0, 0, 0.22);
  }

  .modal-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .modal-title {
    font-weight: 900;
    font-size: 14px;
  }

  .modal-body {
    margin-top: 10px;
    color: var(--muted);
  }

  .input-textarea {
    width: 100%;
    min-height: 96px;
    padding: 10px 12px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: rgba(255, 255, 255, 0.9);
    font: inherit;
    font-size: 13px;
    color: var(--text-strong);
    box-sizing: border-box;
    resize: vertical;
  }

  .quick-inputs {
    margin-top: 10px;
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .modal-actions {
    margin-top: 12px;
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .approval-meta {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    align-items: center;
  }

  .risk-pill {
    display: inline-flex;
    align-items: center;
    padding: 4px 8px;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    font-weight: 900;
    font-size: 12px;
    text-transform: uppercase;
  }

  .risk-pill[data-kind="read"] {
    background: rgba(37, 99, 235, 0.12);
    border-color: rgba(37, 99, 235, 0.22);
    color: #1d4ed8;
  }

  .risk-pill[data-kind="write"] {
    background: rgba(249, 115, 22, 0.12);
    border-color: rgba(249, 115, 22, 0.28);
    color: #92400e;
  }

  .risk-pill[data-kind="exec"] {
    background: rgba(239, 68, 68, 0.1);
    border-color: rgba(239, 68, 68, 0.22);
    color: #991b1b;
  }

  .risk-pill[data-kind="other"] {
    background: rgba(107, 114, 128, 0.12);
    border-color: rgba(107, 114, 128, 0.22);
    color: #374151;
  }

  .approval-summary {
    margin-top: 10px;
    color: var(--text-strong);
  }

  .approval-summary code {
    font-size: 12px;
  }

  .approval-prompt {
    margin-top: 10px;
    font-size: 13px;
    color: var(--text-strong);
    white-space: pre-wrap;
    word-break: break-word;
  }

  .approval-details {
    margin-top: 10px;
  }

  .approval-details summary {
    cursor: pointer;
    font-weight: 900;
    color: #374151;
  }

  button.danger {
    background: rgba(239, 68, 68, 0.1);
    border-color: rgba(239, 68, 68, 0.26);
    color: #991b1b;
  }

  .detail-tabs {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 12px;
    padding: 6px;
    border-radius: var(--radius-lg);
    background: rgba(2, 6, 23, 0.06);
  }

  .detail-tabs button {
    border: none;
    background: transparent;
    border-radius: 12px;
    padding: 10px 10px;
    font-weight: 800;
    font-size: 13px;
  }

  .detail-tabs button.active {
    background: var(--surface);
    box-shadow: var(--shadow-sm);
    border: 1px solid var(--border);
  }

  .awaiting-banner {
    margin-top: 10px;
    padding: 10px 12px;
    border-radius: var(--radius-lg);
    border: 1px solid rgba(249, 115, 22, 0.25);
    background: rgba(255, 247, 237, 0.92);
  }

  .awaiting-banner-top {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    align-items: center;
  }

  .awaiting-banner-prompt {
    margin-top: 8px;
    font-size: 12px;
    color: #9a3412;
    word-break: break-word;
  }

  .chat-feed {
    margin-top: 12px;
    max-height: clamp(320px, 60vh, 720px);
    overflow: auto;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: var(--surface-muted);
    padding: 12px;
  }

  .chat-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 10px 0;
  }

  .chat-row[data-role="assistant"] {
    align-items: flex-start;
  }

  .chat-row[data-role="user"] {
    align-items: flex-end;
  }

  .chat-row[data-role="system"] {
    align-items: center;
    text-align: center;
  }

  .chat-bubble {
    max-width: 70%;
    padding: 10px 12px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: var(--surface);
    word-break: break-word;
    white-space: pre-wrap;
  }

  .chat-bubble[data-role="assistant"] {
    background: rgba(241, 245, 249, 0.96);
  }

  .chat-bubble[data-role="user"] {
    background: rgba(37, 99, 235, 0.12);
    border-color: rgba(37, 99, 235, 0.22);
  }

  .chat-system {
    max-width: 70%;
    font-size: 12px;
    color: var(--muted);
    word-break: break-word;
  }

  .chat-ts {
    font-size: 11px;
    color: var(--muted);
  }

  .tool-card {
    width: 100%;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: var(--surface-muted);
    padding: 10px 12px;
  }

  .approval-card {
    width: 100%;
    border-radius: var(--radius-lg);
    border: 1px solid rgba(249, 115, 22, 0.25);
    background: rgba(255, 247, 237, 0.92);
    padding: 10px 12px;
  }

  .approval-card-top {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    align-items: center;
  }

  .approval-card-prompt {
    margin-top: 8px;
    font-size: 12px;
    color: #9a3412;
    word-break: break-word;
    white-space: pre-wrap;
  }

  .tool-card summary {
    cursor: pointer;
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
    list-style: none;
  }

  .tool-card summary::-webkit-details-marker {
    display: none;
  }

  .tool-card-body {
    margin-top: 8px;
    text-align: left;
  }

  .tool-card-label {
    font-size: 12px;
    color: var(--muted);
    margin-bottom: 6px;
  }

  .output-toolbar {
    margin-top: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .output-searchbar {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    align-items: center;
  }

  .output-searchbar input {
    flex: 1;
    min-width: 160px;
    border-radius: 999px;
  }

  .output-count {
    font-size: 12px;
    color: var(--muted);
    font-weight: 800;
    padding: 0 4px;
  }

  .output-actions {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    align-items: center;
  }

  .output-feed {
    margin-top: 10px;
    max-height: clamp(320px, 60vh, 720px);
    overflow: auto;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: var(--surface-muted);
    padding: 0;
    position: relative;
  }

  .output-pre {
    margin: 0;
    border: none;
    border-radius: 0;
    background: transparent;
    max-height: none;
  }

  .paused-badge {
    position: absolute;
    top: 10px;
    right: 10px;
    border-radius: 999px;
    padding: 6px 10px;
    font-size: 12px;
    font-weight: 900;
    border: 1px solid rgba(249, 115, 22, 0.28);
    background: rgba(255, 247, 237, 0.94);
    color: #9a3412;
  }

  :global(.out-mark) {
    background: rgba(245, 158, 11, 0.35);
    color: inherit;
    border-radius: 4px;
    padding: 0 1px;
  }

  :global(.out-mark.current) {
    background: rgba(245, 158, 11, 0.7);
  }

  .detail-input {
    margin-top: 12px;
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .sessions-todo {
    grid-column: 2;
    grid-row: 2;
  }

  @media (max-width: 640px) {
    .sessions-detail,
    .sessions-todo {
      grid-column: 1;
      grid-row: auto;
    }
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
    color: #334155;
  }

  section {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 14px;
    box-shadow: var(--shadow-sm);
  }

  label {
    display: block;
    margin: 10px 0;
    font-size: 12px;
    font-weight: 700;
    color: #334155;
  }

  .login-prefs {
    display: flex;
    flex-wrap: wrap;
    gap: 10px 14px;
    margin: 6px 0 2px;
  }

  label.checkbox {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    margin: 0;
    font-size: 13px;
    font-weight: 700;
    color: #334155;
    cursor: pointer;
    user-select: none;
  }

  input,
  select {
    width: 100%;
    min-height: 40px;
    padding: 10px 12px;
    box-sizing: border-box;
    border-radius: var(--radius-md);
    border: 1px solid var(--border);
    background: var(--surface);
    font-size: 14px;
    outline: none;
  }

  input[type="checkbox"] {
    width: 16px;
    height: 16px;
    padding: 0;
    border-radius: 6px;
    accent-color: var(--primary);
  }

  input:focus,
  select:focus {
    border-color: rgba(37, 99, 235, 0.5);
    box-shadow: 0 0 0 4px rgba(37, 99, 235, 0.18);
  }

  button {
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    min-height: 40px;
    padding: 10px 12px;
    background: var(--surface);
    font-weight: 700;
    font-size: 13px;
    cursor: pointer;
    transition:
      border-color 160ms ease,
      background-color 160ms ease,
      box-shadow 160ms ease,
      transform 40ms ease;
  }

  button:hover:not(:disabled) {
    border-color: var(--border-strong);
    box-shadow: var(--shadow-sm);
  }

  button:active:not(:disabled) {
    transform: translateY(1px);
  }

  button:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  button:focus-visible,
  input:focus-visible,
  select:focus-visible,
  summary:focus-visible {
    outline: none;
    box-shadow: 0 0 0 4px rgba(37, 99, 235, 0.18);
  }

  @media (prefers-reduced-motion: reduce) {
    * {
      transition: none !important;
      scroll-behavior: auto !important;
    }
  }

  pre {
    white-space: pre-wrap;
    word-break: break-word;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: var(--surface);
    padding: 12px;
    overflow: auto;
  }

  ul {
    padding-left: 18px;
  }
</style>
