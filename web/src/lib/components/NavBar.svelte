<script lang="ts">
  export let view: string = "sessions";
  export let token: string = "";

  export let onSetView: (v: string) => void = () => {};
</script>

{#if token}
  <nav class="segmented" aria-label="navigation">
    <button class:active={view === "sessions"} on:click={() => onSetView("sessions")}>会话</button>
    <button class:active={view === "hosts"} on:click={() => onSetView("hosts")}>主机</button>
    <button class:active={view === "start"} on:click={() => onSetView("start")}>启动</button>
    <button class:active={view === "tools"} on:click={() => onSetView("tools")}>工具</button>
    <button class:active={view === "settings"} on:click={() => onSetView("settings")}>设置</button>
  </nav>
{/if}

<style>
  .segmented {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: 6px;
    background: rgba(148, 163, 184, 0.16);
    border: 1px solid rgba(148, 163, 184, 0.24);
    border-radius: 20px;
    padding: 6px;
  }

  .segmented button {
    border: none;
    background: transparent;
    border-radius: var(--radius-md);
    padding: 10px 8px;
    font-weight: 800;
    font-size: 13px;
    color: #334155;
    transition:
      background-color 180ms ease,
      color 180ms ease,
      box-shadow 180ms ease;
  }

  .segmented button.active {
    background: linear-gradient(140deg, rgba(255, 255, 255, 0.92), rgba(224, 242, 254, 0.9));
    box-shadow:
      0 1px 2px rgba(2, 6, 23, 0.08),
      0 6px 18px rgba(14, 165, 233, 0.16);
    border: 1px solid rgba(14, 165, 233, 0.24);
    color: #0c4a6e;
  }
</style>
