import { describe, expect, it } from "vitest";
import { reconcileBlocks } from "./reconcile";
import type { MarkdownBlock, ToolPairBlock } from "./types";

function md(id: string, text: string): MarkdownBlock {
  return {
    type: "markdown",
    id,
    ts: "2026-01-01T00:00:00Z",
    role: "assistant",
    kind: "chat",
    actor: null,
    request_id: null,
    text,
  };
}

function tp(id: string, label: string, ok: boolean | null = true): ToolPairBlock {
  return {
    type: "tool_pair",
    id,
    ts: "2026-01-01T00:00:00Z",
    actor: null,
    request_id: null,
    label,
    ok,
    call_details: "call",
    result_details: "result",
    call_json: null,
    result_json: null,
    call: { kind: "tool.call", text: "call", data: null },
    result: { kind: "tool.result", text: "result", data: null },
  };
}

describe("reconcileBlocks", () => {
  it("完全相同返回 prev 同一引用", () => {
    const prev = [md("msg:a", "hi"), tp("tool:r1", "bash")];
    const next = [md("msg:a", "hi"), tp("tool:r1", "bash")];
    const out = reconcileBlocks(prev, next);
    expect(out).toBe(prev);
    expect(out[0]).toBe(prev[0]);
    expect(out[1]).toBe(prev[1]);
  });

  it("内容变更返回新块", () => {
    const prev = [md("msg:a", "hi")];
    const next = [md("msg:a", "bye")];
    const out = reconcileBlocks(prev, next);
    expect(out).not.toBe(prev);
    expect(out[0]).toBe(next[0]);
  });

  it("新 id 块加入", () => {
    const prev = [md("msg:a", "hi")];
    const next = [md("msg:a", "hi"), md("msg:b", "new")];
    const out = reconcileBlocks(prev, next);
    expect(out).not.toBe(prev);
    expect(out[0]).toBe(prev[0]);
    expect(out[1]).toBe(next[1]);
  });

  it("prev 为空返回 next", () => {
    const next = [md("msg:a", "hi")];
    expect(reconcileBlocks([], next)).toBe(next);
  });

  it("顺序变化标记 changed", () => {
    const prev = [md("msg:a", "hi"), md("msg:b", "yo")];
    const next = [md("msg:b", "yo"), md("msg:a", "hi")];
    const out = reconcileBlocks(prev, next);
    expect(out).not.toBe(prev);
    expect(out[0]).toBe(prev[1]);
    expect(out[1]).toBe(prev[0]);
  });
});
