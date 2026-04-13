<script lang="ts">
  import ApprovalCard from "./ApprovalCard.svelte";
  import AwaitingCard from "./AwaitingCard.svelte";
  import ChatFeed from "./ChatFeed.svelte";
  import ChatInput from "./ChatInput.svelte";
  import OutputView from "./OutputView.svelte";

  export let selectedRun: any = null;
  export let selectedRunId: string = "";
  export let isMobile: boolean = false;
  export let token: string = "";
  export let status: string = "";
  export let sessionDetailTab: string = "output";

  // Data helpers
  export let statusLabel: (r: any) => any = () => ({ kind: "", label: "" });
  export let hostsById: Record<string, any> = () => ({});
  export let sessionTitle: (r: any) => string = () => "";
  export let formatRelativeTime: (ts: string | null) => string = () => "";

  // Awaiting state
  export let selectedAwaiting: any = null;
  export let approvalForSession: boolean = false;
  export let approvalAnswersJson: string = "";
  export let awaitingIsApproval: (a: any) => boolean = () => false;
  export let awaitingIsPrompt: (a: any) => boolean = () => false;
  export let awaitingWantsYesNo: (a: any) => boolean = () => false;

  // Chat state
  export let chatInputText: string = "";
  export let chatInputEl: any = null;
  export let uiBlocks: any[] = [];
  export let renderMarkdownBasic: (src: string) => string = () => "";
  export let formatAbsTime: (ts: string) => string = () => "";
  export let copyText: (text: string) => void | Promise<void> = () => {};
  export let selectedOutputMode: string = "";
  export let selectedOutput: string = "";
  export let tailLines: (text: string, n: number) => string = () => "";
  export let selectedRunReady: boolean = false;

  // Output state
  export let outputAutoScroll: boolean = false;
  export let outputSearchText: string = "";
  export let outputSearchMatches: any[] = [];
  export let outputSearchCursor: number = 0;
  export let outputSearchActive: string = "";
  export let outputHtml: string = "";
  export let outputDisplayText: string = "";
  export let outputIsAtBottom: boolean = false;
  export let xtermRef: any = null;
  export let xtermRunId: string = "";

  // Callbacks - Session actions
  export let onBack: () => void = () => {};
  export let onOpenApprovalModal: () => void = () => {};
  export let onOpenStopConfirm: () => void = () => {};
  export let onSendStop: (signal: string) => void = () => {};

  // Callbacks - Tabs
  export let onSwitchToOutputTabAction: () => void = () => {};
  export let onSwitchToMessagesTab: () => void = () => {};
  export let onRefreshMessages: () => void = () => {};
  export let onFocusOutputSearch: () => void = () => {};

  // Callbacks - Chat
  export let onSendChatInput: () => void = () => {};
  export let onOpenInputModal: (text: string) => void = () => {};
  export let onHandleChatInputKeydown: (e: KeyboardEvent) => void = () => {};

  // Callbacks - Decision
  export let onSendDecision: (decision: string) => void = () => {};
  export let onToggleApprovalForSession: (v: boolean) => void = () => {};

  // Callbacks - Output
  export let onToggleOutputAutoScroll: () => void = () => {};
  export let onQueueStdin: (data: string) => void = () => {};
  export let onRunOutputSearch: () => void = () => {};
  export let onPrevOutputMatch: () => void = () => {};
  export let onNextOutputMatch: () => void = () => {};
  export let onClearOutputSearch: () => void = () => {};
  export let onCopyOutput: () => void = () => {};
  export let onJumpToLatest: () => void = () => {};
  export let onResumeOutputAutoScroll: () => void = () => {};
  export let onOutputScroll: () => void = () => {};
  export let onXtermReady: (e: CustomEvent) => void = () => {};
  export let onXtermData: (e: CustomEvent) => void = () => {};
  export let onXtermResize: (e: CustomEvent) => void = () => {};
  export let onSearchKeydown: (e: KeyboardEvent) => void = () => {};

  // Callbacks - Offline
  export let onResumeFromStoredToken: () => void = () => {};
  export let onRefreshSelectedSession: () => void = () => {};
</script>

{#if !selectedRun}
  <div class="subtle"></div>
{:else}
  {@const st = statusLabel(selectedRun)}
  {@const host = hostsById[selectedRun.host_id] ?? null}
  {@const title = sessionTitle(selectedRun)}
  <div class="detail-head">
    <div class="detail-title">
      {#if title}
        <div class="detail-title-main">{title}</div>
      {/if}
      <div class="detail-title-sub">
        <span class="dot" data-online={host?.online ? "1" : "0"} aria-hidden="true"></span>
        <span class="detail-host"><code>{selectedRun.host_id}</code></span>
        <span class="subtle">最近 {formatRelativeTime(host?.last_seen_at ?? null)}</span>
        <span class="subtle">活跃 {formatRelativeTime(selectedRun.last_active_at ?? selectedRun.started_at)}</span>
      </div>
      <div class="detail-meta">
        <span class="meta-pill">
          <span class="meta-k">tool</span>
          <span class="meta-v">{selectedRun.tool}</span>
        </span>
        <span class="meta-pill">
          <span class="meta-k">run</span>
          <span class="meta-v"><code>{selectedRun.id}</code></span>
        </span>
        {#if selectedRun.tool === "opencode" && selectedRun.opencode_session_id}
          <span class="meta-pill">
            <span class="meta-k">opencode</span>
            <span class="meta-v"><code>{selectedRun.opencode_session_id}</code></span>
          </span>
        {/if}
        <span class="meta-pill meta-pill-cwd">
          <span class="meta-k">cwd</span>
          <span class="meta-v"><code>{selectedRun.cwd}</code></span>
        </span>
      </div>
    </div>
    <div class="detail-actions">
      {#if isMobile}
        <button class="secondary" on:click={onBack} type="button">返回</button>
      {/if}
      <span class="session-status" data-kind={st.kind}>{st.label}</span>
      {#if selectedAwaiting && awaitingIsApproval(selectedAwaiting)}
        <button on:click={onOpenApprovalModal} disabled={!selectedRunId} type="button">审批</button>
      {/if}
      <button class="secondary" on:click={() => onSendStop("int")} disabled={!selectedRunId || status !== "connected"} type="button" title="Ctrl+C">中断</button>
      <button on:click={onOpenStopConfirm} disabled={!selectedRunId || status !== "connected"}>停止</button>
    </div>
  </div>

  {#if token && status !== "connected"}
    <div class="offline-banner">
      <span class="dot" data-online="0" aria-hidden="true"></span>
      <span>离线</span>
      <button class="secondary" on:click={onResumeFromStoredToken} type="button">重连</button>
      <button class="secondary" on:click={onRefreshSelectedSession} disabled={!selectedRunId} type="button">刷新</button>
    </div>
  {/if}

  <div class="detail-tabs" role="tablist" aria-label="session detail tabs">
    <button class:active={sessionDetailTab === "output"} role="tab" on:click={onSwitchToOutputTabAction}>终端</button>
    <button class:active={sessionDetailTab === "messages"} role="tab" on:click={onSwitchToMessagesTab}>事件</button>
    <button on:click={onRefreshMessages} disabled={!selectedRunId || !token} style="margin-left:auto">刷新</button>
  </div>

  {#if sessionDetailTab === "messages"}
    {#if selectedAwaiting}
      <div class="pinned-actions">
        {#if awaitingIsApproval(selectedAwaiting)}
          <ApprovalCard
            awaiting={selectedAwaiting}
            runTool={selectedRun.tool}
            {status}
            {approvalForSession}
            bind:approvalAnswersJson
            {onSendDecision}
            {onToggleApprovalForSession}
          />
        {:else if awaitingIsPrompt(selectedAwaiting)}
          <AwaitingCard
            awaiting={selectedAwaiting}
            {status}
            onSendInput={(text) => onSendDecision(text)}
          />
        {/if}
      </div>
    {/if}

    <ChatFeed
      {uiBlocks}
      {selectedRun}
      {renderMarkdownBasic}
      {formatAbsTime}
      {copyText}
      {selectedRunId}
      {selectedOutputMode}
      {selectedOutput}
      {tailLines}
      onSwitchToOutputTab={onSwitchToOutputTabAction}
    />
    <ChatInput
      {selectedRunId}
      {status}
      {selectedRunReady}
      bind:chatInputText
      bind:chatInputEl
      {selectedAwaiting}
      {awaitingIsApproval}
      {awaitingWantsYesNo}
      onSendChatInput={onSendChatInput}
      onOpenInputModal={onOpenInputModal}
      onKeydown={onHandleChatInputKeydown}
    />
  {:else}
    <OutputView
      {selectedOutputMode}
      {outputAutoScroll}
      {selectedRunId}
      {status}
      bind:outputSearchText
      {outputSearchMatches}
      {outputSearchCursor}
      {outputSearchActive}
      {outputHtml}
      {outputDisplayText}
      {outputIsAtBottom}
      bind:xtermRef
      {xtermRunId}
      {onToggleOutputAutoScroll}
      {onQueueStdin}
      {onRunOutputSearch}
      {onPrevOutputMatch}
      {onNextOutputMatch}
      {onClearOutputSearch}
      {onCopyOutput}
      {onJumpToLatest}
      {onResumeOutputAutoScroll}
      {onOpenInputModal}
      {onFocusOutputSearch}
      {onOutputScroll}
      {onXtermReady}
      {onXtermData}
      {onXtermResize}
      {onSearchKeydown}
    />
  {/if}
{/if}

<style>
  .detail-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 12px;
    flex-wrap: wrap;
  }

  .detail-title {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .detail-title-main {
    font-weight: 900;
    font-size: 15px;
    line-height: 1.2;
  }

  .detail-title-sub {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: center;
    font-size: 12px;
    color: var(--muted);
  }

  .detail-host code {
    font-weight: 900;
    color: var(--text-strong);
  }

  .detail-meta {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    align-items: center;
    font-size: 12px;
  }

  .meta-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    max-width: 100%;
    min-width: 0;
  }

  .meta-k {
    color: var(--muted);
    font-weight: 900;
  }

  .meta-v {
    color: var(--text-strong);
    font-weight: 900;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta-pill-cwd {
    flex: 1 1 360px;
  }

  .detail-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: center;
    justify-content: flex-end;
  }

  .dot {
    width: 10px;
    height: 10px;
    border-radius: 999px;
    background: rgba(100, 116, 139, 0.7);
    flex: 0 0 auto;
  }

  .dot[data-online="1"] {
    background: var(--success);
  }

  .offline-banner {
    margin-top: 10px;
    padding: 10px 12px;
    border-radius: var(--radius-lg);
    border: 1px solid rgba(239, 68, 68, 0.18);
    background: rgba(239, 68, 68, 0.06);
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    align-items: center;
    font-size: 12px;
    font-weight: 900;
    color: #991b1b;
  }

  .detail-tabs {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 12px;
    padding: 6px;
    border-radius: var(--radius-lg);
    background: rgba(148, 163, 184, 0.16);
    border: 1px solid rgba(148, 163, 184, 0.24);
  }

  .detail-tabs button {
    border: none;
    background: transparent;
    border-radius: 12px;
    padding: 10px 10px;
    font-weight: 800;
    font-size: 13px;
  }

  .detail-tabs button.active {
    background: linear-gradient(140deg, rgba(255, 255, 255, 0.95), rgba(224, 242, 254, 0.86));
    box-shadow: var(--shadow-sm);
    border: 1px solid rgba(14, 165, 233, 0.24);
  }

  .pinned-actions {
    display: flex;
    flex-direction: column;
    gap: 10px;
    margin: 10px 0 12px;
  }
</style>
