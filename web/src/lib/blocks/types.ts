export type ChatMessageLike = {
  key: string;
  ts: string;
  role: string;
  kind: string;
  actor?: string | null;
  request_id?: string | null;
  text: string;
  data?: unknown;
};

export type ToolPairBlock = {
  type: "tool_pair";
  id: string;
  ts: string;
  actor?: string | null;
  request_id?: string | null;
  label: string;
  ok: boolean | null;
  call_details: string;
  result_details: string;
  call_json: string | null;
  result_json: string | null;
  call: {
    kind: string;
    text: string;
    data: Record<string, unknown> | null;
  };
  result: {
    kind: string;
    text: string;
    data: Record<string, unknown> | null;
  };
};

export type MarkdownBlock = {
  type: "markdown";
  id: string;
  ts: string;
  role: string;
  kind: string;
  actor?: string | null;
  request_id?: string | null;
  text: string;
  data?: unknown;
};

export type UiBlock = ToolPairBlock | MarkdownBlock;
