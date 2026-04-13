<script lang="ts">
  export let hosts: any[] = [];
  export let hostDiagHostId: string = "";
  export let hostDiagError: string = "";
  export let hostInfo: string = "";
  export let hostDoctor: string = "";
  export let hostCapabilities: string = "";
  export let hostLogs: string = "";
  export let hostLogsLines: string = "";
  export let hostLogsMaxBytes: string = "";
  export let status: string = "";
  export let token: string = "";

  export let onRefreshHosts: () => void = () => {};
  export let onFetchHostInfo: () => void = () => {};
  export let onFetchHostDoctor: () => void = () => {};
  export let onFetchHostCapabilities: () => void = () => {};
  export let onFetchHostLogs: () => void = () => {};
</script>

<section>
  <h2>主机诊断（WS-RPC）</h2>
  <div style="display:flex;gap:8px;flex-wrap:wrap;margin-bottom:8px">
    <button on:click={onRefreshHosts} disabled={!token}>刷新主机</button>
  </div>
  <label>
    主机 ID
    {#if hosts.length > 0}
      <select bind:value={hostDiagHostId} style="width:100%;padding:10px;box-sizing:border-box">
        {#each hosts as h (h.id)}
          <option value={h.id}>{h.id}{h.online ? "（在线）" : "（离线）"}</option>
        {/each}
      </select>
    {:else}
      <input bind:value={hostDiagHostId} placeholder="host-dev" />
    {/if}
  </label>
  <div style="display:flex;gap:8px;flex-wrap:wrap;margin:8px 0">
    <button on:click={onFetchHostInfo} disabled={status !== "connected"}>host.info</button>
    <button on:click={onFetchHostDoctor} disabled={status !== "connected"}>host.doctor</button>
    <button on:click={onFetchHostCapabilities} disabled={status !== "connected"}>host.capabilities</button>
  </div>
  <div style="display:flex;gap:8px;flex-wrap:wrap;align-items:flex-end;margin:8px 0">
    <label style="flex:1;min-width:140px">
      lines
      <input bind:value={hostLogsLines} placeholder="200" />
    </label>
    <label style="flex:1;min-width:140px">
      max_bytes
      <input bind:value={hostLogsMaxBytes} placeholder="200000" />
    </label>
    <button on:click={onFetchHostLogs} disabled={status !== "connected"}>host.logs.tail</button>
  </div>
  {#if hostDiagError}
    <div style="color:#b91c1c">{hostDiagError}</div>
  {/if}
  {#if hostInfo}
    <h3>info</h3>
    <pre style="white-space:pre-wrap;word-break:break-word;max-height:220px;overflow:auto;border:1px solid #e5e7eb;padding:12px">{hostInfo}</pre>
  {/if}
  {#if hostDoctor}
    <h3>doctor</h3>
    <pre style="white-space:pre-wrap;word-break:break-word;max-height:220px;overflow:auto;border:1px solid #e5e7eb;padding:12px">{hostDoctor}</pre>
  {/if}
  {#if hostCapabilities}
    <h3>capabilities</h3>
    <pre style="white-space:pre-wrap;word-break:break-word;max-height:220px;overflow:auto;border:1px solid #e5e7eb;padding:12px">{hostCapabilities}</pre>
  {/if}
  {#if hostLogs}
    <h3>logs.tail</h3>
    <pre style="white-space:pre-wrap;word-break:break-word;max-height:240px;overflow:auto;border:1px solid #e5e7eb;padding:12px">{hostLogs}</pre>
  {/if}
</section>
