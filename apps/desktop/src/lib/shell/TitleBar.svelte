<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";

  async function minimize() {
    await getCurrentWindow().minimize();
  }

  async function toggleMaximize() {
    await getCurrentWindow().toggleMaximize();
  }

  async function close() {
    await getCurrentWindow().close();
  }
</script>

<header class="titlebar" aria-label="Vessel File Search">
  <div class="drag" data-tauri-drag-region aria-hidden="true"></div>
  <div class="controls" data-tauri-drag-region="false">
    <button type="button" class="win-btn" title="Minimize" aria-label="Minimize" onclick={() => void minimize()}>
      <svg width="10" height="1" viewBox="0 0 10 1" aria-hidden="true"><rect width="10" height="1" fill="currentColor" /></svg>
    </button>
    <button type="button" class="win-btn" title="Maximize" aria-label="Maximize" onclick={() => void toggleMaximize()}>
      <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
        <rect x="0.5" y="0.5" width="9" height="9" fill="none" stroke="currentColor" stroke-width="1" />
      </svg>
    </button>
    <button type="button" class="win-btn close" title="Close" aria-label="Close" onclick={() => void close()}>
      <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
        <path d="M1 1 L9 9 M9 1 L1 9" stroke="currentColor" stroke-width="1.2" fill="none" />
      </svg>
    </button>
  </div>
</header>

<style>
  .titlebar {
    flex-shrink: 0;
    height: 40px;
    display: flex;
    align-items: stretch;
    justify-content: space-between;
    border-bottom: 1px solid var(--border);
    background: var(--bg-elevated);
    user-select: none;
  }

  .drag {
    min-width: 0;
    flex: 1;
  }

  .controls {
    display: flex;
    align-items: stretch;
    flex-shrink: 0;
  }

  .win-btn {
    width: 46px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    display: grid;
    place-items: center;
    cursor: pointer;
    transition: background 0.12s ease, color 0.12s ease;
  }

  .win-btn:hover {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text);
  }

  .win-btn.close:hover {
    background: rgba(220, 60, 60, 0.85);
    color: #fff;
  }

  :global([data-theme="light"]) .win-btn:hover {
    background: rgba(0, 0, 0, 0.06);
  }

  :global([data-theme="light"]) .win-btn.close:hover {
    background: rgba(220, 60, 60, 0.9);
    color: #fff;
  }
</style>
