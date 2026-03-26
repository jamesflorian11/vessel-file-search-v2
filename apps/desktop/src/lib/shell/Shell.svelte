<script lang="ts">
  import { onMount } from "svelte";
  import SearchView from "$lib/search/SearchView.svelte";
  import LibraryView from "$lib/library/LibraryView.svelte";
  import JobsView from "$lib/jobs/JobsView.svelte";
  import SettingsView from "$lib/settings/SettingsView.svelte";
  import SetupModal from "$lib/setup/SetupModal.svelte";
  import { getSettings } from "$lib/tauri";

  type Route = "search" | "library" | "jobs" | "settings";

  let route = $state<Route>("search");
  let vesselName = $state("Vessel");
  let settingsReady = $state(false);
  let onboardingCompleted = $state(true);

  let brandInitial = $derived.by(() => {
    const c = vesselName.trim().charAt(0);
    return c ? c.toUpperCase() : "V";
  });

  async function refreshSettings() {
    try {
      const s = await getSettings();
      vesselName = s.vesselName.trim() || "Vessel";
      onboardingCompleted = s.onboardingCompleted;
    } catch {
      // keep previous values
    } finally {
      settingsReady = true;
    }
  }

  onMount(() => {
    const on = () => void refreshSettings();
    window.addEventListener("vessel-settings-changed", on);
    return () => window.removeEventListener("vessel-settings-changed", on);
  });

  $effect(() => {
    route;
    void refreshSettings();
  });

  const nav: { id: Route; label: string }[] = [
    { id: "search", label: "Search" },
    { id: "library", label: "Library" },
    { id: "jobs", label: "Activity" },
    { id: "settings", label: "Settings" },
  ];
</script>

<div class="app-root">
  <aside class="rail" aria-label="Main navigation">
    <div class="brand">
      <span class="brand-mark">{brandInitial}</span>
      <div class="brand-text">
        <div class="brand-title">{vesselName.trim() || "Vessel"}</div>
        <div class="brand-sub">File Search</div>
      </div>
    </div>
    <nav>
      {#each nav as item}
        <button
          type="button"
          class="nav-item"
          class:active={route === item.id}
          onclick={() => (route = item.id)}
        >
          {item.label}
        </button>
      {/each}
    </nav>
  </aside>
  <main class="main">
    {#if route === "search"}
      <SearchView />
    {:else if route === "library"}
      <LibraryView />
    {:else if route === "jobs"}
      <JobsView />
    {:else}
      <SettingsView />
    {/if}
  </main>
</div>

{#if settingsReady && !onboardingCompleted}
  <SetupModal onCompleted={() => void refreshSettings()} />
{/if}

<style>
  .app-root {
    display: flex;
    height: 100%;
    min-height: 0;
  }

  .rail {
    width: 220px;
    flex-shrink: 0;
    background: var(--bg-elevated);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    padding: 20px 14px;
    gap: 24px;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 6px;
  }

  .brand-mark {
    width: 40px;
    height: 40px;
    border-radius: 12px;
    background: linear-gradient(145deg, var(--accent), #2a4a9e);
    display: grid;
    place-items: center;
    font-weight: 700;
    font-size: 18px;
    box-shadow: var(--shadow);
  }

  .brand-title {
    font-weight: 600;
    letter-spacing: 0.02em;
  }

  .brand-sub {
    font-size: 12px;
    color: var(--text-muted);
    margin-top: 2px;
  }

  nav {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .nav-item {
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

  .nav-item:hover {
    background: rgba(255, 255, 255, 0.04);
    color: var(--text);
  }

  .nav-item.active {
    background: rgba(79, 140, 255, 0.15);
    color: var(--text);
    font-weight: 500;
  }

  .main {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: 28px 32px;
    overflow: hidden;
  }
</style>
