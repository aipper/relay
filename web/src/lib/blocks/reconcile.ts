import type { UiBlock } from "./types";

function eqNullable(a: string | null | undefined, b: string | null | undefined): boolean {
  return (a ?? null) === (b ?? null);
}

function sameBlock(a: UiBlock, b: UiBlock): boolean {
  if (a.type !== b.type) return false;
  if (a.id !== b.id) return false;

  if (a.type === "markdown" && b.type === "markdown") {
    return (
      a.ts === b.ts &&
      a.role === b.role &&
      a.kind === b.kind &&
      a.text === b.text &&
      eqNullable(a.actor, b.actor) &&
      eqNullable(a.request_id, b.request_id)
    );
  }

  if (a.type === "tool_pair" && b.type === "tool_pair") {
    return (
      a.ts === b.ts &&
      eqNullable(a.actor, b.actor) &&
      eqNullable(a.request_id, b.request_id) &&
      a.label === b.label &&
      a.ok === b.ok &&
      a.call.kind === b.call.kind &&
      a.result.kind === b.result.kind &&
      a.call_details === b.call_details &&
      a.result_details === b.result_details &&
      eqNullable(a.call_json, b.call_json) &&
      eqNullable(a.result_json, b.result_json)
    );
  }

  return false;
}

export function reconcileBlocks(prev: UiBlock[], next: UiBlock[]): UiBlock[] {
  if (prev.length === 0) return next;
  const byId = new Map<string, UiBlock>();
  for (const b of prev) byId.set(b.id, b);

  let changed = prev.length !== next.length;
  const result = next.map((b, idx) => {
    const old = byId.get(b.id);
    if (!old) {
      changed = true;
      return b;
    }
    if (sameBlock(old, b)) {
      if (prev[idx] !== old) changed = true;
      return old;
    }
    changed = true;
    return b;
  });

  return changed ? result : prev;
}
