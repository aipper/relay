<script lang="ts">
  export let filePath: string = "";
  export let fileContent: string = "";
  export let fileError: string = "";
  export let searchQuery: string = "";
  export let searchMatches: any[] = [];
  export let searchTruncated: boolean = false;
  export let searchError: string = "";
  export let gitDiffPath: string = "";
  export let gitStatus: string = "";
  export let gitDiff: string = "";
  export let gitError: string = "";
  export let selectedRunId: string = "";
  export let status: string = "";

  export let onFetchFile: () => void = () => {};
  export let onSearchFiles: () => void = () => {};
  export let onFetchGitStatus: () => void = () => {};
  export let onFetchGitDiff: () => void = () => {};
</script>

<section>
  <h2>文件（run cwd）</h2>
  <label>
    路径（相对）
    <input bind:value={filePath} placeholder="README.md" />
  </label>
  <button on:click={onFetchFile} disabled={!selectedRunId || status !== "connected"}>读取</button>
  {#if fileError}
    <div style="color:#b91c1c">{fileError}</div>
  {/if}
  <pre style="white-space:pre-wrap;word-break:break-word;max-height:240px;overflow:auto;border:1px solid var(--border);padding:12px">{fileContent || "(empty)"}</pre>

  <h2>搜索（run cwd）</h2>
  <label>
    Query
    <input bind:value={searchQuery} placeholder="TODO" />
  </label>
  <button on:click={onSearchFiles} disabled={!selectedRunId || status !== "connected"}>Search</button>
  {#if searchError}
    <div style="color:#b91c1c">{searchError}</div>
  {/if}
  {#if searchMatches.length === 0}
    <div>(no matches)</div>
  {:else}
    {#if searchTruncated}
      <div style="color:#92400e">结果已截断</div>
    {/if}
    <ul>
      {#each searchMatches as m (m.path + ":" + m.line + ":" + m.column)}
        <li>
          <code>{m.path}:{m.line}:{m.column}</code> {m.text}
        </li>
      {/each}
    </ul>
  {/if}

  <h2>Git（run cwd）</h2>
  <div style="display:flex;gap:8px;flex-wrap:wrap">
    <button on:click={onFetchGitStatus} disabled={!selectedRunId || status !== "connected"}>状态</button>
    <button on:click={onFetchGitDiff} disabled={!selectedRunId || status !== "connected"}>差异</button>
  </div>
  <label>
    Diff 路径（可选，相对）
    <input bind:value={gitDiffPath} placeholder="src/main.rs" />
  </label>
  {#if gitError}
    <div style="color:#b91c1c">{gitError}</div>
  {/if}
  <h3>status</h3>
  <pre style="white-space:pre-wrap;word-break:break-word;max-height:160px;overflow:auto;border:1px solid var(--border);padding:12px">
{gitStatus || "(empty)"}</pre>
  <h3>diff</h3>
  <pre style="white-space:pre-wrap;word-break:break-word;max-height:240px;overflow:auto;border:1px solid var(--border);padding:12px">{gitDiff || "(empty)"}</pre>
</section>
