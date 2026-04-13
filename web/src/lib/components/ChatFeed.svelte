<script lang="ts">
  import BlocksRenderer from "./BlocksRenderer.svelte";

  export let uiBlocks: any[] = [];
  export let selectedRun: any = null;
  export let renderMarkdownBasic: (src: string) => string = () => "";
  export let formatAbsTime: (ts: string) => string = () => "";
  export let copyText: (text: string) => void | Promise<void> = () => {};
  export let selectedRunId: string = "";
  export let selectedOutputMode: string = "";
  export let selectedOutput: string = "";
  export let tailLines: (text: string, n: number) => string = () => "";

  export let onSwitchToOutputTab: () => void = () => {};
</script>

<div class="events-tail" style="margin:10px 0 12px">
  <div style="display:flex;gap:8px;align-items:center;justify-content:space-between;flex-wrap:wrap">
    <div style="font-weight:600">输出摘要</div>
    <button
      class="secondary"
      on:click={onSwitchToOutputTab}
      disabled={!selectedRunId}
      type="button"
    >
      打开终端
    </button>
  </div>
  {#if !selectedRunId}
    <div class="subtle"></div>
  {:else if selectedOutputMode === "tui"}
    <div class="subtle">(TUI 终端输出请在"终端"里查看)</div>
  {:else}
    <pre class="output-pre" style="max-height:160px;overflow:auto">{tailLines(selectedOutput || "", 40)}</pre>
  {/if}
</div>
<div class="chat-feed">
  {#if uiBlocks.length === 0}
    <div class="subtle"></div>
  {:else}
    <BlocksRenderer
      blocks={uiBlocks}
      runTool={selectedRun?.tool ?? ""}
      {renderMarkdownBasic}
      {formatAbsTime}
      {copyText}
    />
  {/if}
</div>

<style>
  .chat-feed {
    margin-top: 12px;
    max-height: clamp(320px, 60vh, 720px);
    overflow: auto;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.94), rgba(241, 245, 255, 0.95));
    padding: 12px;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.8);
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
    max-width: 70%;
    padding: 10px 12px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: var(--surface);
    word-break: break-word;
    white-space: pre-wrap;
  }

  :global(.chat-bubble[data-role="assistant"]) {
    background: rgba(248, 250, 255, 0.98);
  }

  :global(.chat-bubble[data-role="user"]) {
    background: rgba(14, 165, 233, 0.16);
    border-color: rgba(14, 165, 233, 0.3);
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
    background: linear-gradient(155deg, rgba(255, 255, 255, 0.92), rgba(241, 245, 255, 0.92));
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
    background: rgba(241, 245, 255, 0.78);
    font-size: 12px;
    white-space: pre-wrap;
    word-break: break-word;
    overflow: auto;
    max-height: 240px;
  }
</style>
