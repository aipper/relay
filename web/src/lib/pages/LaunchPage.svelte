<script lang="ts">
  import { relay } from "../stores/relay-store.svelte";
</script>

<div class="page-section">
  <h2 class="page-title">启动运行</h2>
  <p class="page-desc">在远程主机上启动一个 AI 编码会话</p>

  <div class="launch-form">
    <div class="field">
      <label class="field-label">主机</label>
      {#if relay.hosts.length > 0}
        <select class="select" bind:value={relay.startHostId}>
          {#each relay.hosts as h (h.id)}
            <option value={h.id}>{h.id}{h.online ? "（在线）" : "（离线）"}</option>
          {/each}
        </select>
      {:else}
        <input class="input" bind:value={relay.startHostId} placeholder="host-dev" />
      {/if}
    </div>

    <div class="field">
      <label class="field-label">工具</label>
      <select class="select" bind:value={relay.startTool}>
        {#each relay.currentStartToolOptions as option (option.value)}
          <option value={option.value}>{option.label}</option>
        {/each}
      </select>
      <div class="field-hint">
        {#if relay.currentStartHostToolsLoading}
          正在检测该主机已安装的工具…
        {:else if relay.currentStartToolStatuses && relay.currentStartToolOptions.length > 0}
          可用工具：{relay.currentStartToolOptions.map((o) => o.label).join(" / ")}
        {:else if relay.currentStartToolStatuses && relay.currentStartToolOptions.length === 0}
          当前主机未检测到可用工具
        {:else}
          将根据所选主机动态检测可用工具
        {/if}
      </div>
    </div>

    {#if relay.startTool === "opencode"}
      <div class="field">
        <label class="field-label">模型</label>
        <select
          class="select"
          bind:value={relay.startOpencodeModel}
          disabled={relay.currentStartHostToolsLoading || relay.currentStartOpencodeModels.length === 0}
        >
          {#if relay.currentStartOpencodeModels.length === 0}
            <option value="">使用主机默认模型</option>
          {:else}
            {#each relay.currentStartOpencodeModels as model (model)}
              <option value={model}>{model}</option>
            {/each}
          {/if}
        </select>
        <div class="field-hint">
          {#if relay.currentStartHostToolsLoading}
            正在读取模型配置…
          {:else if relay.currentStartOpencodeModelsError}
            模型配置失败：{relay.currentStartOpencodeModelsError}
          {:else if relay.currentStartOpencodeModels.length > 0}
            将覆盖 opencode 默认模型
            {#if relay.currentStartOpencodeDefaultModel}（默认：{relay.currentStartOpencodeDefaultModel}）{/if}
          {:else}
            未读取到显式模型列表，沿用主机默认
          {/if}
        </div>
      </div>

      <div class="field">
        <label class="field-label">Session ID（可选）</label>
        <input class="input" bind:value={relay.startOpencodeSessionId} placeholder="ses_xxx 或留空新建" />
        <div class="field-hint">续接已有的 opencode 会话</div>
      </div>
    {/if}

    <div class="field">
      <label class="field-label">CWD（可选）</label>
      <input class="input" bind:value={relay.startCwd} placeholder={relay.lastSuggestedStartCwd || "/path/to/project"} />
      <div class="field-hint">
        会记住每台主机最近使用的目录。
        {#if relay.lastSuggestedStartCwd}
          <button class="link-btn" onclick={() => relay.applySuggestedStartCwd(true)}>回填 {relay.lastSuggestedStartCwd}</button>
        {:else}
          请填远程主机上的真实目录
        {/if}
      </div>
    </div>

    <div class="field">
      <label class="field-label">命令（可选）</label>
      <input class="input" bind:value={relay.startCmd} placeholder={`留空使用 ${relay.startTool} 默认入口`} />
    </div>

    <button
      class="btn-primary"
      onclick={() => relay.startRun()}
      disabled={relay.status !== "connected" || relay.currentStartHostToolsLoading || (relay.currentStartToolStatuses !== null && relay.currentStartToolOptions.length === 0)}
    >
      启动
    </button>

    {#if relay.startError}
      <div class="error-banner">{relay.startError}</div>
    {/if}
  </div>
</div>

<style>
  .page-section {
    max-width: 480px;
    margin: 0 auto;
    width: 100%;
  }

  .page-title {
    font-family: "Hanken Grotesk", sans-serif;
    font-size: 22px;
    font-weight: 700;
    color: var(--text);
    margin: 0 0 4px 0;
  }

  .page-desc {
    font-size: 14px;
    color: var(--muted);
    margin: 0 0 24px 0;
  }

  .launch-form {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .field-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .field-hint {
    font-size: 12px;
    color: var(--muted);
    line-height: 1.4;
  }

  .input, .select {
    width: 100%;
    box-sizing: border-box;
    padding: 10px 12px;
    border-radius: 10px;
    border: 1px solid var(--border);
    background: var(--bg-input);
    color: var(--text);
    font-size: 14px;
    font-family: inherit;
    outline: none;
  }

  .input:focus, .select:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 20%, transparent);
  }

  .select {
    appearance: none;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%2394a3b8' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'%3E%3C/polyline%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 12px center;
    padding-right: 36px;
  }

  .link-btn {
    border: none;
    background: transparent;
    color: var(--accent);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    padding: 0;
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .btn-primary {
    width: 100%;
    padding: 12px;
    border: none;
    border-radius: 10px;
    background: var(--accent);
    color: white;
    font-size: 15px;
    font-weight: 600;
    cursor: pointer;
    transition: opacity 150ms ease;
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-primary:not(:disabled):hover {
    opacity: 0.9;
  }

  .error-banner {
    padding: 10px 12px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--danger) 12%, transparent);
    border: 1px solid color-mix(in srgb, var(--danger) 25%, transparent);
    color: var(--danger);
    font-size: 13px;
    line-height: 1.4;
  }
</style>
