<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { getSettings, saveSettings, type AppSettings } from "$lib/tauri";
  import JobsView from "$lib/jobs/JobsView.svelte";
  import { applyTheme, type Theme } from "$lib/theme";

  /** Match Rust path_norm::normalize_root_path (trim + strip wrapping quotes). */
  function normalizePathInput(s: string): string {
    let t = s.trim();
    for (let i = 0; i < 4; i++) {
      const trimmed = t.trim();
      if (trimmed.length < 2) return trimmed;
      const first = trimmed[0];
      const last = trimmed[trimmed.length - 1];
      if ((first === '"' || first === "'") && first === last) {
        t = trimmed.slice(1, -1).trim();
      } else {
        return trimmed;
      }
    }
    return t;
  }

  const DEBOUNCE_MS = 400;

  let settings = $state<AppSettings>({
    roots: [],
    exclusionGlobs: ["**/node_modules/**", "**/.git/**", "**/target/**"],
    batchSize: 2000,
    vesselName: "Vessel",
    onboardingCompleted: true,
    theme: "dark",
  });
  let newPath = $state("");
  let newGlob = $state("");
  let error = $state<string | null>(null);
  /** Subtle header feedback; not shown when null. */
  let saveFeedback = $state<"saving" | "saved" | "failed" | null>(null);

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let saveInFlight = false;
  let saveQueued = false;
  let savedClearTimer: ReturnType<typeof setTimeout> | null = null;

  function clearSavedTimer() {
    if (savedClearTimer) {
      clearTimeout(savedClearTimer);
      savedClearTimer = null;
    }
  }

  function scheduleDebouncedSave() {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      debounceTimer = null;
      void runSave();
    }, DEBOUNCE_MS);
  }

  function persistNow() {
    if (debounceTimer) {
      clearTimeout(debounceTimer);
      debounceTimer = null;
    }
    void runSave();
  }

  async function runSave() {
    if (saveInFlight) {
      saveQueued = true;
      return;
    }
    saveInFlight = true;
    clearSavedTimer();
    saveFeedback = "saving";
    try {
      while (true) {
        saveQueued = false;
        if (!settings.vesselName.trim()) {
          throw new Error("Vessel name cannot be empty.");
        }
        if (settings.batchSize < 200 || settings.batchSize > 20000) {
          throw new Error("Batch size must be between 200 and 20000.");
        }
        await saveSettings(settings);
        window.dispatchEvent(new CustomEvent("vessel-settings-changed"));
        if (!saveQueued) break;
      }
      error = null;
      saveFeedback = "saved";
      savedClearTimer = setTimeout(() => {
        savedClearTimer = null;
        saveFeedback = null;
      }, 2000);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      saveFeedback = "failed";
    } finally {
      saveInFlight = false;
    }
  }

  onMount(() => {
    void (async () => {
      try {
        settings = await getSettings();
      } catch (e) {
        error = e instanceof Error ? e.message : String(e);
      }
    })();
    return () => {
      if (debounceTimer) clearTimeout(debounceTimer);
      clearSavedTimer();
    };
  });

  function addRoot() {
    const path = normalizePathInput(newPath);
    if (!path) return;
    const roots = [...settings.roots];
    if (roots.some((r) => r.path === path)) {
      error = "That path is already listed.";
      return;
    }
    roots.push({ path, displayName: null, enabled: true });
    settings = { ...settings, roots };
    newPath = "";
    error = null;
    persistNow();
  }

  function removeRoot(path: string) {
    settings = {
      ...settings,
      roots: settings.roots.filter((r) => r.path !== path),
    };
    persistNow();
  }

  function toggleRoot(path: string) {
    settings = {
      ...settings,
      roots: settings.roots.map((r) =>
        r.path === path ? { ...r, enabled: !r.enabled } : r,
      ),
    };
    persistNow();
  }

  function addGlob() {
    const g = newGlob.trim();
    if (!g) return;
    if (settings.exclusionGlobs.includes(g)) return;
    settings = {
      ...settings,
      exclusionGlobs: [...settings.exclusionGlobs, g],
    };
    newGlob = "";
    persistNow();
  }

  function removeGlob(g: string) {
    settings = {
      ...settings,
      exclusionGlobs: settings.exclusionGlobs.filter((x) => x !== g),
    };
    persistNow();
  }

  function setTheme(t: Theme) {
    settings = { ...settings, theme: t };
    applyTheme(t);
    persistNow();
  }

  function onVesselInput(e: Event) {
    const v = (e.currentTarget as HTMLInputElement).value;
    settings = { ...settings, vesselName: v };
    scheduleDebouncedSave();
  }

  function onBatchInput(e: Event) {
    const raw = (e.currentTarget as HTMLInputElement).value;
    const v = parseInt(raw, 10);
    if (Number.isNaN(v)) return;
    settings = { ...settings, batchSize: v };
    scheduleDebouncedSave();
  }

  async function browseFolder() {
    error = null;
    try {
      const selected = await open({
        directory: true,
        multiple: false,
      });
      if (typeof selected === "string") {
        newPath = normalizePathInput(selected);
      } else if (Array.isArray(selected) && selected[0]) {
        newPath = normalizePathInput(selected[0]);
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
</script>

<div class="view">
  <header class="header-row">
    <div class="header-text">
      <h1>Settings</h1>
      <p class="lede">
        Index locations, exclusions, appearance, and indexing options for this install. Changes save
        automatically.
      </p>
    </div>
    {#if saveFeedback}
      <p class="save-feedback" data-kind={saveFeedback}>
        {#if saveFeedback === "saving"}
          Saving…
        {:else if saveFeedback === "saved"}
          Saved
        {:else}
          Save failed
        {/if}
      </p>
    {/if}
  </header>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <div class="layout-grid">
    <div class="col col-left">
      <section class="card">
        <h2>Vessel name</h2>
        <p class="hint">Shown in the sidebar.</p>
        <label class="field">
          <span>Name</span>
          <input type="text" value={settings.vesselName} maxlength={120} oninput={onVesselInput} />
        </label>
      </section>

      <section class="card card-roots">
        <h2>Indexed folders</h2>
        <p class="hint">Add one or more folders to search. Turn off a folder to skip it on the next scan.</p>
        <div class="row-input">
          <input
            type="text"
            placeholder="e.g. D:\Projects"
            bind:value={newPath}
          />
          <button type="button" class="secondary" onclick={() => void browseFolder()}>Browse…</button>
          <button type="button" class="secondary" onclick={addRoot}>Add root</button>
        </div>
        <ul class="items">
          {#each settings.roots as r}
            <li class="item">
              <label class="toggle">
                <input
                  type="checkbox"
                  checked={r.enabled}
                  onchange={() => toggleRoot(r.path)}
                />
                <span>enabled</span>
              </label>
              <span class="path">{r.path}</span>
              <button type="button" class="linkish" onclick={() => removeRoot(r.path)}>Remove</button>
            </li>
          {/each}
        </ul>
      </section>
    </div>

    <div class="col col-right">
      <section class="card">
        <h2>Appearance</h2>
        <p class="hint">Light or dark interface.</p>
        <div class="segment" role="group" aria-label="Theme">
          <button
            type="button"
            class="seg-btn"
            class:active={settings.theme === "light"}
            onclick={() => setTheme("light")}
          >
            Light
          </button>
          <button
            type="button"
            class="seg-btn"
            class:active={settings.theme === "dark"}
            onclick={() => setTheme("dark")}
          >
            Dark
          </button>
        </div>
      </section>

      <section class="card">
        <h2>Exclusions</h2>
        <p class="hint">Paths matching these patterns are skipped (same idea as .gitignore).</p>
        <div class="row-input">
          <input type="text" placeholder="**/.cache/**" bind:value={newGlob} />
          <button type="button" class="secondary" onclick={addGlob}>Add pattern</button>
        </div>
        <ul class="chips">
          {#each settings.exclusionGlobs as g}
            <li>
              <code>{g}</code>
              <button type="button" class="linkish" onclick={() => removeGlob(g)}>×</button>
            </li>
          {/each}
        </ul>
      </section>

      <section class="card">
        <h2>Indexing</h2>
        <label class="field">
          <span>Batch size (rows per database transaction)</span>
          <input
            type="number"
            min="200"
            max="20000"
            value={settings.batchSize}
            oninput={onBatchInput}
          />
        </label>
      </section>

      <section class="card activity-card">
        <h2>Indexing activity</h2>
        <p class="hint">Scan jobs and history. Start or rescan from the Search screen.</p>
        <div class="activity-embed">
          <JobsView embedded />
        </div>
      </section>
    </div>
  </div>
</div>

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: 16px;
    width: 100%;
    max-width: 1200px;
    margin: 0 auto;
    overflow: auto;
    min-height: 0;
  }

  .header-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
  }

  .header-text {
    min-width: 0;
    flex: 1;
  }

  .header-row h1 {
    margin: 0 0 8px;
    font-size: 22px;
    font-weight: 600;
  }

  .lede {
    margin: 0;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .save-feedback {
    margin: 4px 0 0;
    font-size: 12px;
    letter-spacing: 0.02em;
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .save-feedback[data-kind="saving"] {
    color: var(--text-muted);
  }

  .save-feedback[data-kind="saved"] {
    color: var(--success);
    opacity: 0.85;
  }

  .save-feedback[data-kind="failed"] {
    color: var(--danger);
  }

  .layout-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 20px;
    align-items: start;
    min-width: 0;
  }

  @media (max-width: 960px) {
    .layout-grid {
      grid-template-columns: 1fr;
    }
  }

  .col {
    display: flex;
    flex-direction: column;
    gap: 20px;
    min-width: 0;
  }

  .card {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 18px 20px;
    background: var(--bg-elevated);
  }

  .card h2 {
    margin: 0 0 8px;
    font-size: 15px;
    font-weight: 600;
  }

  .hint {
    margin: 0 0 14px;
    font-size: 13px;
    color: var(--text-muted);
    line-height: 1.45;
  }

  .row-input {
    display: flex;
    gap: 10px;
    margin-bottom: 14px;
    flex-wrap: wrap;
  }

  .row-input input {
    flex: 1;
    min-width: 160px;
    padding: 10px 12px;
    border-radius: 8px;
    border: 1px solid var(--border);
    background: var(--bg);
  }

  .secondary {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px 14px;
    background: transparent;
    color: var(--text);
    font-size: 14px;
  }

  .items {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    flex-wrap: wrap;
  }

  .path {
    flex: 1;
    font-size: 13px;
    word-break: break-all;
  }

  .toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .chips {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .chips li {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
  }

  code {
    background: var(--bg);
    padding: 6px 8px;
    border-radius: 6px;
    border: 1px solid var(--border);
    flex: 1;
    word-break: break-all;
  }

  .linkish {
    border: none;
    background: none;
    color: var(--accent);
    font-size: 13px;
    padding: 4px 6px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 13px;
    color: var(--text-muted);
  }

  .field input {
    max-width: 100%;
    padding: 8px 10px;
    border-radius: 8px;
    border: 1px solid var(--border);
    background: var(--bg);
  }

  .error {
    color: var(--danger);
    margin: 0;
  }

  .segment {
    display: flex;
    gap: 0;
    border-radius: 8px;
    border: 1px solid var(--border);
    overflow: hidden;
    width: fit-content;
  }

  .seg-btn {
    border: none;
    padding: 10px 18px;
    background: var(--bg);
    color: var(--text-muted);
    font-size: 14px;
    cursor: pointer;
    transition: background 0.15s ease, color 0.15s ease;
  }

  .seg-btn + .seg-btn {
    border-left: 1px solid var(--border);
  }

  .seg-btn:hover {
    color: var(--text);
  }

  .seg-btn.active {
    background: rgba(79, 140, 255, 0.18);
    color: var(--text);
    font-weight: 600;
  }

  :global([data-theme="light"]) .seg-btn.active {
    background: rgba(45, 98, 216, 0.15);
  }

  .activity-card .hint {
    margin-bottom: 12px;
  }

  .activity-embed {
    margin: 0 -4px;
    max-height: min(480px, 52vh);
    overflow: auto;
    padding: 4px;
  }
</style>
