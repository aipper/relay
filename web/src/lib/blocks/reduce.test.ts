import { describe, expect, it } from "vitest";
import { reduceToBlocks } from "./reduce";
import type { ChatMessageLike } from "./types";

let seq = 0;
function m(over: Partial<ChatMessageLike> = {}): ChatMessageLike {
  seq += 1;
  return {
    key: `k${seq}`,
    ts: "2026-01-01T00:00:00Z",
    role: "assistant",
    kind: "chat",
    text: "",
    request_id: null,
    actor: null,
    data: undefined,
    ...over,
  };
}

describe("reduceToBlocks tool_pair", () => {
  it("data.ok 布尔决定 ok 三态", () => {
    const msgs = [
      m({ kind: "tool.call", text: "tool.call bash exec", request_id: "r1", data: { tool: "bash", args: { cmd: "ls" } } }),
      m({ kind: "tool.result", text: "tool.result bash done", request_id: "r1", data: { ok: true, result: "out" } }),
    ];
    const out = reduceToBlocks(msgs, { runTool: "codex", pinnedRequestId: null });
    expect(out).toHaveLength(1);
    const b = out[0]!;
    expect(b.type).toBe("tool_pair");
    if (b.type !== "tool_pair") throw new Error("bad type");
    expect(b.id).toBe("tool:r1");
    expect(b.label).toBe("bash");
    expect(b.ok).toBe(true);
    expect(b.call_json).toContain("ls");
    expect(b.result_json).toContain("out");
  });

  it("data.ok=false 取 error 字段", () => {
    const msgs = [
      m({ kind: "tool.call", text: "tool.call bash exec", request_id: "r2", data: { tool: "bash", args: {} } }),
      m({ kind: "tool.result", text: "tool.result bash fail", request_id: "r2", data: { ok: false, error: "boom" } }),
    ];
    const out = reduceToBlocks(msgs, { runTool: "codex", pinnedRequestId: null });
    expect(out[0]!.type).toBe("tool_pair");
    if (out[0]!.type !== "tool_pair") throw new Error("bad type");
    expect(out[0]!.ok).toBe(false);
    expect(out[0]!.result_json).toContain("boom");
  });

  it("文本 ok=true / ok=false / null", () => {
    const t = (text: string) =>
      reduceToBlocks(
        [
          m({ kind: "tool.call", text: "tool.call x", request_id: "rx" }),
          m({ kind: "tool.result", text, request_id: "rx" }),
        ],
        { runTool: "codex", pinnedRequestId: null },
      )[0]!;
    const a = t("tool.result x ok=true done");
    const b = t("tool.result x ok=false fail");
    const c = t("tool.result x pending");
    if (a.type !== "tool_pair" || b.type !== "tool_pair" || c.type !== "tool_pair") throw new Error("bad type");
    expect(a.ok).toBe(true);
    expect(b.ok).toBe(false);
    expect(c.ok).toBeNull();
  });

  it("unpaired tool.call 落 markdown", () => {
    const msgs = [m({ kind: "tool.call", text: "tool.call lone", request_id: "lone", data: { tool: "x" } })];
    const out = reduceToBlocks(msgs, { runTool: "codex", pinnedRequestId: null });
    expect(out).toHaveLength(1);
    expect(out[0]).toMatchObject({ type: "markdown", kind: "tool.call", text: "tool.call lone" });
  });

  it("request_id 不一致不配对", () => {
    const msgs = [
      m({ kind: "tool.call", text: "tool.call a", request_id: "a" }),
      m({ kind: "tool.result", text: "tool.result b", request_id: "b" }),
    ];
    const out = reduceToBlocks(msgs, { runTool: "codex", pinnedRequestId: null });
    expect(out).toHaveLength(2);
    expect(out[0]!.type).toBe("markdown");
    expect(out[1]!.type).toBe("markdown");
  });
});

describe("reduceToBlocks run.output", () => {
  it("tui 模式过滤非重要输出", () => {
    const msgs = [m({ kind: "run.output", text: "hello world", role: "assistant" })];
    const out = reduceToBlocks(msgs, { runTool: "codex", pinnedRequestId: null, outputMode: "tui" });
    expect(out).toHaveLength(0);
  });

  it("tui 模式保留重要输出", () => {
    const msgs = [m({ kind: "run.output", text: "fatal error boom", role: "assistant" })];
    const out = reduceToBlocks(msgs, { runTool: "codex", pinnedRequestId: null, outputMode: "tui" });
    expect(out).toHaveLength(1);
    expect(out[0]).toMatchObject({ type: "markdown", text: "fatal error boom" });
  });

  it("log / opencode 模式保留非重要输出", () => {
    const msgs = [m({ kind: "run.output", text: "hello world", role: "assistant" })];
    const viaLog = reduceToBlocks(msgs, { runTool: "codex", pinnedRequestId: null, outputMode: "log" });
    const viaOpencode = reduceToBlocks(msgs, { runTool: "opencode", pinnedRequestId: null });
    expect(viaLog).toHaveLength(1);
    expect(viaOpencode).toHaveLength(1);
  });

  it("assistant 连续 run.output 合并", () => {
    const msgs = [
      m({ kind: "run.output", text: "hello ", role: "assistant" }),
      m({ kind: "run.output", text: "world", role: "assistant" }),
    ];
    const out = reduceToBlocks(msgs, { runTool: "opencode", pinnedRequestId: null });
    expect(out).toHaveLength(1);
    expect(out[0]).toMatchObject({ type: "markdown", text: "hello world" });
  });
});

describe("reduceToBlocks pinned + label", () => {
  it("pinned request 的 permission_requested/awaiting_input 被跳过", () => {
    const msgs = [
      m({ kind: "run.permission_requested", text: "perm", request_id: "pin-1" }),
      m({ kind: "run.awaiting_input", text: "await", request_id: "pin-1" }),
      m({ kind: "run.permission_requested", text: "other", request_id: "pin-2" }),
    ];
    const out = reduceToBlocks(msgs, { runTool: "opencode", pinnedRequestId: "pin-1" });
    expect(out).toHaveLength(1);
    expect(out[0]).toMatchObject({ kind: "run.permission_requested", text: "other" });
  });

  it("toolMeta label 提取：文本首词 / data.tool 优先 / 回退 kind", () => {
    const pair = (callText: string, callData: unknown) =>
      reduceToBlocks(
        [
          m({ kind: "tool.call", text: callText, request_id: "rl", data: callData }),
          m({ kind: "tool.result", text: "tool.result mytool ok=true", request_id: "rl", data: { ok: true } }),
        ],
        { runTool: "opencode", pinnedRequestId: null },
      )[0]!;
    const fromText = pair("tool.call mytool arg1 arg2", undefined);
    const fromData = pair("tool.call mytool arg1", { tool: "real-tool" });
    const fallback = pair("weird text", undefined);
    if (fromText.type !== "tool_pair" || fromData.type !== "tool_pair" || fallback.type !== "tool_pair")
      throw new Error("bad type");
    expect(fromText.label).toBe("mytool");
    expect(fromData.label).toBe("real-tool");
    expect(fallback.label).toBe("tool.call");
  });
});
