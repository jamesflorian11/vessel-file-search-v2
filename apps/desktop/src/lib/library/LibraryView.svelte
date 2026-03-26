<script lang="ts">
  import { getSettings } from "$lib/tauri";
  import { onMount } from "svelte";

  let roots = $state<string[]>([]);
  let loadError = $state<string | null>(null);

  onMount(async () => {
    try {
      const s = await getSettings();
      roots = s.roots.filter((r) => r.enabled).map((r) => r.path);
    } catch (e) {
      loadError = e instanceof Error ? e.message : String(e);
    }
  });
</script>

<div class="view">
  <header class="header">
    <h1>Library</h1>
    <p class="lede">
      These are the folders you are currently searching. Add or change them in Settings, then run a
      scan from Activity.
    </p>
  </header>
  {#if loadError}
    <p class="error">{loadError}</p>
  {:else if roots.length === 0}
    <div class="empty">
      <p>No enabled roots yet.</p>
      <p class="hint">Open Settings to add folders, then start a scan from Activity.</p>
    </div>
  {:else}
    <ul class="list">
      {#each roots as path}
        <li class="row">{path}</li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: 20px;
    min-height: 0;
  }

  .header h1 {
    margin: 0 0 8px;
    font-size: 22px;
    font-weight: 600;
  }

  .lede {
    margin: 0;
    color: var(--text-muted);
    max-width: 560px;
    line-height: 1.5;
  }

  .empty {
    border: 1px dashed var(--border);
    border-radius: var(--radius);
    padding: 24px;
    color: var(--text-muted);
  }

  .hint {
    margin-top: 8px;
    font-size: 13px;
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .row {
    padding: 12px 14px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    font-size: 13px;
    word-break: break-all;
  }

  .error {
    color: var(--danger);
    margin: 0;
  }
</style>
