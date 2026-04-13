<script lang="ts">
  import SessionSelector from "../SessionSelector.svelte";

  export let token: string = "";
  export let selectedRunId: string = "";
  export let sessionSearch: string = "";
  export let apiBaseUrl: string = "";

  export let runGroups: any[] = [];
  export let statusLabel: (r: any) => any = () => ({ kind: "", label: "" });
  export let sessionTitle: (r: any) => string = () => "";
  export let sessionSummary: (r: any) => string = () => "";
  export let formatRelativeTime: (ts: string | null) => string = () => "";

  export let onSelectSession: (id: string) => void = () => {};
  export let onRefresh: () => Promise<void> = async () => {};
  export let onSessionSelect: (e: CustomEvent<string>) => void = () => {};

  let showSessionSelector = false;
  let hostGroupCollapsed: Record<string, boolean> = {};

  function toggleHostGroup(hostId: string) {
    hostGroupCollapsed[hostId] = !hostGroupCollapsed[hostId];
    hostGroupCollapsed = hostGroupCollapsed;
  }

  $: hostGroups = runGroups;
</script>

<div class="list-head">
  <h2 style="margin:0">会话</h2>
  <button class="secondary" on:click={onRefresh} disabled={!token}>
    刷新
  </button>
  <button
    class="secondary"
    on:click={() => { showSessionSelector = !showSessionSelector; }}
    disabled={!token}
  >
    {showSessionSelector ? "关闭" : "切换"}
  </button>
</div>
<div class="list-search">
  <input bind:value={sessionSearch} placeholder="搜索" />
</div>

{#if showSessionSelector}
  <SessionSelector
    {token}
    baseUrl={apiBaseUrl}
    currentSessionId={selectedRunId}
    on:select={onSessionSelect}
  />
{/if}

{#each runGroups as g (g.host_id)}
  <div class="host-group">
    <button class="host-group-header" on:click={() => toggleHostGroup(g.host_id)} aria-expanded={!hostGroupCollapsed[g.host_id]}>
      <span class="chevron">{hostGroupCollapsed[g.host_id] ? "▸" : "▾"}</span>
      <span class="dot" data-online={g.online ? "1" : "0"} aria-hidden="true"></span>
      <span class="host-name">{g.display_name}</span>
      <span class="host-last-seen">{formatRelativeTime(g.last_seen_at)}</span>
    </button>
    {#if !hostGroupCollapsed[g.host_id]}
      <div class="session-items">
        {#each g.sessions as r (r.id)}
          {@const st = statusLabel(r)}
          {@const title = sessionTitle(r)}
          {@const summary = sessionSummary(r)}
          <button class="session-item" class:selected={selectedRunId === r.id} on:click={() => onSelectSession(r.id)}>
            <div class="session-item-top">
              {#if title}
                <div class="session-title">{title}</div>
              {/if}
              <span class="session-status" data-kind={st.kind}>{st.label}</span>
            </div>
            {#if r.status === "awaiting_approval"}
              <div class="session-meta">
                <span class="session-tool">{r.tool}</span>
                {#if r.pending_op_tool}<span class="session-op">{r.pending_op_tool}</span>{/if}
                {#if r.pending_op_args_summary}<span class="session-op-args">{r.pending_op_args_summary}</span>{/if}
              </div>
            {:else}
              {#if summary}
                <div class="session-summary">{summary}</div>
              {/if}
            {/if}
            <div class="session-time">{formatRelativeTime(r.last_active_at ?? r.started_at)}</div>
          </button>
        {/each}
      </div>
    {/if}
  </div>
{/each}

<style>
  .list-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
  }

  .list-search {
    margin: 10px 0 12px 0;
  }

  .list-search input {
    border-radius: 999px;
    padding-left: 14px;
    padding-right: 14px;
  }

  .host-group {
    margin: 10px 0;
  }

  .host-group-header {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: linear-gradient(150deg, rgba(255, 255, 255, 0.86), rgba(239, 246, 255, 0.88));
    text-align: left;
    box-shadow: var(--shadow-sm);
  }

  .host-group-header:hover {
    border-color: var(--border-strong);
    background: linear-gradient(150deg, rgba(255, 255, 255, 0.95), rgba(224, 242, 254, 0.9));
  }

  .chevron {
    width: 16px;
    color: var(--muted);
    flex: 0 0 auto;
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

  .host-name {
    font-weight: 800;
    font-size: 13px;
    flex: 1;
  }

  .host-last-seen {
    font-size: 12px;
    color: var(--muted);
    flex: 0 0 auto;
  }

  .session-items {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 8px;
  }

  .session-item {
    width: 100%;
    text-align: left;
    position: relative;
    padding: 10px 12px;
    border-radius: var(--radius-lg);
    border: 1px solid var(--border);
    background: linear-gradient(150deg, rgba(255, 255, 255, 0.94), rgba(240, 249, 255, 0.88));
    display: flex;
    flex-direction: column;
    gap: 6px;
    box-shadow: var(--shadow-sm);
    transition:
      border-color 160ms ease,
      background-color 160ms ease,
      box-shadow 160ms ease;
  }

  .session-item:hover {
    border-color: var(--border-strong);
    box-shadow: 0 10px 20px rgba(2, 6, 23, 0.1);
  }

  .session-item.selected {
    border-color: rgba(14, 165, 233, 0.45);
    background: linear-gradient(150deg, rgba(224, 242, 254, 0.9), rgba(186, 230, 253, 0.45));
  }

  .session-item.selected::before {
    content: "";
    position: absolute;
    left: 0;
    top: 10px;
    bottom: 10px;
    width: 3px;
    border-radius: 999px;
    background: linear-gradient(180deg, #0ea5e9, #22d3ee);
  }

  .session-item-top {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 10px;
  }

  .session-title {
    font-weight: 900;
    font-size: 13px;
    line-height: 1.2;
    color: var(--text-strong);
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .session-meta {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    align-items: center;
    font-size: 12px;
    color: #355271;
  }

  .session-tool {
    font-weight: 900;
    color: var(--text-strong);
  }

  .session-op,
  .session-op-args {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
    font-size: 12px;
    background: rgba(255, 255, 255, 0.9);
    border: 1px solid rgba(14, 165, 233, 0.2);
    padding: 2px 6px;
    border-radius: 8px;
  }

  .session-summary {
    font-size: 12px;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  @media (max-width: 1024px) {
    .session-summary {
      display: none;
    }
  }

  .session-time {
    font-size: 12px;
    color: var(--muted);
  }
</style>
