<script lang="ts">
  import { onMount } from "svelte";
  import logoPng from "../../assets/logo.png";
  import SearchView from "$lib/search/SearchView.svelte";
  import SettingsView from "$lib/settings/SettingsView.svelte";
  import SetupModal from "$lib/setup/SetupModal.svelte";
  import TitleBar from "$lib/shell/TitleBar.svelte";
  import { getSettings } from "$lib/tauri";
  import { applyTheme } from "$lib/theme";

  const SIDEBAR_KEY = "vessel-sidebar-collapsed";

  type Route = "search" | "settings";

  let route = $state<Route>("search");
  let vesselName = $state("Vessel");
  let settingsReady = $state(false);
  let onboardingCompleted = $state(true);

  let logoFailed = $state(false);

  let sidebarCollapsed = $state(false);

  let brandInitial = $derived.by(() => {
    const c = vesselName.trim().charAt(0);
    return c ? c.toUpperCase() : "V";
  });

  function onLogoError() {
    logoFailed = true;
  }

  function readSidebarCollapsed(): boolean {
    try {
      return localStorage.getItem(SIDEBAR_KEY) === "1";
    } catch {
      return false;
    }
  }

  function persistSidebarCollapsed(collapsed: boolean) {
    try {
      localStorage.setItem(SIDEBAR_KEY, collapsed ? "1" : "0");
    } catch {
      /* ignore */
    }
  }

  function toggleSidebar() {
    sidebarCollapsed = !sidebarCollapsed;
    persistSidebarCollapsed(sidebarCollapsed);
  }

  async function refreshSettings() {
    try {
      const s = await getSettings();
      vesselName = s.vesselName.trim() || "Vessel";
      onboardingCompleted = s.onboardingCompleted;
      applyTheme(s.theme);
    } catch {
      // keep previous values
    } finally {
      settingsReady = true;
    }
  }

  onMount(() => {
    sidebarCollapsed = readSidebarCollapsed();
    void refreshSettings();
    const on = () => void refreshSettings();
    window.addEventListener("vessel-settings-changed", on);
    return () => window.removeEventListener("vessel-settings-changed", on);
  });

  /** Extend this array when adding e.g. Categories. */
  const navItems: { id: Route; label: string }[] = [
    { id: "search", label: "Search" },
    { id: "settings", label: "Settings" },
  ];
</script>

<div class="app-shell">
  <TitleBar />
  <div class="app-body">
    <aside class="rail" class:collapsed={sidebarCollapsed} aria-label="Main navigation">
      <div class="brand" title={vesselName.trim() || "Vessel"}>
        <div class="brand-stack" class:collapsed={sidebarCollapsed}>
          {#if !logoFailed}
            <div class="brand-logo-wrap" class:compact={sidebarCollapsed}>
              <img class="brand-logo" src={logoPng} alt="" width="110" height="110" onerror={onLogoError} />
            </div>
          {:else}
            <span class="brand-mark" class:compact={sidebarCollapsed}>{brandInitial}</span>
          {/if}
          {#if !sidebarCollapsed}
            <div class="brand-title">{vesselName.trim() || "Vessel"}</div>
          {/if}
        </div>
      </div>
      <nav class="nav" aria-label="Primary">
        {#each navItems as item}
          <button
            type="button"
            class="nav-item"
            class:active={route === item.id}
            title={item.label}
            aria-current={route === item.id ? "page" : undefined}
            aria-label={item.label}
            onclick={() => (route = item.id)}
          >
            {#if item.id === "search"}
              <span class="nav-ico" aria-hidden="true">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
                  <circle cx="11" cy="11" r="7" />
                  <path d="M20 20l-3-3" />
                </svg>
              </span>
            {:else}
              <span class="nav-ico" aria-hidden="true">
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <circle cx="12" cy="12" r="3" />
                  <path
                    d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"
                  />
                </svg>
              </span>
            {/if}
            <span class="nav-label">{item.label}</span>
          </button>
        {/each}
      </nav>
      <div class="rail-footer">
        <button
          type="button"
          class="collapse-toggle"
          aria-expanded={!sidebarCollapsed}
          title={sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}
          onclick={toggleSidebar}
        >
          <span class="collapse-ico" aria-hidden="true">
            {#if sidebarCollapsed}
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M9 6l6 6-6 6" />
              </svg>
            {:else}
              <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M15 6l-6 6 6 6" />
              </svg>
            {/if}
          </span>
          {#if !sidebarCollapsed}
            <span class="collapse-label">Collapse</span>
          {/if}
        </button>
      </div>
    </aside>
    <main class="main">
      {#if route === "search"}
        <SearchView />
      {:else}
        <SettingsView />
      {/if}
    </main>
  </div>
</div>

{#if settingsReady && !onboardingCompleted}
  <SetupModal onCompleted={() => void refreshSettings()} />
{/if}

<style>
  .app-shell {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .app-body {
    flex: 1;
    min-height: 0;
    display: flex;
    min-width: 0;
  }

  .rail {
    width: 240px;
    flex-shrink: 0;
    background: var(--bg-elevated);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    padding: 18px 14px 14px;
    gap: 0;
    transition: width 0.18s ease;
  }

  .rail.collapsed {
    width: 56px;
    padding-left: 10px;
    padding-right: 10px;
  }

  .brand {
    flex-shrink: 0;
    margin-bottom: 14px;
  }

  .brand-stack {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 24px;
  }

  .brand-stack.collapsed {
    gap: 0;
  }

  .brand-logo-wrap {
    width: 110px;
    height: 110px;
    border-radius: 50%;
    overflow: hidden;
    flex-shrink: 0;
    box-shadow: 0 1px 6px rgba(0, 0, 0, 0.16);
  }

  :global([data-theme="light"]) .brand-logo-wrap {
    box-shadow: 0 1px 5px rgba(0, 0, 0, 0.06);
  }

  .brand-logo-wrap.compact {
    width: 32px;
    height: 32px;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.14);
  }

  .brand-logo {
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center;
    transform: scale(1.045);
    display: block;
  }

  .brand-mark {
    width: 110px;
    height: 110px;
    border-radius: 50%;
    background: linear-gradient(145deg, var(--accent), #2a4a9e);
    display: grid;
    place-items: center;
    font-weight: 700;
    font-size: 42px;
    flex-shrink: 0;
    color: #fff;
    box-shadow: 0 1px 6px rgba(0, 0, 0, 0.16);
  }

  .brand-mark.compact {
    width: 32px;
    height: 32px;
    font-size: 14px;
  }

  .brand-title {
    font-weight: 700;
    letter-spacing: 0.03em;
    font-size: 17px;
    line-height: 1.3;
    word-break: break-word;
    color: var(--text);
    max-width: 100%;
    padding: 0 2px;
  }

  .nav {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
    min-height: 0;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 10px;
    text-align: left;
    border: none;
    border-radius: var(--radius);
    padding: 10px 12px;
    background: transparent;
    color: var(--text-muted);
    font-size: 14px;
    transition:
      background 0.15s ease,
      color 0.15s ease;
  }

  .rail.collapsed .nav-item {
    justify-content: center;
    padding: 10px;
  }

  .rail.collapsed .nav-label {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  .nav-item {
    position: relative;
  }

  .nav-ico {
    display: grid;
    place-items: center;
    flex-shrink: 0;
    opacity: 0.92;
  }

  .nav-item:hover {
    background: rgba(255, 255, 255, 0.04);
    color: var(--text);
  }

  :global([data-theme="light"]) .nav-item:hover {
    background: rgba(0, 0, 0, 0.05);
  }

  .nav-item.active {
    background: rgba(79, 140, 255, 0.15);
    color: var(--text);
    font-weight: 500;
  }

  :global([data-theme="light"]) .nav-item.active {
    background: rgba(45, 98, 216, 0.12);
  }

  .rail-footer {
    margin-top: auto;
    padding-top: 10px;
    border-top: 1px solid var(--border);
  }

  .collapse-toggle {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: flex-start;
    gap: 8px;
    padding: 10px 12px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-muted);
    font-size: 13px;
    cursor: pointer;
    opacity: 0.78;
    transition:
      background 0.15s ease,
      color 0.15s ease,
      opacity 0.15s ease;
  }

  .rail.collapsed .collapse-toggle {
    justify-content: center;
    padding: 10px;
  }

  .collapse-toggle:hover {
    background: rgba(255, 255, 255, 0.04);
    color: var(--text);
    opacity: 1;
  }

  :global([data-theme="light"]) .collapse-toggle:hover {
    background: rgba(0, 0, 0, 0.05);
  }

  .collapse-ico {
    display: grid;
    place-items: center;
    flex-shrink: 0;
  }

  .rail.collapsed .collapse-label {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
  }

  .main {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: 24px 28px;
    overflow: hidden;
  }
</style>
