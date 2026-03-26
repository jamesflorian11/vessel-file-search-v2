<script lang="ts">
  import { onMount } from "svelte";
  import {
    openFile,
    revealInExplorer,
    searchFiles,
    type SearchHit,
  } from "$lib/tauri";

  let query = $state("");
  let hits = $state<SearchHit[]>([]);
  let totalLoaded = $state(0);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let actionError = $state<string | null>(null);
  let exhausted = $state(false);
  let selectedIndex = $state<number | null>(null);

  const rowHeight = 44;
  const viewport = 520;
  let scrollTop = $state(0);
  let scrollerEl = $state<HTMLDivElement | undefined>(undefined);
  let searchInputEl = $state<HTMLInputElement | undefined>(undefined);
  let ctxMenu = $state<{ x: number; y: number; hit: SearchHit } | null>(null);
  let ctxMenuEl = $state<HTMLDivElement | undefined>(undefined);

  const pageSize = 200;

  function fullPath(h: SearchHit): string {
    return h.fullPath;
  }

  let selectedHit = $derived.by(() => {
    if (selectedIndex === null) return null;
    if (selectedIndex < 0 || selectedIndex >= hits.length) return null;
    return hits[selectedIndex];
  });

  async function runSearch(reset: boolean) {
    const q = query.trim();
    if (!q) {
      hits = [];
      totalLoaded = 0;
      exhausted = false;
      selectedIndex = null;
      return;
    }
    loading = true;
    error = null;
    actionError = null;
    if (reset) exhausted = false;
    const offset = reset ? 0 : hits.length;
    try {
      const batch = await searchFiles(q, pageSize, offset);
      if (batch.length < pageSize) exhausted = true;
      hits = reset ? batch : [...hits, ...batch];
      totalLoaded = hits.length;
      if (reset) selectedIndex = null;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  function onSubmit(e: Event) {
    e.preventDefault();
    scrollTop = 0;
    if (scrollerEl) scrollerEl.scrollTop = 0;
    void runSearch(true);
  }

  function formatSize(n: number): string {
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
    return `${(n / (1024 * 1024)).toFixed(1)} MB`;
  }

  function formatTime(ns: number): string {
    if (ns <= 0) return "—";
    const d = new Date(ns / 1_000_000);
    return d.toLocaleString();
  }

  let visible = $derived.by(() => {
    const overscan = 8;
    const start = Math.max(0, Math.floor(scrollTop / rowHeight) - overscan);
    const count = Math.ceil(viewport / rowHeight) + overscan * 2;
    const slice = hits.slice(start, start + count);
    return { start, slice };
  });

  async function onScroll(e: Event) {
    const el = e.currentTarget as HTMLDivElement;
    scrollTop = el.scrollTop;
    const nearBottom = el.scrollTop + el.clientHeight > el.scrollHeight - rowHeight * 10;
    if (
      nearBottom &&
      !loading &&
      !exhausted &&
      query.trim() &&
      hits.length > 0
    ) {
      await runSearch(false);
    }
  }

  function scrollRowIntoView(index: number) {
    const el = scrollerEl;
    if (!el || index < 0 || index >= hits.length) return;
    const rowTop = index * rowHeight;
    const rowBottom = rowTop + rowHeight;
    const viewTop = el.scrollTop;
    const viewBottom = viewTop + viewport;
    if (rowTop < viewTop) {
      el.scrollTop = rowTop;
    } else if (rowBottom > viewBottom) {
      el.scrollTop = rowBottom - viewport;
    }
    scrollTop = el.scrollTop;
  }

  async function openPath(path: string) {
    actionError = null;
    closeCtxMenu();
    try {
      await openFile(path);
    } catch (e) {
      actionError = e instanceof Error ? e.message : String(e);
    }
  }

  async function revealPath(path: string) {
    actionError = null;
    closeCtxMenu();
    try {
      await revealInExplorer(path);
    } catch (e) {
      actionError = e instanceof Error ? e.message : String(e);
    }
  }

  async function copyPathString(path: string) {
    actionError = null;
    closeCtxMenu();
    try {
      await navigator.clipboard.writeText(path);
    } catch (e) {
      actionError = e instanceof Error ? e.message : String(e);
    }
  }

  async function copySelectedPath() {
    const h = selectedHit;
    if (!h) return;
    await copyPathString(fullPath(h));
  }

  function selectRow(globalIndex: number) {
    selectedIndex = globalIndex;
    scrollerEl?.focus();
  }

  function moveSelection(delta: number) {
    if (hits.length === 0) return;
    let next: number;
    if (selectedIndex === null) {
      next = delta > 0 ? 0 : hits.length - 1;
    } else {
      next = Math.max(0, Math.min(hits.length - 1, selectedIndex + delta));
    }
    selectedIndex = next;
    scrollRowIntoView(next);
    scrollerEl?.focus();
  }

  function onRowClick(globalIndex: number) {
    closeCtxMenu();
    selectRow(globalIndex);
  }

  function onRowDblClick(h: SearchHit) {
    closeCtxMenu();
    void openPath(fullPath(h));
  }

  function closeCtxMenu() {
    ctxMenu = null;
  }

  function onRowContextMenu(e: MouseEvent, globalIndex: number, h: SearchHit) {
    e.preventDefault();
    selectedIndex = globalIndex;
    ctxMenu = { x: e.clientX, y: e.clientY, hit: h };
  }

  function onScrollerKeydown(e: KeyboardEvent) {
    if (ctxMenu) {
      if (e.key === "Escape") {
        e.preventDefault();
        closeCtxMenu();
      }
      return;
    }

    if (e.key === "ArrowDown") {
      e.preventDefault();
      moveSelection(1);
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      moveSelection(-1);
      return;
    }
    if (e.key === "Escape") {
      e.preventDefault();
      if (selectedIndex !== null) {
        selectedIndex = null;
      } else {
        searchInputEl?.focus();
      }
      return;
    }
    if (e.key === "Enter" && !e.repeat) {
      if (selectedHit === null) return;
      e.preventDefault();
      void openPath(fullPath(selectedHit));
    }
  }

  function onSearchKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && selectedIndex !== null) {
      e.preventDefault();
      selectedIndex = null;
    }
  }

  onMount(() => {
    const onDocClick = (ev: MouseEvent) => {
      if (!ctxMenu) return;
      const t = ev.target as Node;
      if (ctxMenuEl?.contains(t)) return;
      closeCtxMenu();
    };
    document.addEventListener("click", onDocClick, true);
    return () => document.removeEventListener("click", onDocClick, true);
  });
</script>

<div class="view">
  <header class="header">
    <h1>Search</h1>
    <p class="lede">
      Find files by name or path across folders you have indexed. Large result sets load in
      pages as you scroll.
    </p>
  </header>

  <form class="search-bar" onsubmit={onSubmit}>
    <input
      bind:this={searchInputEl}
      type="search"
      placeholder="Search by file or path"
      bind:value={query}
      autocomplete="off"
      onkeydown={onSearchKeydown}
    />
    <button type="submit" class="primary">Search</button>
  </form>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <div class="frame">
    <div class="col-head">
      <span class="h path">Path</span>
      <span class="h size">Size</span>
      <span class="h time">Modified</span>
    </div>
    <div class="actions" aria-label="Result actions">
      <button
        type="button"
        class="secondary"
        disabled={selectedHit === null}
        onclick={() => selectedHit && void openPath(fullPath(selectedHit))}
      >
        Open File
      </button>
      <button
        type="button"
        class="secondary"
        disabled={selectedHit === null}
        onclick={() => selectedHit && void revealPath(fullPath(selectedHit))}
      >
        Open Folder
      </button>
      <button
        type="button"
        class="secondary"
        disabled={selectedHit === null}
        onclick={() => void copySelectedPath()}
      >
        Copy File Path
      </button>
    </div>
    {#if actionError}
      <p class="action-error">{actionError}</p>
    {/if}
    <div
      bind:this={scrollerEl}
      class="scroller"
      style:height="{viewport}px"
      onscroll={onScroll}
      onkeydown={onScrollerKeydown}
      tabindex="0"
      role="region"
      aria-label="Search results"
    >
      {#if hits.length === 0 && !loading}
        <div class="placeholder">
          No results yet. Add folders in Settings, run a scan from Activity, then search here.
        </div>
      {:else}
        <div class="phantom" style:height="{hits.length * rowHeight}px">
          <div
            class="rows"
            style:transform="translateY({visible.start * rowHeight}px)"
          >
            {#each visible.slice as h, i (h.id)}
              <div
                class="row"
                class:selected={selectedIndex === visible.start + i}
                style:height="{rowHeight}px"
                onclick={() => onRowClick(visible.start + i)}
                ondblclick={() => onRowDblClick(h)}
                oncontextmenu={(e) => onRowContextMenu(e, visible.start + i, h)}
                role="row"
                aria-selected={selectedIndex === visible.start + i}
              >
                <div class="cell path" title={h.fullPath}>{h.fullPath}</div>
                <div class="cell size">{formatSize(h.size)}</div>
                <div class="cell time">{formatTime(h.mtimeNs)}</div>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </div>
  </div>

  {#if ctxMenu}
    <div
      bind:this={ctxMenuEl}
      class="ctx-menu"
      style:top="{ctxMenu.y}px"
      style:left="{ctxMenu.x}px"
      role="menu"
    >
      <button
        type="button"
        class="ctx-item"
        onclick={() => void openPath(fullPath(ctxMenu!.hit))}
      >
        Open File
      </button>
      <button
        type="button"
        class="ctx-item"
        onclick={() => void revealPath(fullPath(ctxMenu!.hit))}
      >
        Open Folder
      </button>
      <button
        type="button"
        class="ctx-item"
        onclick={() => void copyPathString(fullPath(ctxMenu!.hit))}
      >
        Copy File Path
      </button>
    </div>
  {/if}

  <div class="footer">
    <span class="meta">{totalLoaded} results loaded</span>
    {#if loading}<span class="meta">Loading…</span>{/if}
  </div>
</div>

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-height: 0;
    flex: 1;
    position: relative;
  }

  .header h1 {
    margin: 0 0 8px;
    font-size: 22px;
    font-weight: 600;
  }

  .lede {
    margin: 0;
    color: var(--text-muted);
    max-width: 640px;
    line-height: 1.5;
  }

  .search-bar {
    display: flex;
    gap: 10px;
    align-items: center;
  }

  .search-bar input {
    flex: 1;
    padding: 12px 14px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--bg-elevated);
    font-size: 15px;
  }

  .primary {
    border: none;
    border-radius: var(--radius);
    padding: 12px 20px;
    background: var(--accent);
    color: #0b1020;
    font-weight: 600;
    font-size: 14px;
  }

  .frame {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-elevated);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    min-height: 200px;
  }

  .col-head {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 100px 180px;
    gap: 8px;
    padding: 10px 12px;
    background: #1c2230;
    border-bottom: 1px solid var(--border);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    padding: 10px 12px;
    border-bottom: 1px solid var(--border);
    background: rgba(0, 0, 0, 0.15);
  }

  .secondary {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 8px 14px;
    background: var(--bg);
    color: var(--text);
    font-size: 13px;
    font-weight: 500;
  }

  .secondary:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .secondary:not(:disabled):hover {
    background: rgba(255, 255, 255, 0.06);
  }

  .action-error {
    margin: 0;
    padding: 0 12px 8px;
    font-size: 13px;
    color: var(--danger);
  }

  .scroller {
    overflow: auto;
    position: relative;
    outline: none;
  }

  .scroller:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }

  .placeholder {
    padding: 24px;
    color: var(--text-muted);
    font-size: 14px;
  }

  .phantom {
    position: relative;
  }

  .rows {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    display: flex;
    flex-direction: column;
  }

  .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 100px 180px;
    gap: 8px;
    align-items: center;
    padding: 0 12px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
    box-sizing: border-box;
    cursor: default;
  }

  .row.selected {
    background: rgba(79, 140, 255, 0.15);
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .cell {
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ctx-menu {
    position: fixed;
    z-index: 1000;
    min-width: 180px;
    padding: 4px 0;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--bg-elevated);
    box-shadow: var(--shadow);
  }

  .ctx-item {
    display: block;
    width: 100%;
    text-align: left;
    border: none;
    background: transparent;
    color: var(--text);
    font-size: 13px;
    padding: 8px 14px;
    cursor: default;
  }

  .ctx-item:hover {
    background: rgba(255, 255, 255, 0.06);
  }

  .footer {
    display: flex;
    gap: 12px;
    font-size: 12px;
    color: var(--text-muted);
  }

  .error {
    color: var(--danger);
    margin: 0;
  }
</style>
