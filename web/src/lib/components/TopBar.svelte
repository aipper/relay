<script lang="ts">
  export let health: any = null;
  export let apiBaseUrl: string = "";
  export let uiVersion: string | number = "";
  export let status: string = "";
  export let connLabel: (s: string) => string = () => "";
  export let token: string = "";
  export let username: string = "";
</script>

<header class="topbar">
  <div>
    <h1 style="margin: 0">Relay</h1>
    <div class="subtle subtle-row">
      {#if health}
        <span>{health.name} {health.version}</span>
      {:else}
        <span>{apiBaseUrl}</span>
      {/if}
      <span class="version-pill">ui v{uiVersion}</span>
    </div>
  </div>
  <div style="display:flex;gap:8px;align-items:center;flex-wrap:wrap;justify-content:flex-end">
    <span class="conn-status" data-kind={status}>
      <span class="conn-dot" aria-hidden="true"></span>
      <span>{connLabel(status)}</span>
    </span>
    {#if token}
      <span class="subtle"><code>{username}</code></span>
    {/if}
  </div>
</header>

<style>
  .topbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
    padding: 12px 14px;
    border: 1px solid rgba(14, 165, 233, 0.24);
    border-radius: 22px;
    background:
      linear-gradient(135deg, rgba(255, 255, 255, 0.9) 0%, rgba(240, 249, 255, 0.86) 100%);
    box-shadow: var(--shadow-sm);
    backdrop-filter: blur(10px);
    position: sticky;
    top: 8px;
    z-index: 20;
  }

  .version-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    font-weight: 900;
    color: #0c4a6e;
    padding: 3px 8px;
    border-radius: 999px;
    border: 1px solid rgba(14, 165, 233, 0.28);
    background: rgba(186, 230, 253, 0.45);
  }

  .conn-status {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    border-radius: 999px;
    padding: 6px 10px;
    font-size: 12px;
    border: 1px solid var(--border);
    background: var(--surface-2);
  }

  .conn-dot {
    width: 8px;
    height: 8px;
    border-radius: 999px;
    background: rgba(100, 116, 139, 0.8);
  }

  .conn-status[data-kind="connected"] {
    background: rgba(34, 197, 94, 0.12);
    border-color: rgba(34, 197, 94, 0.28);
    color: #065f46;
  }

  .conn-status[data-kind="connected"] .conn-dot {
    background: rgba(34, 197, 94, 0.95);
  }

  .conn-status[data-kind="checking"],
  .conn-status[data-kind="connecting"] {
    background: rgba(37, 99, 235, 0.12);
    border-color: rgba(37, 99, 235, 0.28);
    color: #1d4ed8;
  }

  .conn-status[data-kind="checking"] .conn-dot,
  .conn-status[data-kind="connecting"] .conn-dot {
    background: rgba(37, 99, 235, 0.95);
  }

  .conn-status[data-kind="error"] {
    background: rgba(239, 68, 68, 0.1);
    border-color: rgba(239, 68, 68, 0.22);
    color: #991b1b;
  }

  .conn-status[data-kind="error"] .conn-dot {
    background: rgba(239, 68, 68, 0.9);
  }
</style>
