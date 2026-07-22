<script lang="ts">
  import { relay } from "../stores/relay-store.svelte";
  import SessionList from "../components/SessionList.svelte";
  import SessionDetail from "../components/SessionDetail.svelte";
  import TodoPanel from "../components/TodoPanel.svelte";
  import ToolsPanel from "../components/ToolsPanel.svelte";
  import HostDiagnostics from "../components/HostDiagnostics.svelte";

  let showTools = $state(false);
  let showHostDiag = $state(false);

  const isDetailOpen = $derived(Boolean(relay.selectedRunId));
  const mobileDetailOpen = $derived(Boolean(relay.isMobile && relay.selectedRunId));

  function handleSessionSelect(e: CustomEvent<string>) {
    relay.selectSession(e.detail);
  }

  function handleBack() {
    relay.selectedRunId = "";
  }

  function refreshAll() {
    Promise.all([relay.refreshHosts(), relay.refreshRuns()]);
  }
</script>

<div class="tools-bar">
  <button class="icon-btn" class:active={showTools} onclick={() => showTools = !showTools}>
    <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <path d="M14.7 6.3a1 1 0 000 1.4l1.6 1.6a1 1 0 001.4 0l3.77-3.77a6 6 0 01-7.94 7.94l-6.91 6.91a2.12 2.12 0 01-3-3l6.91-6.91a6 6 0 017.94-7.94l-3.76 3.76z"/>
    </svg>
    工具
  </button>
  <button class="icon-btn" class:active={showHostDiag} onclick={() => showHostDiag = !showHostDiag}>
    <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <rect x="2" y="3" width="20" height="14" rx="2" ry="2"/>
      <line x1="8" y1="21" x2="16" y2="21"/>
      <line x1="12" y1="17" x2="12" y2="21"/>
    </svg>
    主机诊断
  </button>
</div>

{#if showTools}
  <ToolsPanel
    bind:filePath={relay.filePath}
    fileContent={relay.fileContent}
    fileError={relay.fileError}
    bind:searchQuery={relay.searchQuery}
    searchMatches={relay.searchMatches}
    searchTruncated={relay.searchTruncated}
    searchError={relay.searchError}
    bind:gitDiffPath={relay.gitDiffPath}
    gitStatus={relay.gitStatus}
    gitDiff={relay.gitDiff}
    gitError={relay.gitError}
    selectedRunId={relay.selectedRunId}
    status={relay.status}
    onFetchFile={() => relay.fetchFile()}
    onSearchFiles={() => relay.searchFiles()}
    onFetchGitStatus={() => relay.fetchGitStatus()}
    onFetchGitDiff={() => relay.fetchGitDiff()}
  />
{/if}

{#if showHostDiag}
  <HostDiagnostics
    hosts={relay.hosts}
    bind:hostDiagHostId={relay.hostDiagHostId}
    hostDiagError={relay.hostDiagError}
    hostInfo={relay.hostInfo}
    hostDoctor={relay.hostDoctor}
    hostCapabilities={relay.hostCapabilities}
    hostLogs={relay.hostLogs}
    bind:hostLogsLines={relay.hostLogsLines}
    bind:hostLogsMaxBytes={relay.hostLogsMaxBytes}
    status={relay.status}
    token={relay.token}
    onRefreshHosts={() => relay.refreshHosts()}
    onFetchHostInfo={() => relay.fetchHostInfo()}
    onFetchHostDoctor={() => relay.fetchHostDoctor()}
    onFetchHostCapabilities={() => relay.fetchHostCapabilities()}
    onFetchHostLogs={() => relay.fetchHostLogs()}
  />
{/if}

<div
  class="sessions-layout"
  class:mobile-detail-open={mobileDetailOpen}
  class:detail-open={isDetailOpen}
>
  <div class="sessions-sidebar" class:hidden={mobileDetailOpen}>
    <SessionList
      token={relay.token}
      selectedRunId={relay.selectedRunId}
      sessionSearch={relay.sessionSearch}
      apiBaseUrl={relay.apiBaseUrl}
      runGroups={relay.runGroups}
      statusLabel={relay.statusLabel}
      sessionTitle={relay.sessionTitle}
      sessionSummary={relay.sessionSummary}
      formatRelativeTime={relay.formatRelativeTime}
      onSelectSession={(id) => relay.selectSession(id)}
      onRefresh={refreshAll}
      onSessionSelect={handleSessionSelect}
    />
  </div>

  <div class="sessions-main" class:hidden={!isDetailOpen && relay.isMobile}>
    {#if relay.selectedRunId}
      <SessionDetail
        selectedRun={relay.selectedRun}
        selectedRunId={relay.selectedRunId}
        isMobile={relay.isMobile}
        token={relay.token}
        status={relay.status}
        sessionDetailTab={relay.sessionDetailTab}
        statusLabel={relay.statusLabel}
        hostsById={relay.hostsById}
        sessionTitle={relay.sessionTitle}
        formatRelativeTime={relay.formatRelativeTime}
        selectedAwaiting={relay.selectedAwaiting}
        approvalForSession={relay.approvalForSession}
        bind:approvalAnswersJson={relay.approvalAnswersJson}
        awaitingIsApproval={relay.awaitingIsApproval}
        awaitingIsPrompt={relay.awaitingIsPrompt}
        awaitingWantsYesNo={relay.awaitingWantsYesNo}
        uiBlocks={relay.uiBlocks}
        renderMarkdownBasic={relay.renderMarkdownBasic}
        formatAbsTime={relay.formatAbsTime}
        copyText={relay.copyText}
        selectedOutput={relay.selectedOutput}
        tailLines={relay.tailLines}
        selectedRunReady={relay.selectedRunReady}
        outputAutoScroll={relay.outputAutoScroll}
        bind:outputSearchText={relay.outputSearchText}
        outputSearchMatches={relay.outputSearchMatches}
        outputSearchCursor={relay.outputSearchCursor}
        outputSearchActive={relay.outputSearchActive}
        outputHtml={relay.outputHtml}
        outputDisplayText={relay.outputDisplayText}
        outputIsAtBottom={relay.outputIsAtBottom}
        onBack={handleBack}
        onOpenApprovalModal={() => { relay.approvalModalShowArgs = false; relay.approvalModalOpen = true; }}
        onOpenStopConfirm={() => relay.stopConfirmOpen = true}
        onSendStop={(signal: string) => relay.sendStop(signal)}
        onSwitchToOutputTabAction={async () => {
          relay.sessionDetailTab = "output";
          if (relay.selectedRunId) {
            relay.subscribeToRun(relay.selectedRunId);
            await relay.loadMessages(relay.selectedRunId);
          }
        }}
        onSwitchToMessagesTab={() => {
          relay.sessionDetailTab = "messages";
          if (relay.selectedRunId) {
            relay.subscribeToRun(relay.selectedRunId);
            relay.loadMessages(relay.selectedRunId);
          }
        }}
        onRefreshMessages={() => relay.selectedRunId && relay.loadMessages(relay.selectedRunId)}
        onFocusOutputSearch={() => relay.focusOutputSearch()}
        onSendChatInput={(text: string) => relay.sendChatInput(text)}
        onOpenInputModal={(text: string) => relay.openInputModal(text)}
        onSendDecision={(d: string) => relay.sendDecision(d)}
        onToggleApprovalForSession={(v: boolean) => relay.approvalForSession = v}
        onToggleOutputAutoScroll={() => relay.toggleOutputAutoScroll()}
        onQueueStdin={(data: string) => relay.queueStdin(data)}
        onRunOutputSearch={() => relay.runOutputSearch()}
        onPrevOutputMatch={() => relay.prevOutputMatch()}
        onNextOutputMatch={() => relay.nextOutputMatch()}
        onClearOutputSearch={() => relay.clearOutputSearch()}
        onCopyOutput={() => relay.copyOutput()}
        onJumpToLatest={() => relay.jumpToLatest()}
        onResumeOutputAutoScroll={() => relay.resumeOutputAutoScroll()}
        onOutputScroll={() => relay.handleOutputScroll()}
        onSearchKeydown={(e: KeyboardEvent) => relay.handleOutputSearchKeydown(e)}
        onResumeFromStoredToken={() => relay.resumeFromStoredToken()}
        onRefreshSelectedSession={() => relay.refreshSelectedSession()}
      />
      <TodoPanel
        todos={relay.todos}
        bind:todoText={relay.todoText}
        todoSuggestions={relay.todoSuggestions}
        selectedRunId={relay.selectedRunId}
        onAddTodo={(text: string) => { relay.addTodo(text); relay.todoText = ""; }}
        onToggleTodo={(id: string) => relay.toggleTodo(id)}
        onRemoveTodo={(id: string) => relay.removeTodo(id)}
      />
    {:else if !relay.isMobile}
      <div class="empty-state">
        <svg viewBox="0 0 24 24" width="48" height="48" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <rect x="3" y="3" width="7" height="7" rx="1"/>
          <rect x="14" y="3" width="7" height="7" rx="1"/>
          <rect x="3" y="14" width="7" height="7" rx="1"/>
          <rect x="14" y="14" width="7" height="7" rx="1"/>
        </svg>
        <p>选择一个会话来查看详情</p>
      </div>
    {/if}
  </div>
</div>

<style>
  .tools-bar {
    display: flex;
    gap: 8px;
    padding: 8px 0;
    flex-wrap: wrap;
  }

  .icon-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 8px 14px;
    border-radius: 10px;
    border: 1px solid var(--border);
    background: var(--bg-surface);
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: border-color 150ms ease, color 150ms ease;
  }

  .icon-btn:hover, .icon-btn.active {
    border-color: var(--accent);
    color: var(--accent);
  }

  .sessions-layout {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr;
    gap: 12px;
    align-items: start;
  }

  @media (min-width: 768px) {
    .sessions-layout.detail-open {
      grid-template-columns: clamp(280px, 34vw, 340px) 1fr;
    }
  }

  .sessions-sidebar.hidden {
    display: none;
  }

  .sessions-main.hidden {
    display: none;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 16px;
    padding: 48px 16px;
    color: var(--muted);
  }

  .empty-state p {
    margin: 0;
    font-size: 14px;
  }
</style>
