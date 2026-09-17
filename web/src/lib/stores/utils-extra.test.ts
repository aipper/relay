import { describe, expect, it } from "vitest";
import {
  codexStructuredEventText,
  connLabel,
  dataAny,
  formatAbsTime,
  formatRelativeTime,
  isLikelyTuiToolName,
  isRecord,
  looksLikeTuiAnsi,
  renderOutputHtml,
  sessionSummary,
  sessionTitle,
  uid,
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

describe("uid", () => {
  it("返回非空字符串", () => {
    expect(typeof uid()).toBe("string");
    expect(uid().length).toBeGreaterThan(0);
  });
  it("多次调用唯一", () => {
    const a = uid();
    const b = uid();
    expect(a).not.toBe(b);
  });
});

describe("dataAny", () => {
  it("正常取值", () => {
    expect(dataAny(env({ k: "v" }), "k")).toBe("v");
    expect(dataAny(env({ k: { a: 1 } }), "k")).toEqual({ a: 1 });
  });
  it("缺 key 返回 undefined", () => {
    expect(dataAny(env({ a: 1 }), "missing")).toBeUndefined();
    expect(dataAny(env(null), "k")).toBeUndefined();
  });
});

describe("isRecord", () => {
  it("普通对象返回 true", () => {
    expect(isRecord({})).toBe(true);
    expect(isRecord({ a: 1 })).toBe(true);
  });
  it("null/原始值返回 false", () => {
    expect(isRecord(null)).toBe(false);
    expect(isRecord(undefined)).toBe(false);
    expect(isRecord(42)).toBe(false);
    expect(isRecord("str")).toBe(false);
  });
});

describe("formatRelativeTime", () => {
  it("空/非法返回空串", () => {
    expect(formatRelativeTime("")).toBe("");
    expect(formatRelativeTime(null)).toBe("");
    expect(formatRelativeTime("not-a-time")).toBe("");
  });
  it("5秒前返回刚刚", () => {
    expect(formatRelativeTime(new Date(Date.now() - 5000).toISOString())).toBe("刚刚");
  });
  it("90秒前返回分钟前后缀", () => {
    expect(formatRelativeTime(new Date(Date.now() - 90_000).toISOString())).toMatch(/分钟前$/);
  });
  it("昨天返回天前后缀", () => {
    expect(formatRelativeTime(new Date(Date.now() - 25 * 3600_000).toISOString())).toMatch(/天前$/);
  });
});

describe("formatAbsTime", () => {
  it("合法时间返回非空且不等于原串", () => {
    const out = formatAbsTime("2026-01-01T00:00:00Z");
    expect(out.length).toBeGreaterThan(0);
    expect(out).not.toBe("2026-01-01T00:00:00Z");
  });
  it("非法串原样返回", () => {
    expect(formatAbsTime("not-a-time")).toBe("not-a-time");
  });
});

describe("connLabel", () => {
  it("全映射", () => {
    expect(connLabel("connected")).toBe("已连接");
    expect(connLabel("checking")).toBe("检查中");
    expect(connLabel("connecting")).toBe("连接中");
    expect(connLabel("disconnected")).toBe("未连接");
    expect(connLabel("error")).toBe("错误");
  });
  it("未知串原样", () => {
    expect(connLabel("weird-state")).toBe("weird-state");
  });
});

describe("sessionTitle", () => {
  it("取 cwd 最后一段", () => {
    expect(sessionTitle(runRow({ cwd: "/a/b/proj" }))).toBe("proj");
  });
  it("空 cwd 返回空", () => {
    expect(sessionTitle(runRow({ cwd: "" }))).toBe("");
  });
});

describe("sessionSummary", () => {
  it("短串原样", () => {
    expect(sessionSummary(runRow({ cwd: "/tmp/proj" }))).toBe("/tmp/proj");
  });
  it("超 60 字符截断加省略号", () => {
    const long = `/tmp/${"a".repeat(70)}`;
    const out = sessionSummary(runRow({ cwd: long }));
    expect(out.endsWith("…")).toBe(true);
    expect(out.length).toBe(61);
  });
});

describe("looksLikeTuiAnsi", () => {
  it("空串返回 false", () => {
    expect(looksLikeTuiAnsi("")).toBe(false);
  });
  it("无 ESC 返回 false", () => {
    expect(looksLikeTuiAnsi("plain text")).toBe(false);
  });
  it("含清屏序列返回 true", () => {
    expect(looksLikeTuiAnsi("\x1b[2Jhello")).toBe(true);
  });
});

describe("isLikelyTuiToolName", () => {
  it("codex/gemini 返回 true", () => {
    expect(isLikelyTuiToolName("codex")).toBe(true);
    expect(isLikelyTuiToolName("gemini")).toBe(true);
  });
  it("其他返回 false", () => {
    expect(isLikelyTuiToolName("opencode")).toBe(false);
    expect(isLikelyTuiToolName("")).toBe(false);
  });
});

describe("codexStructuredEventText", () => {
  it("合法 codex/event 返回含标记且换行结尾", () => {
    const raw = JSON.stringify({
      method: "codex/event",
      params: { msg: { type: "agent_message", message: "hello" } },
    });
    const out = codexStructuredEventText(raw);
    expect(out).toContain("[codex:agent_message]");
    expect(out!.endsWith("\n")).toBe(true);
  });
  it("非 JSON 返回 null", () => {
    expect(codexStructuredEventText("not json")).toBeNull();
  });
  it("非法 method 返回 null", () => {
    expect(codexStructuredEventText(JSON.stringify({ method: "other", params: {} }))).toBeNull();
  });
});

describe("renderOutputHtml", () => {
  it("无 match 时转义尖括号", () => {
    const out = renderOutputHtml(["<div>hi</div>"], [], 0);
    expect(out).toContain("&lt;div&gt;");
    expect(out).not.toContain("<div>");
  });
  it("有 match 时含高亮且原文转义", () => {
    const out = renderOutputHtml(["<a>foo</a>"], [{ id: "out-match-r1-0", line: 0, start: 3, end: 6 }], 0);
    expect(out).toContain("<mark");
    expect(out).toContain("&lt;");
  });
});
