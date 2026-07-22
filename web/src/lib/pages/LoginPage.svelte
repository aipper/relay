<script lang="ts">
  import { relay } from "../stores/relay-store.svelte";

  let customUrl = $state("");
  let showCustomUrl = $state(false);
  let loginUsername = $state(relay.username);
  let loginPassword = $state("");

  function handleConnect() {
    if (showCustomUrl && customUrl.trim()) {
      relay.setCustomBaseUrl(customUrl.trim());
    }
    relay.setCredentials(loginUsername, loginPassword);
    relay.connect();
  }
</script>

<div class="login-page">
  <div class="login-brand">
    <svg class="brand-icon" viewBox="0 0 40 40" fill="none" aria-hidden="true">
      <rect x="2" y="2" width="36" height="36" rx="10" stroke="currentColor" stroke-width="2"/>
      <path d="M14 20l4 4 8-8" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>
    <h1 class="brand-name">Relay</h1>
    <p class="brand-desc">远程 AI 编码工作台</p>
  </div>

  <div class="login-card">
    <div class="card-header">
      <span class="card-title">连接到服务器</span>
    </div>

    <div class="field">
      <div class="field-row">
        <span class="field-label">Server URL</span>
        <button class="link-btn" onclick={() => showCustomUrl = !showCustomUrl}>
          {showCustomUrl ? "使用默认" : "自定义"}
        </button>
      </div>
      {#if showCustomUrl}
        <input
          class="input"
          bind:value={customUrl}
          placeholder="http(s)://host:8787"
          type="url"
        />
      {:else}
        <div class="field-hint">
          <code class="code-inline">{relay.defaultApiBaseUrl}</code>
        </div>
      {/if}
    </div>

    <div class="field">
      <label class="field-label" for="username-input">用户名</label>
      <input
        id="username-input"
        class="input"
        bind:value={loginUsername}
        placeholder="admin"
        autocomplete="username"
      />
    </div>

    <div class="field">
      <label class="field-label" for="password-input">密码</label>
      <input
        id="password-input"
        class="input"
        type="password"
        bind:value={loginPassword}
        placeholder="••••••••"
        autocomplete="current-password"
        onkeydown={(e) => { if (e.key === "Enter") handleConnect(); }}
      />
    </div>

    <button
      class="btn-primary"
      onclick={handleConnect}
      disabled={relay.loginBusy || !loginUsername.trim() || !loginPassword}
    >
      {relay.loginBusy ? "连接中…" : "登录"}
    </button>

    {#if relay.lastError}
      <div class="error-banner">{relay.lastError}</div>
    {/if}

    {#if relay.isProbablyInsecureUrl}
      <div class="warning-banner">
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
          <path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/>
          <line x1="12" y1="9" x2="12" y2="13"/>
          <line x1="12" y1="17" x2="12.01" y2="17"/>
        </svg>
        <span>连接使用 HTTP，密码会明文传输。建议通过 HTTPS 访问。</span>
      </div>
    {/if}
  </div>
</div>

<style>
  .login-page {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 32px;
    padding: 32px 16px;
  }

  .login-brand {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    text-align: center;
  }

  .brand-icon {
    width: 48px;
    height: 48px;
    color: var(--accent);
  }

  .brand-name {
    font-family: "Hanken Grotesk", sans-serif;
    font-size: 28px;
    font-weight: 700;
    color: var(--text);
    margin: 0;
    letter-spacing: -0.03em;
  }

  .brand-desc {
    font-size: 14px;
    color: var(--muted);
    margin: 0;
  }

  .login-card {
    width: 100%;
    max-width: 360px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 24px;
    border-radius: 16px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
  }

  .card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .card-title {
    font-weight: 600;
    font-size: 15px;
    color: var(--text);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .field-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .field-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .field-hint {
    font-size: 13px;
    color: var(--muted);
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
    transition: border-color 150ms ease;
  }

  .input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 20%, transparent);
  }

  .input::placeholder {
    color: var(--muted);
  }

  .code-inline {
    font-family: "Geist Mono", "Fira Code", monospace;
    font-size: 13px;
    color: var(--text-secondary);
    padding: 2px 6px;
    border-radius: 6px;
    background: var(--bg-canvas);
  }

  .link-btn {
    border: none;
    background: transparent;
    color: var(--accent);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 6px;
  }

  .link-btn:hover {
    background: color-mix(in srgb, var(--accent) 10%, transparent);
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

  .warning-banner {
    display: flex;
    gap: 8px;
    padding: 10px 12px;
    border-radius: 10px;
    background: color-mix(in srgb, var(--warning) 12%, transparent);
    border: 1px solid color-mix(in srgb, var(--warning) 25%, transparent);
    color: var(--warning);
    font-size: 13px;
    line-height: 1.4;
    align-items: flex-start;
  }

  .warning-banner svg {
    flex-shrink: 0;
    margin-top: 1px;
  }
</style>
