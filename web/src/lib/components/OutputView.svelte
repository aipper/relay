<script lang="ts">
  export let outputAutoScroll: boolean = false;
  export let selectedRunId: string = "";
  export let status: string = "";
  export let outputSearchText: string = "";
  export let outputSearchMatches: any[] = [];
  export let outputSearchCursor: number = 0;
  export let outputSearchActive: boolean = false;
  export let outputHtml: string = "";
  export let outputDisplayText: string = "";
  export let outputIsAtBottom: boolean = false;

  export let onToggleOutputAutoScroll: () => void = () => {};
  export let onQueueStdin: (data: string) => void = () => {};
  export let onRunOutputSearch: () => void = () => {};
  export let onPrevOutputMatch: () => void = () => {};
  export let onNextOutputMatch: () => void = () => {};
  export let onClearOutputSearch: () => void = () => {};
  export let onCopyOutput: () => void = () => {};
  export let onJumpToLatest: () => void = () => {};
  export let onResumeOutputAutoScroll: () => void = () => {};
  export let onOpenInputModal: (text?: string) => void = () => {};
  export let onOutputScroll: () => void = () => {};
  export let onSearchKeydown: (e: KeyboardEvent) => void = () => {};

  let outputSearchInputEl: HTMLInputElement;
  let outputFeedEl: HTMLDivElement;
</script>

<div class="output-toolbar">
  <div class="output-searchbar">
    <input
      bind:this={outputSearchInputEl}
      bind:value={outputSearchText}
      on:keydown={onSearchKeydown}
      placeholder=""
    />
    <button on:click={onRunOutputSearch} disabled={!outputSearchText.trim()}>搜索</button>
    <button on:click={onPrevOutputMatch} disabled={outputSearchMatches.length === 0}>↑</button>
    <button on:click={onNextOutputMatch} disabled={outputSearchMatches.length === 0}>↓</button>
    {#if outputSearchActive}
      <div class="output-count">
        {outputSearchMatches.length === 0 ? "0/0" : `${outputSearchCursor + 1}/${outputSearchMatches.length}`}
      </div>
    {/if}
    <button on:click={onClearOutputSearch} disabled={!outputSearchText && !outputSearchActive}>清空</button>
  </div>
  <div class="output-actions">
    <button on:click={onToggleOutputAutoScroll} disabled={!selectedRunId}>
      {outputAutoScroll ? "暂停" : "继续"}
    </button>
    {#if !outputAutoScroll && !outputIsAtBottom}
      <button on:click={onJumpToLatest} disabled={!selectedRunId}>跳到最新</button>
    {/if}
    <button on:click={onCopyOutput} disabled={!outputDisplayText}>复制输出</button>
  </div>
</div>
<div class="output-feed" bind:this={outputFeedEl} on:scroll={onOutputScroll}>
  {#if outputSearchActive}
    <pre class="output-pre">{@html outputHtml}</pre>
  {:else}
    <pre class="output-pre">{outputDisplayText}</pre>
  {/if}
  {#if !outputAutoScroll}
    <button class="paused-badge" on:click={onResumeOutputAutoScroll} type="button">已暂停</button>
  {/if}
</div>

<div class="detail-input">
  <button class="secondary" on:click={() => onQueueStdin("\x03")} disabled={!selectedRunId || status !== "connected"} type="button">
    Ctrl+C
  </button>
  <button class="secondary" on:click={onOpenInputModal} disabled={!selectedRunId || status !== "connected"} type="button">
    输入
  </button>
</div>

<style>
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
    color: var(--warning);
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
</style>
