<script lang="ts">
  export let hosts: any[] = [];
  export let startHostId: string = "";
  export let startTool: string = "";
  export let currentStartToolOptions: ReadonlyArray<{ readonly value: string; readonly label: string }> = [];
  export let currentStartHostToolsLoading: boolean = false;
  export let currentStartToolStatuses: any = null;
  export let startOpencodeModel: string = "";
  export let currentStartOpencodeModels: string[] = [];
  export let currentStartOpencodeModelsError: string = "";
  export let currentStartOpencodeDefaultModel: string = "";
  export let currentStartOpencodeModelsNote: string = "";
  export let startOpencodeSessionId: string = "";
  export let startCwd: string = "";
  export let lastSuggestedStartCwd: string = "";
  export let startCmd: string = "";
  export let startError: string = "";
  export let status: string = "";
  export let token: string = "";

  export let onRefreshHosts: () => void = () => {};
  export let onStartRun: () => void = () => {};
  export let onApplySuggestedStartCwd: () => void = () => {};
</script>

<section>
  <h2>启动运行（远程）</h2>
  <button on:click={onRefreshHosts} disabled={!token} style="margin-bottom:8px">刷新主机</button>
  <label>
    主机 ID
    {#if hosts.length > 0}
      <select bind:value={startHostId} style="width:100%;padding:10px;box-sizing:border-box">
        {#each hosts as h (h.id)}
          <option value={h.id}>{h.id}{h.online ? "（在线）" : "（离线）"}</option>
        {/each}
      </select>
    {:else}
      <input bind:value={startHostId} placeholder="host-dev" />
    {/if}
  </label>
  <label>
    工具
    <select bind:value={startTool} style="width:100%;padding:10px;box-sizing:border-box">
      {#each currentStartToolOptions as option (option.value)}
        <option value={option.value}>{option.label}</option>
      {/each}
    </select>
  </label>
  <div style="font-size:12px;color:#475569;margin:-4px 0 10px 0">
    {#if currentStartHostToolsLoading}
      正在检测该主机已安装的工具…
    {:else if currentStartToolStatuses && currentStartToolOptions.length > 0}
      当前主机可用工具：{currentStartToolOptions.map((option) => option.label).join(" / ")}
    {:else if currentStartToolStatuses && currentStartToolOptions.length === 0}
      当前主机未检测到可用的受支持工具（opencode）。
    {:else}
      将根据所选 host 动态检测可用工具。
    {/if}
  </div>
  {#if startTool === "opencode"}
    <label>
      模型
      <select bind:value={startOpencodeModel} style="width:100%;padding:10px;box-sizing:border-box" disabled={currentStartHostToolsLoading || currentStartOpencodeModels.length === 0}>
        {#if currentStartOpencodeModels.length === 0}
          <option value="">使用 host 默认模型</option>
        {:else}
          {#each currentStartOpencodeModels as model (model)}
            <option value={model}>{model}</option>
          {/each}
        {/if}
      </select>
    </label>
    <div style="font-size:12px;color:#475569;margin:-4px 0 10px 0">
      {#if currentStartHostToolsLoading}
        正在读取该主机的 opencode 模型配置…
      {:else if currentStartOpencodeModelsError}
        当前主机的 opencode 模型配置读取失败：{currentStartOpencodeModelsError}
      {:else if currentStartOpencodeModels.length > 0}
        将以本次启动覆盖 opencode 模型。{#if currentStartOpencodeDefaultModel}当前默认：<code>{currentStartOpencodeDefaultModel}</code>。{/if}
      {:else}
        未从该主机读取到显式模型列表，将沿用 opencode 主机默认模型。
      {/if}
      {#if !currentStartHostToolsLoading && currentStartOpencodeModelsNote}
        <br />{currentStartOpencodeModelsNote}
      {/if}
    </div>
    <label>
      Session ID（可选，续接已有会话）
      <input bind:value={startOpencodeSessionId} placeholder="ses_xxx 或留空新建" />
    </label>
  {/if}
  <label>
    CWD（可选，主机路径）
    <input bind:value={startCwd} placeholder={lastSuggestedStartCwd || "/path/to/project"} />
  </label>
  <div style="font-size:12px;color:#475569;margin:-4px 0 10px 0">
    会记住每台主机最近一次成功启动的目录。
    {#if lastSuggestedStartCwd}
      最近可用路径：<code>{lastSuggestedStartCwd}</code>
      <button type="button" class="secondary" style="margin-left:8px;padding:2px 8px" on:click={onApplySuggestedStartCwd}>回填</button>
    {:else}
      请填写远程主机上的真实目录，例如 <code>/home/ab/test</code>。
    {/if}
  </div>
  <label>
    命令
    <input bind:value={startCmd} placeholder={`（留空：运行工具默认入口，例如 ${startTool}）`} />
  </label>
  <button on:click={onStartRun} disabled={status !== "connected" || currentStartHostToolsLoading || (currentStartToolStatuses !== null && currentStartToolOptions.length === 0)}>
    启动
  </button>
  {#if startError}
    <div style="color:#b91c1c">{startError}</div>
  {/if}
</section>
