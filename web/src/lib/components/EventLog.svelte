<script lang="ts">
  export let events: any[] = [];
  export let token: string = "";
  export let view: string = "";
</script>

{#if token && view === "settings"}
  <section>
    <h2>事件</h2>
    <pre>{JSON.stringify(events[0] ?? null, null, 2)}</pre>
    <ul>
      {#each events as e (e.ts + e.type + (e.seq ?? 0))}
        <li>
          <code>{e.ts}</code> <strong>{e.type}</strong>
          {#if e.host_id}<code> host={e.host_id}</code>{/if}
          {#if e.run_id}<code> run={e.run_id}</code>{/if}
          {#if e.seq !== undefined}<code> seq={e.seq}</code>{/if}
        </li>
      {/each}
    </ul>
  </section>
{/if}
