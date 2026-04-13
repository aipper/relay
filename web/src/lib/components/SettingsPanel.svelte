<script lang="ts">
  export let apiBaseUrl: string = "";
  export let health: any = null;
  export let uiVersion: string | number = "";
  export let useCustomServer: boolean = false;
  export let customBaseUrl: string = "";
  export let isProbablyInsecureUrl: (url: string) => boolean = () => false;
  export let username: string = "";
  export let keepSignedIn: boolean = false;
  export let rememberPassword: boolean = false;
  export let password: string = "";
  export let status: string = "";
  export let token: string = "";
  export let serverLogsLines: string = "";
  export let serverLogsMaxBytes: string = "";
  export let serverLogs: string = "";
  export let serverLogsPath: string = "";
  export let serverLogsTruncated: boolean = false;
  export let serverLogsError: string = "";

  export let onDisconnect: () => void = () => {};
  export let onPersistServerPrefs: () => void = () => {};
  export let onPersistAuthPrefs: () => void = () => {};
  export let onFetchServerLogs: () => void = () => {};
</script>

{#if token}
<section>
  <h2>设置</h2>
  <div style="display:flex;justify-content:space-between;align-items:center;gap:12px;flex-wrap:wrap">
    <div>
      <div style="font-weight:600">当前服务</div>
      <div class="subtle subtle-row">
        <code>{apiBaseUrl}</code>
        {#if health}
          <span>{health.name} {health.version}</span>
        {/if}
        <span class="version-pill">ui v{uiVersion}</span>
      </div>
    </div>
    <div style="display:flex;gap:8px;flex-wrap:wrap">
      <button on:click={onDisconnect} disabled={status !== "connected" && !token}>断开</button>
    </div>
  </div>

  <div style="margin-top:12px">
    <label style="display:flex;gap:8px;align-items:center;margin:0">
      <input
        type="checkbox"
        bind:checked={useCustomServer}
        on:change={() => { onPersistServerPrefs(); }}
      />
      使用自定义 Server URL（仅当 PWA 与服务不同源时需要）
    </label>
    {#if useCustomServer}
      <label style="margin-top:10px">
        Server URL
        <input
          bind:value={customBaseUrl}
          placeholder="http(s)://host:8787"
          on:change={() => { onPersistServerPrefs(); }}
        />
      </label>
    {/if}
    {#if isProbablyInsecureUrl(apiBaseUrl)}
      <div style="margin-top:10px;padding:8px;border:1px solid #f59e0b;background:#fffbeb">
        检测到 <code>http://</code>：密码与 token 在传输层不加密。建议通过 HTTPS 访问。
      </div>
    {/if}
  </div>

  <div style="margin-top:14px">
    <div style="font-weight:600">登录</div>
    <div class="subtle">当前用户：<code>{username}</code></div>
    <div class="login-prefs" style="margin-top:8px">
      <label class="checkbox">
        <input
          type="checkbox"
          bind:checked={keepSignedIn}
          on:change={() => { onPersistAuthPrefs(); }}
        />
        刷新后保持登录
      </label>
      <label class="checkbox">
        <input
          type="checkbox"
          bind:checked={rememberPassword}
          on:change={() => {
            if (!rememberPassword) password = "";
            onPersistAuthPrefs();
          }}
        />
        记住密码（本机）
      </label>
    </div>
    {#if rememberPassword}
      <label style="margin-top:10px">
        密码
        <input type="password" bind:value={password} autocomplete="current-password" on:change={onPersistAuthPrefs} />
      </label>
    {/if}
  </div>

  <div style="margin-top:14px">
    <div style="font-weight:600">Server 日志</div>
    <div style="display:flex;gap:8px;flex-wrap:wrap;align-items:flex-end;margin:8px 0">
      <label style="flex:1;min-width:140px">
        lines
        <input bind:value={serverLogsLines} placeholder="200" />
      </label>
      <label style="flex:1;min-width:140px">
        max_bytes
        <input bind:value={serverLogsMaxBytes} placeholder="200000" />
      </label>
      <button on:click={onFetchServerLogs} disabled={status !== "connected"}>server.logs.tail</button>
    </div>
    {#if serverLogsError}
      <div style="color:#b91c1c">{serverLogsError}</div>
    {/if}
    {#if serverLogsPath}
      <div class="subtle" style="margin-bottom:6px;display:flex;gap:8px;align-items:center;flex-wrap:wrap">
        <code>{serverLogsPath}</code>
        {#if serverLogsTruncated}
          <span class="subtle">truncated</span>
        {/if}
      </div>
      <pre style="white-space:pre-wrap;word-break:break-word;max-height:240px;overflow:auto;border:1px solid #e5e7eb;padding:12px">{serverLogs}</pre>
    {/if}
  </div>
</section>
{/if}
