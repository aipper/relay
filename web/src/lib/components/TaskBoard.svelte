<script lang="ts">
  import { relay } from "../stores/relay-store.svelte";

  let {
    selectedRunId = "",
    runGroups = [] as any[],
    token = "",
    apiBaseUrl = "",
    formatRelativeTime = ((ts: string | null) => "") as (ts: string | null) => string,
    statusLabel = ((r: any) => ({ kind: "", label: "" })) as (r: any) => { kind: string; label: string },
    sessionTitle = ((r: any) => "") as (r: any) => string,
    onSelectSession = ((id: string) => {}) as (id: string) => void,
  } = $props();

  function selectRun(runId: string) {
    onSelectSession(runId);
  }
</script>

<div class="taskboard">
  {#if runGroups.length === 0}
    <div class="empty-board">
      <p>暂无运行记录</p>
    </div>
  {:else}
    {#each runGroups as group}
      <div class="host-section">
        <div class="host-header" title={group.host_id}>
          <span class="host-name">{group.host_id}</span>
          <span class="host-count">{group.sessions.length}</span>
        </div>
        <div class="run-grid">
          {#each group.sessions as run}
            {@const st = statusLabel(run)}
            {@const title = sessionTitle(run)}
            <button
              class="run-card"
              class:selected={run.id === selectedRunId}
              data-kind={st.kind}
              onclick={() => selectRun(run.id)}
              type="button"
            >
              <div class="card-bar" data-kind={st.kind}></div>
              <div class="card-body">
                <div class="card-tool">{run.tool}</div>
                <div class="card-title" title={title}>{title || run.id}</div>
                <div class="card-meta">
                  {#if run.cwd}
                    <span class="card-cwd" title={run.cwd}>{run.cwd.split("/").filter(Boolean).pop() || run.cwd}</span>
                  {/if}
                  <span class="card-time">{formatRelativeTime(run.last_active_at ?? run.started_at)}</span>
                </div>
              </div>
            </button>
          {/each}
        </div>
      </div>
    {/each}
  {/if}
</div>

<style>
  .taskboard {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 4px;
  }

  .empty-board {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 32px 16px;
    color: var(--muted);
    font-size: 13px;
  }

  .host-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .host-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px;
    font-size: 13px;
    font-weight: 700;
    color: var(--text-secondary);
  }

  .host-count {
    font-family: "Geist Mono", monospace;
    font-size: 11px;
    font-weight: 600;
    color: var(--muted);
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0 8px;
    line-height: 1.6;
  }

  .run-grid {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .run-card {
    display: flex;
    gap: 0;
    border-radius: 10px;
    border: 1px solid var(--border);
    background: var(--bg-surface);
    overflow: hidden;
    cursor: pointer;
    text-align: left;
    padding: 0;
    transition: border-color 150ms ease, box-shadow 150ms ease;
  }

  .run-card:hover {
    border-color: color-mix(in srgb, var(--accent) 40%, transparent);
  }

  .run-card.selected {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px color-mix(in srgb, var(--accent) 50%, transparent);
  }

  .card-bar {
    width: 4px;
    flex: 0 0 auto;
  }

  .card-bar[data-kind="running"] { background: var(--accent); }
  .card-bar[data-kind="done"]    { background: var(--success); }
  .card-bar[data-kind="error"]   { background: var(--danger); }
  .card-bar[data-kind="warning"] { background: var(--warning); }

  .card-body {
    flex: 1;
    min-width: 0;
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .card-tool {
    font-family: "Geist Mono", monospace;
    font-size: 10px;
    font-weight: 600;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .card-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-strong);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    line-height: 1.3;
  }

  .card-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    color: var(--muted);
  }

  .card-cwd {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 120px;
  }

  .card-time {
    flex: 0 0 auto;
    margin-left: auto;
  }
</style>
