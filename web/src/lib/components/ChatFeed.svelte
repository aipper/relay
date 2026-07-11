<script lang="ts">
  import { onMount, tick } from "svelte";
  import BlocksRenderer from "./BlocksRenderer.svelte";

  let {
    uiBlocks = [] as any[],
    selectedRun = null as any,
    renderMarkdownBasic = ((src: string) => "") as (src: string) => string,
    formatAbsTime = ((ts: string) => "") as (ts: string) => string,
    copyText = ((text: string) => {}) as (text: string) => void,
    selectedRunId = "",
    selectedOutput = "",
    tailLines = ((text: string, n: number) => "") as (text: string, n: number) => string,
    onSendMessage = ((text: string) => {}) as (text: string) => void,
  } = $props();

  let feedEl: HTMLDivElement;
  let autoScroll = true;

  let quickOptions = $derived.by(() => {
    if (uiBlocks.length === 0) return [];
    const last = uiBlocks[uiBlocks.length - 1];
    if (!last || (last.role !== "assistant" && last.kind === "tool.result")) return [];
    const text = (last.text ?? last.result_details ?? "") as string;
    if (!text) return [];
    return extractOptions(text);
  });

  function extractOptions(text: string): string[] {
    const lines = text.split("\n");
    const out: string[] = [];
    for (const line of lines) {
      const trimmed = line.trim();
      if (!trimmed) continue;
      if (/^\s/.test(line)) continue;
      const m1 = trimmed.match(/^(?:\d+)[.)]\s+(.+)/);
      if (m1?.[1]) { out.push(m1[1]); continue; }
      const m2 = trimmed.match(/^[-*]\s+(.+)/);
      if (m2?.[1]) { out.push(m2[1]); continue; }
      const m3 = trimmed.match(/^([A-Za-z\u4e00-\u9fff][\w\u4e00-\u9fff]*)[:：]\s*(.+)/);
      if (m3?.[1] && m3[2]) { out.push(m3[1] + '：' + m3[2]); continue; }
    }
    return out.slice(0, 8);
  }

  onMount(() => {
    scrollToBottom();
  });

  $effect(() => {
    const n = uiBlocks.length;
    if (n > 0 && autoScroll) {
      tick().then(() => scrollToBottom());
    }
  });

  function scrollToBottom() {
    if (!feedEl) return;
    feedEl.scrollTop = feedEl.scrollHeight;
  }

  function handleScroll() {
    if (!feedEl) return;
    const threshold = 80;
    autoScroll = feedEl.scrollHeight - feedEl.scrollTop - feedEl.clientHeight < threshold;
  }
</script>

<div class="chat-feed" bind:this={feedEl} onscroll={handleScroll}>
  {#if uiBlocks.length === 0}
    <div class="empty-feed">
      <p>消息将在对话中显示</p>
    </div>
  {:else}
    <BlocksRenderer
      blocks={uiBlocks}
      runTool={selectedRun?.tool ?? ""}
      {renderMarkdownBasic}
      {formatAbsTime}
      {copyText}
    />
  {/if}

  {#if quickOptions.length > 0}
    <div class="quick-options">
      {#each quickOptions as opt, i (i)}
        <button class="opt-btn" onclick={() => onSendMessage(opt)} type="button">
          {opt}
        </button>
      {/each}
    </div>
  {/if}
</div>

{#if !autoScroll && uiBlocks.length > 0}
  <button class="scroll-btn" onclick={scrollToBottom} type="button">↓ 最新</button>
{/if}

<style>
  .chat-feed {
    margin-top: 12px;
    max-height: clamp(320px, 60vh, 720px);
    overflow: auto;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: var(--bg-surface);
    padding: 12px;
  }

  .empty-feed {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 32px 16px;
    color: var(--muted);
    gap: 12px;
  }

  .empty-feed p {
    margin: 0;
    font-size: 14px;
  }

  .scroll-btn {
    position: fixed;
    bottom: 80px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 20;
    padding: 8px 16px;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: var(--bg-surface);
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    box-shadow: var(--shadow-md);
  }

  .scroll-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }

  .quick-options {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 12px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }

  .opt-btn {
    padding: 8px 14px;
    border-radius: 999px;
    border: 1px solid var(--border-strong);
    background: color-mix(in srgb, var(--accent) 8%, var(--bg-surface));
    color: var(--accent);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: background 150ms ease;
  }

  .opt-btn:hover {
    background: color-mix(in srgb, var(--accent) 20%, var(--bg-surface));
  }

  :global(.chat-row) {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 10px 0;
  }

  :global(.chat-row[data-role="assistant"]) {
    align-items: flex-start;
  }

  :global(.chat-row[data-role="user"]) {
    align-items: flex-end;
  }

  :global(.chat-row[data-role="system"]) {
    align-items: center;
    text-align: center;
  }

  :global(.chat-bubble) {
    position: relative;
    max-width: 70%;
    padding: 10px 12px;
    border-radius: 16px;
    border: 1px solid var(--border);
    background: var(--surface);
    word-break: break-word;
    white-space: pre-wrap;
    line-height: 1.5;
  }

  :global(.chat-bubble[data-role="assistant"]) {
    background: var(--bg-surface);
    border-bottom-left-radius: 4px;
  }

  :global(.chat-bubble[data-role="user"]) {
    background: color-mix(in srgb, var(--accent) 20%, var(--bg-surface));
    border-color: color-mix(in srgb, var(--accent) 35%, transparent);
    border-bottom-right-radius: 4px;
  }

  :global(.chat-bubble:hover .copy-btn) {
    opacity: 1;
  }

  :global(.copy-btn) {
    position: absolute;
    top: 4px;
    right: 4px;
    opacity: 0;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 4px;
    cursor: pointer;
    color: var(--muted);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: opacity 150ms ease, color 150ms ease;
    z-index: 2;
  }

  :global(.copy-btn:active),
  :global(.copy-btn:focus-visible) {
    opacity: 1;
    color: var(--accent);
  }

  :global(.chat-system) {
    max-width: 70%;
    font-size: 12px;
    color: var(--muted);
    word-break: break-word;
  }

  :global(.chat-ts) {
    font-size: 11px;
    color: var(--muted);
  }

  :global(.tool-card) {
    width: 100%;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: var(--bg-surface);
    padding: 10px 12px;
  }

  :global(.tool-card summary) {
    cursor: pointer;
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
    list-style: none;
  }

  :global(.tool-card summary::-webkit-details-marker) {
    display: none;
  }

  :global(.tool-card-body) {
    margin-top: 8px;
    text-align: left;
  }

  :global(.tool-card-actions) {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    margin-bottom: 10px;
  }

  :global(.tool-card-label) {
    font-size: 12px;
    color: var(--muted);
    margin-bottom: 6px;
  }

  :global(.tool-json) {
    margin: 0;
    padding: 10px 12px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: var(--bg-canvas);
    font-size: 12px;
    white-space: pre-wrap;
    word-break: break-word;
    overflow: auto;
    max-height: 240px;
  }
</style>
