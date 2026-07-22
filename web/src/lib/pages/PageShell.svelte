<script lang="ts">
  import { relay } from "../stores/relay-store.svelte";
  import Toast from "../components/Toast.svelte";

  let { children }: { children: import("svelte").Snippet } = $props();

  const navItems = [
    { id: "sessions", icon: "layout-list", label: "会话" },
    { id: "start", icon: "play", label: "启动" },
    { id: "settings", icon: "settings", label: "设置" },
  ] as const;
</script>

<header class="topbar">
  <div class="topbar-left">
    <svg class="logo-icon" viewBox="0 0 24 24" fill="none" aria-hidden="true">
      <path d="M3 9l9-7 9 7v11a2 2 0 01-2 2H5a2 2 0 01-2-2V9z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
      <path d="M9 22V12h6v10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
    </svg>
    <span class="brand">Relay</span>
  </div>
  <div class="topbar-right">
    <span class="conn-badge" data-status={relay.status === "connected" ? "connected" : "disconnected"}>
      <span class="conn-dot" aria-hidden="true"></span>
      {relay.status === "connected" ? "已连接" : "未连接"}
    </span>
  </div>
</header>

<main class="page-content">
  {@render children()}
</main>

<nav class="bottom-nav">
  {#each navItems as item (item.id)}
    <button
      class="nav-btn"
      class:active={relay.view === item.id}
      onclick={() => relay.navigate(item.id)}
    >
      <svg class="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        {#if item.id === "sessions"}
          <rect x="3" y="3" width="7" height="7" rx="1"/>
          <rect x="14" y="3" width="7" height="7" rx="1"/>
          <rect x="3" y="14" width="7" height="7" rx="1"/>
          <rect x="14" y="14" width="7" height="7" rx="1"/>
        {:else if item.id === "start"}
          <polygon points="5 3 19 12 5 21 5 3"/>
        {:else if item.id === "settings"}
          <circle cx="12" cy="12" r="3"/>
          <path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 010 2.83 2 2 0 01-2.83 0l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-2 2 2 2 0 01-2-2v-.09A1.65 1.65 0 009 19.4a1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83 0 2 2 0 010-2.83l.06-.06A1.65 1.65 0 004.68 15a1.65 1.65 0 00-1.51-1H3a2 2 0 01-2-2 2 2 0 012-2h.09A1.65 1.65 0 004.6 9a1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 010-2.83 2 2 0 012.83 0l.06.06A1.65 1.65 0 009 4.68a1.65 1.65 0 001-1.51V3a2 2 0 012-2 2 2 0 012 2v.09a1.65 1.65 0 001 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 0 2 2 0 010 2.83l-.06.06A1.65 1.65 0 0019.32 9a1.65 1.65 0 001.51 1H21a2 2 0 012 2 2 2 0 01-2 2h-.09a1.65 1.65 0 00-1.51 1z"/>
        {/if}
      </svg>
      <span class="nav-label">{item.label}</span>
    </button>
  {/each}
</nav>

<Toast text={relay.toastText} />

<style>
  .topbar {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    z-index: 30;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 16px;
    padding-top: calc(8px + env(safe-area-inset-top));
    background: color-mix(in srgb, var(--bg-canvas) 92%, transparent);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    border-bottom: 1px solid var(--border);
  }

  .topbar-left {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .logo-icon {
    width: 22px;
    height: 22px;
    color: var(--accent);
  }

  .brand {
    font-family: "Hanken Grotesk", sans-serif;
    font-weight: 700;
    font-size: 18px;
    color: var(--text);
    letter-spacing: -0.02em;
  }

  .topbar-right {
    display: flex;
    align-items: center;
  }

  .conn-badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    font-weight: 600;
    color: var(--muted);
    padding: 3px 10px;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: var(--bg-surface);
  }

  .conn-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--muted);
  }

  .conn-badge[data-status="connected"] .conn-dot {
    background: var(--success);
  }

  .page-content {
    padding: 60px 16px calc(72px + env(safe-area-inset-bottom));
    padding-top: calc(60px + env(safe-area-inset-top));
    padding-bottom: calc(72px + env(safe-area-inset-bottom));
    min-height: 100dvh;
    display: flex;
    flex-direction: column;
  }

  .bottom-nav {
    position: fixed;
    bottom: 0;
    left: 0;
    right: 0;
    z-index: 30;
    display: flex;
    background: color-mix(in srgb, var(--bg-canvas) 94%, transparent);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    border-top: 1px solid var(--border);
    padding-bottom: env(safe-area-inset-bottom);
  }

  .nav-btn {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: 8px 4px;
    border: none;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
    transition: color 150ms ease;
  }

  .nav-btn.active {
    color: var(--accent);
  }

  .nav-icon {
    width: 22px;
    height: 22px;
  }

  .nav-label {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.02em;
  }
</style>
