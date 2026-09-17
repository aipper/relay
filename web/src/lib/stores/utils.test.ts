import { describe, expect, it } from "vitest";
import {
  applyTerminalEdits,
  basename,
  compareTsDesc,
  computeOutputMatches,
  dataBool,
  dataString,
  isProbablyInsecureUrl,
  parseHostToolStatuses,
  sanitizeTerminalOutput,
  statusLabel,
  toWsBase,
  truncateHead,
  truncateTail,
} from "./utils";
import type { RunRow, WsEnvelope } from "./types";

function runRow(over: Partial<RunRow> = {}): RunRow {
  return {
    id: "r1",
    host_id: "h1",
    tool: "opencode",
    cwd: "/tmp/proj",
    status: "running",
    started_at: "2026-01-01T00:00:00Z",
    ...over,
  };
}

function env(data: unknown): WsEnvelope {
  return { type: "run.output", ts: "2026-01-01T00:00:00Z", data };
}

describe("truncateTail", () => {
  it("短串原样返回", () => {
    expect(truncateTail("abc", 10)).toBe("abc");
  });
  it("超长保留尾部", () => {
    expect(truncateTail("abcdef", 3)).toBe("def");
  });
});

describe("truncateHead", () => {
  it("超长保留尾部元素（消息截断上限语义）", () => {
    expect(truncateHead([1, 2, 3, 4, 5], 3)).toEqual([3, 4, 5]);
  });
  it("未超限返回原数组", () => {
    const a = [1, 2];
    expect(truncateHead(a, 5)).toBe(a);
  });
});

describe("sanitizeTerminalOutput", () => {
  it("纯文本原样返回", () => {
    expect(sanitizeTerminalOutput("hello")).toBe("hello");
  });
  it("剥离 ANSI SGR 序列", () => {
    expect(sanitizeTerminalOutput("\x1b[31mred\x1b[0m")).toBe("red");
  });
});

describe("applyTerminalEdits", () => {
  it("无控制字符直接追加", () => {
    expect(applyTerminalEdits("ab", "cd")).toBe("abcd");
  });
  it("\\r 回到行首覆盖", () => {
    expect(applyTerminalEdits("hello", "\rbye")).toBe("bye");
  });
});

describe("toWsBase", () => {
  it("http→ws 且去尾斜杠", () => {
    expect(toWsBase("http://127.0.0.1:8787/")).toBe("ws://127.0.0.1:8787");
  });
  it("https→wss", () => {
    expect(toWsBase("https://example.com")).toBe("wss://example.com");
  });
});

describe("isProbablyInsecureUrl", () => {
  it("公网 http 判为不安全", () => {
    expect(isProbablyInsecureUrl("http://example.com")).toBe(true);
  });
  it("本地回环不判不安全", () => {
    expect(isProbablyInsecureUrl("http://127.0.0.1:8787")).toBe(false);
    expect(isProbablyInsecureUrl("http://localhost:8787")).toBe(false);
  });
  it("https 不判不安全", () => {
    expect(isProbablyInsecureUrl("https://example.com")).toBe(false);
  });
});

describe("dataString / dataBool", () => {
  it("data 非 record 返回 undefined", () => {
    expect(dataString(env(null), "text")).toBeUndefined();
    expect(dataBool(env("str"), "ok")).toBeUndefined();
  });
  it("类型不匹配返回 undefined", () => {
    expect(dataString(env({ text: 1 }), "text")).toBeUndefined();
    expect(dataBool(env({ ok: "yes" }), "ok")).toBeUndefined();
  });
  it("正常取值", () => {
    expect(dataString(env({ text: "hi" }), "text")).toBe("hi");
    expect(dataBool(env({ ok: true }), "ok")).toBe(true);
  });
});

describe("parseHostToolStatuses", () => {
  it("非数组返回空", () => {
    expect(parseHostToolStatuses(null)).toEqual([]);
  });
  it("过滤无 tool 项并清洗 models", () => {
    const out = parseHostToolStatuses([
      { tool: "opencode", ok: true, models: ["m1", " ", 1] },
      { ok: true },
    ]);
    expect(out).toHaveLength(1);
    expect(out[0]!.tool).toBe("opencode");
    expect(out[0]!.models).toEqual(["m1"]);
  });
});

describe("compareTsDesc", () => {
  it("新时间排前（返回正数当 a 更旧）", () => {
    expect(compareTsDesc("2026-01-01T00:00:00Z", "2026-01-02T00:00:00Z")).toBeGreaterThan(0);
    expect(compareTsDesc("2026-01-02T00:00:00Z", "2026-01-01T00:00:00Z")).toBeLessThan(0);
  });
});

describe("basename", () => {
  it("取最后一段", () => {
    expect(basename("/a/b/proj")).toBe("proj");
  });
  it("空/点/根返回空", () => {
    expect(basename("")).toBe("");
    expect(basename(".")).toBe("");
    expect(basename("/")).toBe("");
  });
});

describe("statusLabel", () => {
  it("running→运行中", () => {
    expect(statusLabel(runRow({ status: "running" }))).toEqual({ label: "运行中", kind: "running" });
  });
  it("exited 非零→错误", () => {
    expect(statusLabel(runRow({ status: "exited", exit_code: 1 }))).toEqual({ label: "错误", kind: "error" });
  });
  it("awaiting_approval prompt 原因→待输入", () => {
    expect(statusLabel(runRow({ status: "awaiting_approval", pending_reason: "prompt" }))).toEqual({
      label: "待输入",
      kind: "warning",
    });
  });
});

describe("computeOutputMatches", () => {
  it("空查询返回空", () => {
    expect(computeOutputMatches(["abc"], "  ", "r1")).toEqual([]);
  });
  it("大小写不敏感多处命中", () => {
    const ms = computeOutputMatches(["Foo foo", "bar"], "foo", "r1");
    expect(ms).toHaveLength(2);
    expect(ms[0]).toMatchObject({ line: 0, start: 0, end: 3 });
    expect(ms[1]).toMatchObject({ line: 0, start: 4, end: 7 });
  });
});
