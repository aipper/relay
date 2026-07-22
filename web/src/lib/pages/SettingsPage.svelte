<script lang="ts">
  import { relay } from "../stores/relay-store.svelte";
  import EventLog from "../components/EventLog.svelte";

  let localPassword = $state(relay.password);
  let localCustomUrl = $state(relay.customBaseUrl);
  let localUseCustom = $state(relay.useCustomServer);
</script>

<div class="page-section">
  <h2 class="page-title">设置</h2>

  <div class="settings-card">
    <div class="card-header">服务器</div>

    <div class="field-row">
      <div class="field">
        <span class="field-label">当前服务</span>
        <div class="field-value">
          <code class="code-inline">{relay.apiBaseUrl}</code>
          {#if relay.health}
            <span>{relay.health.name} {relay.health.version}</span>
          {/if}
        </div>
      </div>
      <button class="btn-secondary" onclick={() => relay.disconnect()} disabled={relay.status !== "connected" && !relay.token}>
        断开
      </button>
    </div>

    <div class="field">
      <div class="toggle-row">
        <span class="field-label">自定义 Server URL</span>
        <label class="toggle">
          <input type="checkbox" bind:checked={localUseCustom} onchange={() => { relay.useCustomServer = localUseCustom; relay.persistServerPrefs(); }} />
          <span class="toggle-track"></span>
        </label>
      </div>
      {#if localUseCustom}
        <input class="input" bind:value={localCustomUrl} placeholder="http(s)://host:8787" onchange={() => { relay.customBaseUrl = localCustomUrl; relay.persistServerPrefs(); }} />
      {/if}
    </div>

    {#if relay.isProbablyInsecureUrl}
      <div class="warning-banner">
        HTTP 连接：密码与 token 在传输层不加密。建议通过 HTTPS 访问。
      </div>
    {/if}
  </div>

  <div class="settings-card">
    <div class="card-header">账号</div>
    <div class="field-value">
      当前用户：<code class="code-inline">{relay.username}</code>
    </div>

    <div class="field">
      <div class="toggle-row">
        <span class="field-label">刷新后保持登录</span>
        <label class="toggle">
          <input type="checkbox" bind:checked={relay.keepSignedIn} onchange={() => relay.persistAuthPrefs()} />
          <span class="toggle-track"></span>
        </label>
      </div>
    </div>

    <div class="field">
      <div class="toggle-row">
        <span class="field-label">记住密码（本机）</span>
        <label class="toggle">
          <input type="checkbox" bind:checked={relay.rememberPassword} onchange={() => { if (!relay.rememberPassword) localPassword = ""; relay.persistAuthPrefs(); }} />
          <span class="toggle-track"></span>
        </label>
      </div>
      {#if relay.rememberPassword}
        <input class="input" type="password" bind:value={localPassword} onchange={() => { relay.password = localPassword; relay.persistAuthPrefs(); }} />
      {/if}
    </div>
  </div>

  <div class="settings-card">
    <div class="card-header">Server 日志</div>
    <div class="field-row">
      <div class="field">
        <label class="field-label" for="log-lines">行数</label>
        <input id="log-lines" class="input" bind:value={relay.serverLogsLines} placeholder="200" />
      </div>
      <div class="field">
        <label class="field-label" for="log-maxbytes">最大字节</label>
        <input id="log-maxbytes" class="input" bind:value={relay.serverLogsMaxBytes} placeholder="200000" />
      </div>
    </div>
    <button class="btn-secondary" onclick={() => relay.fetchServerLogs()} disabled={relay.status !== "connected"}>
      读取日志
    </button>
    {#if relay.serverLogsError}
      <div class="error-banner">{relay.serverLogsError}</div>
    {/if}
    {#if relay.serverLogsPath}
      <div class="log-meta">
        <code class="code-inline">{relay.serverLogsPath}</code>
        {#if relay.serverLogsTruncated}
          <span class="muted">truncated</span>
        {/if}
      </div>
      <pre class="log-output">{relay.serverLogs}</pre>
    {/if}
  </div>

  <div class="settings-card">
    <div class="card-header">事件日志</div>
    <EventLog events={relay.events} token={relay.token} view={relay.view} />
  </div>
</div>

<style>
  .page-section {
    max-width: 520px;
    margin: 0 auto;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding-bottom: 24px;
  }

  .page-title {
    font-family: "Hanken Grotesk", sans-serif;
    font-size: 22px;
    font-weight: 700;
    color: var(--text);
    margin: 0;
  }

  .settings-card {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 20px;
    border-radius: 16px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
  }

  .card-header {
    font-weight: 600;
    font-size: 15px;
    color: var(--text);
    padding-bottom: 4px;
    border-bottom: 1px solid var(--border);
  }

  .field-row {
    display: flex;
    gap: 12px;
    align-items: flex-end;
    flex-wrap: wrap;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
  }

  .field-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .field-value {
    font-size: 14px;
    color: var(--text-secondary);
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }

  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .toggle {
    position: relative;
    display: inline-flex;
    width: 40px;
    height: 22px;
    flex-shrink: 0;
  }

  .toggle input {
    position: absolute;
    opacity: 0;
    width: 0;
    height: 0;
  }

  .toggle-track {
    width: 100%;
    height: 100%;
    border-radius: 999px;
    background: var(--border);
    cursor: pointer;
    transition: background 150ms ease;
  }

  .toggle input:checked + .toggle-track {
    background: var(--accent);
  }

  .toggle-track::after {
    content: "";
    position: absolute;
    top: 2px;
    left: 2px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: white;
    transition: transform 150ms ease;
  }

  .toggle input:checked + .toggle-track::after {
    transform: translateX(18px);
  }

  .input {
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

  .input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 20%, transparent);
  }

  .code-inline {
    font-family: "Geist Mono", "Fira Code", monospace;
    font-size: 13px;
    color: var(--text-secondary);
    padding: 2px 6px;
    border-radius: 6px;
    background: var(--bg-canvas);
  }

  .btn-secondary {
    padding: 8px 16px;
    border-radius: 10px;
    border: 1px solid var(--border);
    background: var(--bg-surface);
    color: var(--text-secondary);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: border-color 150ms ease;
  }

  .btn-secondary:hover {
    border-color: var(--accent);
  }

  .btn-secondary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .warning-banner {
    padding: 10px 12px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--warning) 12%, transparent);
    border: 1px solid color-mix(in srgb, var(--warning) 25%, transparent);
    color: var(--warning);
    font-size: 13px;
    line-height: 1.4;
  }

  .error-banner {
    padding: 10px 12px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--danger) 12%, transparent);
    border: 1px solid color-mix(in srgb, var(--danger) 25%, transparent);
    color: var(--danger);
    font-size: 13px;
  }

  .log-meta {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }

  .log-output {
    font-family: "Geist Mono", "Fira Code", monospace;
    font-size: 12px;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 240px;
    overflow: auto;
    padding: 12px;
    border-radius: 10px;
    border: 1px solid var(--border);
    background: var(--bg-canvas);
    color: var(--text-secondary);
    margin: 0;
  }

  .muted {
    color: var(--muted);
    font-size: 12px;
  }
</style>
