<script lang="ts">
  export let useCustomServer: boolean = false;
  export let customBaseUrl: string = "";
  export let apiBaseUrl: string = "";
  export let username: string = "";
  export let password: string = "";
  export let keepSignedIn: boolean = false;
  export let rememberPassword: boolean = false;
  export let loginBusy: boolean = false;
  export let lastError: string = "";
  export let isProbablyInsecureUrl: (url: string) => boolean = () => false;

  export let onConnect: () => void = () => {};
  export let onPersistServerPrefs: () => void = () => {};
  export let onPersistAuthPrefs: () => void = () => {};
</script>

<section>
  <div style="display:flex;justify-content:space-between;align-items:center;gap:12px;flex-wrap:wrap">
    <div>
      <div style="font-weight:600">Server</div>
      <div style="font-size:12px;color:#6b7280">
        {#if useCustomServer}
          自定义：<code>{apiBaseUrl}</code>
        {:else}
          当前页面（同源）：<code>{apiBaseUrl}</code>
        {/if}
      </div>
    </div>
    <label style="display:flex;gap:8px;align-items:center;margin:0">
      <input
        type="checkbox"
        bind:checked={useCustomServer}
        on:change={() => { onPersistServerPrefs(); }}
      />
      使用自定义 Server URL
    </label>
  </div>
  {#if useCustomServer}
    <label>
      Server URL
      <input
        bind:value={customBaseUrl}
        placeholder="http(s)://host:8787"
        on:change={() => { onPersistServerPrefs(); }}
      />
    </label>
  {/if}
  <label>
    用户名
    <input bind:value={username} autocomplete="username" on:change={onPersistAuthPrefs} />
  </label>
  <label>
    密码
    <input type="password" bind:value={password} autocomplete="current-password" on:change={onPersistAuthPrefs} />
  </label>
  <div class="login-prefs">
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
  <div style="display:flex;gap:8px;flex-wrap:wrap">
    <button on:click={onConnect} disabled={loginBusy || !username.trim() || !password}>
      {loginBusy ? "登录中…" : "登录"}
    </button>
  </div>
  {#if isProbablyInsecureUrl(apiBaseUrl)}
    <div style="margin-top:8px;padding:8px;border:1px solid #f59e0b;background:#fffbeb">
      注意：当前是 <code>http://</code>，密码会明文传输。建议通过 HTTPS 访问（例如用 Caddy 反代）。
    </div>
  {/if}
  {#if lastError}
    <div style="color:#b91c1c">{lastError}</div>
  {/if}
</section>
