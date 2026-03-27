<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { getSettings, saveSettings, type AppSettings } from "$lib/tauri";
  import { applyTheme, type Theme } from "$lib/theme";

  let { onCompleted }: { onCompleted: () => void } = $props();

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

  let vesselName = $state("");
  let folderPath = $state("");
  let themeChoice = $state<Theme>("dark");
  let error = $state<string | null>(null);
  let busy = $state(false);

  function setTheme(t: Theme) {
    themeChoice = t;
    applyTheme(t);
  }

  onMount(async () => {
    try {
      const s = await getSettings();
      vesselName = s.vesselName.trim() || "Vessel";
      const t = s.theme === "light" ? "light" : "dark";
      themeChoice = t;
      applyTheme(t);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  });

  async function browse() {
    error = null;
    try {
      const selected = await open({
        directory: true,
        multiple: false,
      });
      if (typeof selected === "string") {
        folderPath = normalizePathInput(selected);
      } else if (Array.isArray(selected) && selected[0]) {
        folderPath = normalizePathInput(selected[0]);
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function finish() {
    const name = vesselName.trim();
    if (!name) {
      error = "Enter a vessel name.";
      return;
    }
    if (!folderPath.trim()) {
      error = "Choose a default folder to index.";
      return;
    }
    busy = true;
    error = null;
    try {
      const s = await getSettings();
      const roots = [...s.roots];
      if (folderPath.trim()) {
        const p = normalizePathInput(folderPath);
        if (!roots.some((r) => r.path === p)) {
          roots.push({ path: p, displayName: null, enabled: true, readOnly: false });
        }
      }
      const next: AppSettings = {
        ...s,
        vesselName: name,
        roots,
        onboardingCompleted: true,
        theme: themeChoice,
      };
      await saveSettings(next);
      window.dispatchEvent(new CustomEvent("vessel-settings-changed"));
      onCompleted();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="overlay" role="dialog" aria-modal="true" aria-labelledby="setup-title">
  <div class="panel">
    <h1 id="setup-title">Welcome</h1>
    <p class="lede">
      Name this install, pick a default folder to index, and choose appearance. You can change everything later in Settings.
    </p>

    {#if error}
      <p class="error">{error}</p>
    {/if}

    <label class="field">
      <span>Vessel name</span>
      <input type="text" bind:value={vesselName} maxlength={120} autocomplete="off" />
    </label>

    <div class="field">
      <span>Default folder to index</span>
      <div class="row-input">
        <input type="text" readonly value={folderPath} placeholder="Browse to choose a folder" />
        <button type="button" class="secondary" onclick={() => void browse()}>Browse…</button>
      </div>
      <p class="hint">Required for first setup. You can add more folders in Settings.</p>
    </div>

    <div class="field">
      <span>Appearance</span>
      <div class="segment" role="group" aria-label="Theme">
        <button
          type="button"
          class="seg-btn"
          class:active={themeChoice === "light"}
          onclick={() => setTheme("light")}
        >
          Light
        </button>
        <button
          type="button"
          class="seg-btn"
          class:active={themeChoice === "dark"}
          onclick={() => setTheme("dark")}
        >
          Dark
        </button>
      </div>
    </div>

    <div class="actions">
      <button type="button" class="primary" disabled={busy} onclick={() => void finish()}>
        {busy ? "Saving…" : "Continue"}
      </button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 2000;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgba(8, 10, 16, 0.72);
    backdrop-filter: blur(4px);
  }

  .panel {
    width: min(440px, 100%);
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--bg-elevated);
    padding: 28px 24px;
    box-shadow: var(--shadow);
  }

  h1 {
    margin: 0 0 8px;
    font-size: 20px;
    font-weight: 600;
  }

  .lede {
    margin: 0 0 20px;
    font-size: 14px;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 18px;
    font-size: 13px;
    color: var(--text-muted);
  }

  .field input[type="text"] {
    padding: 10px 12px;
    border-radius: 8px;
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--text);
    font-size: 14px;
  }

  .row-input {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
  }

  .row-input input {
    flex: 1;
    min-width: 160px;
  }

  .hint {
    margin: 6px 0 0;
    font-size: 12px;
    color: var(--text-muted);
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

  .secondary {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 10px 14px;
    background: transparent;
    color: var(--text);
    font-size: 14px;
  }

  .actions {
    margin-top: 8px;
  }

  .primary {
    border: none;
    border-radius: var(--radius);
    padding: 10px 20px;
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
    font-size: 13px;
    margin: 0 0 12px;
  }
</style>
