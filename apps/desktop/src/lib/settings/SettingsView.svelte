<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { getSettings, saveSettings, type AppSettings } from "$lib/tauri";

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

  let settings = $state<AppSettings>({
    roots: [],
    exclusionGlobs: ["**/node_modules/**", "**/.git/**", "**/target/**"],
    batchSize: 2000,
    vesselName: "Vessel",
    onboardingCompleted: true,
  });
  let newPath = $state("");
  let newGlob = $state("");
  let status = $state<string | null>(null);
  let error = $state<string | null>(null);
  let saving = $state(false);

  onMount(async () => {
    try {
      settings = await getSettings();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
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
    status = null;
  }

  function removeRoot(path: string) {
    settings = {
      ...settings,
      roots: settings.roots.filter((r) => r.path !== path),
    };
    status = null;
  }

  function toggleRoot(path: string) {
    settings = {
      ...settings,
      roots: settings.roots.map((r) =>
        r.path === path ? { ...r, enabled: !r.enabled } : r,
      ),
    };
    status = null;
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
    status = null;
  }

  function removeGlob(g: string) {
    settings = {
      ...settings,
      exclusionGlobs: settings.exclusionGlobs.filter((x) => x !== g),
    };
    status = null;
  }

  async function onSave() {
    saving = true;
    error = null;
    status = null;
    try {
      if (!settings.vesselName.trim()) {
        throw new Error("Vessel name cannot be empty.");
      }
      if (settings.batchSize < 200 || settings.batchSize > 20000) {
        throw new Error("Batch size must be between 200 and 20000.");
      }
      await saveSettings(settings);
      status = "Saved.";
      window.dispatchEvent(new CustomEvent("vessel-settings-changed"));
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      saving = false;
    }
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
  <header class="header">
    <h1>Settings</h1>
    <p class="lede">
      Choose which folders to index, what to skip, and how this install is labeled. Settings are saved on
      your computer.
    </p>
  </header>

  {#if error}
    <p class="error">{error}</p>
  {/if}
  {#if status}
    <p class="status">{status}</p>
  {/if}

  <section class="card">
    <h2>Vessel name</h2>
    <p class="hint">Shown in the sidebar. You can change it anytime.</p>
    <label class="field">
      <span>Name</span>
      <input type="text" bind:value={settings.vesselName} maxlength={120} />
    </label>
  </section>

  <section class="card">
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
      <input type="number" min="200" max="20000" bind:value={settings.batchSize} />
    </label>
  </section>

  <div class="actions">
    <button type="button" class="primary" disabled={saving} onclick={() => void onSave()}>
      {saving ? "Saving…" : "Save settings"}
    </button>
  </div>
</div>

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: 20px;
    max-width: 720px;
    overflow: auto;
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
    line-height: 1.5;
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
    min-width: 200px;
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
    max-width: 200px;
    padding: 8px 10px;
    border-radius: 8px;
    border: 1px solid var(--border);
    background: var(--bg);
  }

  .field input[type="text"] {
    max-width: 100%;
  }

  .actions {
    display: flex;
    gap: 12px;
  }

  .primary {
    border: none;
    border-radius: var(--radius);
    padding: 10px 18px;
    background: var(--accent);
    color: #0b1020;
    font-weight: 600;
    font-size: 14px;
  }

  .primary:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .error {
    color: var(--danger);
    margin: 0;
  }

  .status {
    color: var(--success);
    margin: 0;
  }
</style>
