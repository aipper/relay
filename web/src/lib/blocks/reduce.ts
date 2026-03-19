import type { ChatMessageLike, MarkdownBlock, ToolPairBlock, UiBlock } from "./types";

function isRecord(v: unknown): v is Record<string, unknown> {
  return Boolean(v) && typeof v === "object";
}

function toolMetaFromText(kind: string, text: string): { label: string; details: string } {
  const t = (text || "").trim();
  if (kind === "tool.call" && t.startsWith("tool.call ")) {
    const rest = t.slice("tool.call ".length);
    const name = rest.split(/\s+/g)[0] || "tool.call";
    return { label: name, details: t };
  }
  if (kind === "tool.result" && t.startsWith("tool.result ")) {
    const rest = t.slice("tool.result ".length);
    const name = rest.split(/\s+/g)[0] || "tool.result";
    return { label: name, details: t };
  }
  return { label: kind, details: t };
}

function jsonMaybe(v: unknown): string | null {
  if (v === undefined) return null;
  try {
    return JSON.stringify(v, null, 2);
  } catch {
    return null;
  }
}

export type ReduceToBlocksOptions = {
  runTool: string;
  pinnedRequestId: string | null;
  outputMode?: "log" | "tui";
};

function isImportantRunOutput(m: ChatMessageLike): boolean {
  if (m.kind !== "run.output") return false;
  const raw = m.text || "";
  const t = raw.replace(/\x1b\[[0-9;]*m/g, "").toLowerCase();
  const isCodexEvent = raw.includes('"method":"codex/event"');

  if (isCodexEvent) {
    return (
      raw.includes('"type":"stream_error"') ||
      (raw.includes('"type":"mcp_startup_complete"') && raw.includes('"failed":[')) ||
      (raw.includes('"type":"mcp_startup_update"') && raw.toLowerCase().includes('error'))
    );
  }

  return (
    t.includes(" error") ||
    t.startsWith("error") ||
    t.includes("fatal") ||
    t.includes("exception traceback") ||
    t.includes("traceback (most recent call last)") ||
    t.includes("connection closed") ||
    t.includes("bad gateway") ||
    t.includes("unexpectedcontenttype") ||
    t.includes("unexpected content type") ||
    t.includes("missing-content-type") ||
    t.includes("stream_error") ||
    t.includes("reconnecting")
  );
}

export function reduceToBlocks(msgs: ChatMessageLike[], opts: ReduceToBlocksOptions): UiBlock[] {
  const includeOutputInEvents = opts.runTool === "opencode" || opts.outputMode === "log";
  const out: UiBlock[] = [];
  const pinnedReqId = opts.pinnedRequestId || null;

  for (let i = 0; i < msgs.length; i++) {
    const m = msgs[i];

    if (!includeOutputInEvents && m.kind === "run.output" && !isImportantRunOutput(m)) continue;

    if (pinnedReqId && (m.kind === "run.permission_requested" || m.kind === "run.awaiting_input") && m.request_id === pinnedReqId) {
      continue;
    }

    if (m.kind === "tool.call") {
      const n = msgs[i + 1];
      if (n && n.kind === "tool.result" && n.request_id && n.request_id === m.request_id) {
        const callMeta = toolMetaFromText(m.kind, m.text || "");
        const resMeta = toolMetaFromText(n.kind, n.text || "");
        const callData = isRecord(m.data) ? m.data : null;
        const resData = isRecord(n.data) ? n.data : null;
        const label = callData && typeof callData["tool"] === "string" ? String(callData["tool"]) : callMeta.label;
        const okState =
          resData && typeof resData["ok"] === "boolean"
            ? Boolean(resData["ok"])
            : (n.text || "").includes(" ok=true")
              ? true
              : (n.text || "").includes(" ok=false")
                ? false
                : null;

        const callArgsJson = callData && "args" in callData ? jsonMaybe(callData.args) : null;
        const resJson =
          resData && okState === true && "result" in resData
            ? jsonMaybe(resData.result)
            : resData && okState === false && "error" in resData
              ? jsonMaybe(resData.error)
              : null;

        const b: ToolPairBlock = {
          type: "tool_pair",
          id: `tool:${m.request_id ?? m.key}`,
          ts: m.ts,
          actor: m.actor ?? null,
          request_id: m.request_id ?? null,
          label,
          ok: okState,
          call_details: callMeta.details,
          result_details: resMeta.details,
          call_json: callArgsJson,
          result_json: resJson,
          call: { kind: m.kind, text: m.text || "", data: callData },
          result: { kind: n.kind, text: n.text || "", data: resData },
        };

        out.push(b);
        i++;
        continue;
      }
    }

    if (m.kind === "run.output" && m.role === "assistant") {
      const prev = out[out.length - 1];
      if (prev?.type === "markdown" && prev.kind === "run.output" && prev.role === "assistant") {
        (prev as MarkdownBlock).text = `${(prev as MarkdownBlock).text ?? ""}${m.text ?? ""}`;
        continue;
      }
    }

    const mb: MarkdownBlock = {
      type: "markdown",
      id: `msg:${m.key}`,
      ts: m.ts,
      role: m.role,
      kind: m.kind,
      actor: m.actor ?? null,
      request_id: m.request_id ?? null,
      text: m.text || "",
      data: m.data,
    };

    out.push(mb);
  }

  return out;
}
