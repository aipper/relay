<script lang="ts">
  import { createEventDispatcher, onMount } from "svelte";

  export let token: string = "";
  export let currentSessionId: string = "";
  export let baseUrl: string = "";

  interface Session {
    id: string;
    host_id: string;
    tool: string;
    opencode_session_id?: string | null;
    cwd: string;
    status: string;
    started_at: string;
    last_active_at?: string | null;
  }

  const dispatch = createEventDispatcher<{ select: string }>();

  let sessions: Session[] = [];
  let loading = false;
  let error = "";

  onMount(() => {
    fetchSessions();
  });

  async function fetchSessions() {
    if (!token || !baseUrl) {
      error = "Missing auth token or base URL";
      return;
    }
    loading = true;
    error = "";
    try {
      const resp = await fetch(`${baseUrl}/sessions?limit=50`, {
        headers: { Authorization: `Bearer ${token}` },
      });
      if (!resp.ok) {
        throw new Error(`HTTP ${resp.status}`);
      }
      sessions = await resp.json();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  function selectSession(session: Session) {
    dispatch("select", session.id);
  }

  function formatTime(iso: string): string {
    const d = new Date(iso);
    const now = new Date();
    const diff = now.getTime() - d.getTime();
    if (diff < 60_000) return "刚刚";
    if (diff < 3600_000) return `${Math.floor(diff / 60_000)} 分钟前`;
    if (diff < 86400_000) return `${Math.floor(diff / 3600_000)} 小时前`;
    return d.toLocaleDateString("zh-CN");
  }

  function statusLabel(status: string): string {
    const labels: Record<string, string> = {
      running: "运行中",
      exited: "已退出",
      awaiting_approval: "待批准",
    };
    return labels[status] ?? status;
  }
</script>

<div class="session-selector">
  <div class="header">
    <h3>选择会话</h3>
    <button on:click={fetchSessions} disabled={loading} class="refresh-btn">
      {loading ? "加载中…" : "刷新"}
    </button>
  </div>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if sessions.length === 0 && !loading && !error}
    <div class="empty">暂无会话</div>
  {:else}
    <ul class="session-list">
      {#each sessions as session (session.id)}
        <li>
          <button
            class="session-item"
            class:selected={session.id === currentSessionId}
            on:click={() => selectSession(session)}
          >
            <div class="session-main">
              <span class="session-id">{session.id}</span>
              <span class="session-status" class:running={session.status === "running"}>
                {statusLabel(session.status)}
              </span>
            </div>
            <div class="session-meta">
              <span class="host">{session.host_id}</span>
              <span class="tool">{session.tool}</span>
              {#if session.opencode_session_id}
                <span class="opencode-session" title="OpenCode Session">
                  🎭 {session.opencode_session_id}
                </span>
              {/if}
              <span class="time">{formatTime(session.started_at)}</span>
            </div>
            {#if session.cwd}
              <div class="cwd" title="工作目录">{session.cwd}</div>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .session-selector {
    background: #fff;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    padding: 12px;
    max-height: 400px;
    overflow-y: auto;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
  }

  .header h3 {
    margin: 0;
    font-size: 14px;
    color: #1e293b;
  }

  .refresh-btn {
    padding: 4px 12px;
    font-size: 12px;
    background: #f1f5f9;
    border: 1px solid #cbd5e1;
    border-radius: 4px;
    cursor: pointer;
  }

  .refresh-btn:hover {
    background: #e2e8f0;
  }

  .error {
    color: #dc2626;
    font-size: 12px;
    padding: 8px;
    background: #fef2f2;
    border-radius: 4px;
  }

  .empty {
    color: #64748b;
    font-size: 12px;
    text-align: center;
    padding: 20px;
  }

  .session-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .session-list li {
    margin-bottom: 4px;
  }

  .session-item {
    width: 100%;
    text-align: left;
    padding: 8px 12px;
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
  }

  .session-item:hover {
    background: #f1f5f9;
    border-color: #94a3b8;
  }

  .session-item.selected {
    background: #eff6ff;
    border-color: #3b82f6;
  }

  .session-main {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 4px;
  }

  .session-id {
    font-family: monospace;
    font-size: 13px;
    color: #1e293b;
  }

  .session-status {
    font-size: 11px;
    padding: 2px 6px;
    border-radius: 10px;
    background: #e2e8f0;
    color: #64748b;
  }

  .session-status.running {
    background: #dcfce7;
    color: #16a34a;
  }

  .session-meta {
    display: flex;
    gap: 8px;
    font-size: 11px;
    color: #64748b;
  }

  .opencode-session {
    color: #7c3aed;
  }

  .cwd {
    font-size: 10px;
    color: #94a3b8;
    margin-top: 4px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>