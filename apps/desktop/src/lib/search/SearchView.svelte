<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import {
    openFile,
    revealInExplorer,
    searchFiles,
    startScan,
    listJobs,
    type SearchHit,
    type JobProgress,
  } from "$lib/tauri";
  import {
    scanPhase,
    scanLiveFilesSeen,
    lastScanSummary,
    scanTerminalMessage,
    syncScanFromJobs,
    formatLastScanTime,
  } from "$lib/scanLifecycle";

  let query = $state("");
  let extensionFilter = $state("");
  let modifiedFrom = $state("");
  let modifiedTo = $state("");
  let hits = $state<SearchHit[]>([]);
  let totalLoaded = $state(0);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let actionError = $state<string | null>(null);
  let exhausted = $state(false);
  let selectedIndex = $state<number | null>(null);

  let scanBusy = $state(false);
  let scanError = $state<string | null>(null);

  let latestSearchId = 0;
  let paginationInFlight = false;
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  const rowHeight = 44;
  let scrollerClientHeight = $state(0);
  let scrollTop = $state(0);
  let scrollerEl = $state<HTMLDivElement | undefined>(undefined);
  let searchInputEl = $state<HTMLInputElement | undefined>(undefined);
  let ctxMenu = $state<{ x: number; y: number; hit: SearchHit } | null>(null);
  let ctxMenuEl = $state<HTMLDivElement | undefined>(undefined);

  const pageSize = 200;

  function dateStartNs(iso: string): number | null {
    const t = iso.trim();
    if (!t) return null;
    const d = new Date(`${t}T00:00:00`);
    if (Number.isNaN(d.getTime())) return null;
    return d.getTime() * 1_000_000;
  }

  function dateEndNs(iso: string): number | null {
    const t = iso.trim();
    if (!t) return null;
    const d = new Date(`${t}T23:59:59.999`);
    if (Number.isNaN(d.getTime())) return null;
    return d.getTime() * 1_000_000;
  }

  function searchOptions() {
    const ext = extensionFilter.trim();
    return {
      extensionFilter: ext ? ext : null,
      modifiedFromNs: modifiedFrom ? dateStartNs(modifiedFrom) : null,
      modifiedToNs: modifiedTo ? dateEndNs(modifiedTo) : null,
    };
  }

  function fullPath(h: SearchHit): string {
    return h.fullPath;
  }

  let viewportHeight = $derived.by(() => Math.max(120, scrollerClientHeight || 0));

  function viewportRows(): number {
    return Math.max(1, Math.floor(viewportHeight / rowHeight));
  }

  function formatActionError(raw: string): string {
    const s = raw.trim();
    if (/clipboard|not allowed|permission|denied/i.test(s)) {
      return "Could not copy to the clipboard. Try again or check that the app can access the clipboard.";
    }
    return s;
  }

  let selectedHit = $derived.by(() => {
    if (selectedIndex === null) return null;
    if (selectedIndex < 0 || selectedIndex >= hits.length) return null;
    return hits[selectedIndex];
  });

  async function refreshJobs() {
    try {
      const j = await listJobs();
      syncScanFromJobs(j);
    } catch {
      // keep previous
    }
  }

  async function runSearch(reset: boolean) {
    if (reset) {
      latestSearchId++;
      paginationInFlight = false;
    } else if (paginationInFlight) {
      return;
    }
    const id = latestSearchId;
    loading = true;
    error = null;
    actionError = null;
    if (reset) exhausted = false;
    const offset = reset ? 0 : hits.length;
    if (!reset) paginationInFlight = true;
    try {
      const batch = await searchFiles(
        query,
        pageSize,
        offset,
        searchOptions(),
      );
      if (id !== latestSearchId) return;
      if (batch.length < pageSize) exhausted = true;
      hits = reset ? batch : [...hits, ...batch];
      totalLoaded = hits.length;
      if (reset) selectedIndex = null;
    } catch (e) {
      if (id !== latestSearchId) return;
      error = e instanceof Error ? e.message : String(e);
    } finally {
      if (!reset) paginationInFlight = false;
      if (id === latestSearchId) loading = false;
    }
  }

  function scheduleDebouncedSearch() {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      debounceTimer = null;
      scrollTop = 0;
      if (scrollerEl) scrollerEl.scrollTop = 0;
      void runSearch(true);
    }, 250);
  }

  function onSubmit(e: Event) {
    e.preventDefault();
    if (debounceTimer) {
      clearTimeout(debounceTimer);
      debounceTimer = null;
    }
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
    const vh = viewportHeight;
    const start = Math.max(0, Math.floor(scrollTop / rowHeight) - overscan);
    const count = Math.ceil(vh / rowHeight) + overscan * 2;
    const slice = hits.slice(start, start + count);
    return { start, slice };
  });

  async function onScroll(e: Event) {
    const el = e.currentTarget as HTMLDivElement;
    scrollTop = el.scrollTop;
    const nearBottom =
      el.scrollTop + el.clientHeight > el.scrollHeight - rowHeight * 10;
    if (
      nearBottom &&
      !loading &&
      !exhausted &&
      hits.length > 0
    ) {
      const id = latestSearchId;
      await runSearch(false);
      if (id !== latestSearchId) return;
    }
  }

  function scrollRowIntoView(index: number) {
    const el = scrollerEl;
    if (!el || index < 0 || index >= hits.length) return;
    const rowTop = index * rowHeight;
    const rowBottom = rowTop + rowHeight;
    const viewTop = el.scrollTop;
    const ch = el.clientHeight;
    const viewBottom = viewTop + ch;
    if (rowTop < viewTop) {
      el.scrollTop = rowTop;
    } else if (rowBottom > viewBottom) {
      el.scrollTop = rowBottom - ch;
    }
    scrollTop = el.scrollTop;
  }

  async function openPath(path: string) {
    actionError = null;
    closeCtxMenu();
    try {
      await openFile(path);
    } catch (e) {
      actionError = formatActionError(
        e instanceof Error ? e.message : String(e),
      );
    }
  }

  async function revealPath(path: string) {
    actionError = null;
    closeCtxMenu();
    // Temporary debug: remove after verifying Open Folder on Windows
    console.log("[Vessel debug] Open Folder click", {
      pathArg: path,
      selectedIndex,
      selectedHitFullPath: selectedHit?.fullPath,
      selectedHitId: selectedHit?.id,
    });
    try {
      await revealInExplorer(path);
    } catch (e) {
      actionError = formatActionError(
        e instanceof Error ? e.message : String(e),
      );
    }
  }

  async function copyPathString(path: string) {
    actionError = null;
    closeCtxMenu();
    try {
      await navigator.clipboard.writeText(path);
    } catch (e) {
      actionError = formatActionError(
        e instanceof Error ? e.message : String(e),
      );
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

  function onOptionKeydown(e: KeyboardEvent, globalIndex: number) {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      onRowClick(globalIndex);
    }
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
    if (e.key === "Home") {
      e.preventDefault();
      if (hits.length === 0) return;
      selectedIndex = 0;
      scrollRowIntoView(0);
      scrollerEl?.focus();
      return;
    }
    if (e.key === "End") {
      e.preventDefault();
      if (hits.length === 0) return;
      const last = hits.length - 1;
      selectedIndex = last;
      scrollRowIntoView(last);
      scrollerEl?.focus();
      return;
    }
    if (e.key === "PageDown") {
      e.preventDefault();
      moveSelection(viewportRows());
      return;
    }
    if (e.key === "PageUp") {
      e.preventDefault();
      moveSelection(-viewportRows());
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
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      if (debounceTimer) {
        clearTimeout(debounceTimer);
        debounceTimer = null;
      }
      scrollTop = 0;
      if (scrollerEl) scrollerEl.scrollTop = 0;
      void runSearch(true);
    }
  }

  async function onStartScan() {
    scanBusy = true;
    scanError = null;
    try {
      await startScan();
      await refreshJobs();
    } catch (e) {
      scanError = e instanceof Error ? e.message : String(e);
    } finally {
      scanBusy = false;
    }
  }

  onMount(() => {
    void refreshJobs();
    let unlistenProgress: (() => void) | undefined;
    let unlistenTerminal: (() => void) | undefined;
    void listen<{ jobId: string; progress: JobProgress }>(
      "job_progress",
      (ev) => {
        scanPhase.set("scanning");
        scanLiveFilesSeen.set(ev.payload.progress.filesSeen);
        void refreshJobs();
      },
    ).then((fn) => {
      unlistenProgress = fn;
    });
    void listen("job_terminal", () => {
      void refreshJobs();
    }).then((fn) => {
      unlistenTerminal = fn;
    });

    const onDocClick = (ev: MouseEvent) => {
      if (!ctxMenu) return;
      const t = ev.target as Node;
      if (ctxMenuEl?.contains(t)) return;
      closeCtxMenu();
    };
    document.addEventListener("click", onDocClick, true);
    return () => {
      document.removeEventListener("click", onDocClick, true);
      unlistenProgress?.();
      unlistenTerminal?.();
      if (debounceTimer) clearTimeout(debounceTimer);
    };
  });
</script>

<div class="view">
  <header class="header">
    <h1>Search</h1>
    <p class="lede">
      Find files by name or path across indexed folders. Results update as you type (short delay).
      Leave the search box empty and press Search to browse everything in the index (with optional
      filters). Large result sets load in pages as you scroll.
    </p>
  </header>

  <div class="scan-row" aria-label="Indexing" aria-live="polite">
    <div class="scan-main">
      {#if $scanPhase === "scanning"}
        <div class="scan-active">
          <span class="spinner" aria-hidden="true"></span>
          <div class="scan-copy">
            <span class="scan-line strong"
              >Scanning… {$scanLiveFilesSeen.toLocaleString()} files</span
            >
            <span class="scan-sub">Updating the index — you can keep using the app.</span>
          </div>
        </div>
      {:else if $scanPhase === "completed" && $lastScanSummary}
        <div class="scan-copy">
          <span class="scan-line success"
            >Scan complete — {$lastScanSummary.filesIndexed.toLocaleString()} files indexed</span
          >
          <span class="last-scan"
            >Last scan: {formatLastScanTime($lastScanSummary.completedAtIso)}</span
          >
        </div>
      {:else if $scanPhase === "failed"}
        <p class="scan-line error">{$scanTerminalMessage ?? "Scan failed."}</p>
      {:else}
        <p class="scan-line muted">
          {#if $lastScanSummary}
            Index is ready. Search below or run a rescan after large file changes.
          {:else}
            No index yet. Add folders in Settings, then run a scan.
          {/if}
        </p>
        {#if $lastScanSummary && $scanPhase === "idle"}
          <p class="last-scan">
            Last scan: {formatLastScanTime($lastScanSummary.completedAtIso)} — {$lastScanSummary.filesIndexed.toLocaleString()} files indexed
          </p>
        {/if}
      {/if}
    </div>
    <button
      type="button"
      class="primary scan-cta"
      disabled={scanBusy || $scanPhase === "scanning"}
      onclick={() => void onStartScan()}
    >
      {#if scanBusy}
        Starting…
      {:else if $scanPhase === "scanning"}
        Scanning…
      {:else if $lastScanSummary}
        Rescan
      {:else}
        Start scan
      {/if}
    </button>
  </div>
  {#if scanError}
    <p class="error">{scanError}</p>
  {/if}

  <form class="search-bar" onsubmit={onSubmit}>
    <input
      bind:this={searchInputEl}
      type="search"
      placeholder="Search by file or path (FTS: words are AND’d)"
      bind:value={query}
      autocomplete="off"
      oninput={scheduleDebouncedSearch}
      onkeydown={onSearchKeydown}
    />
    <button type="submit" class="primary">Search</button>
  </form>

  <div class="filters" aria-label="Filters">
    <label class="filter">
      <span class="filter-label">Extension</span>
      <input
        type="text"
        placeholder="e.g. pdf"
        bind:value={extensionFilter}
        oninput={scheduleDebouncedSearch}
      />
    </label>
    <label class="filter">
      <span class="filter-label">Modified from</span>
      <input type="date" bind:value={modifiedFrom} onchange={scheduleDebouncedSearch} />
    </label>
    <label class="filter">
      <span class="filter-label">Modified to</span>
      <input type="date" bind:value={modifiedTo} onchange={scheduleDebouncedSearch} />
    </label>
  </div>

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
      bind:clientHeight={scrollerClientHeight}
      class="scroller"
      onscroll={onScroll}
      onkeydown={onScrollerKeydown}
      tabindex="0"
      role="listbox"
      aria-label="Search results"
      aria-multiselectable="false"
    >
      {#if hits.length === 0 && !loading}
        <div class="placeholder">
          No results yet. Add folders in Settings, start a scan above, then type a query or browse
          with an empty search and optional filters.
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
                tabindex="-1"
                onclick={() => onRowClick(visible.start + i)}
                onkeydown={(e) => onOptionKeydown(e, visible.start + i)}
                ondblclick={() => onRowDblClick(h)}
                oncontextmenu={(e) =>
                  onRowContextMenu(e, visible.start + i, h)}
                role="option"
                aria-selected={selectedIndex === visible.start + i}
              >
                <span class="cell path" title={h.fullPath}>{h.fullPath}</span>
                <span class="cell size">{formatSize(h.size)}</span>
                <span class="cell time">{formatTime(h.mtimeNs)}</span>
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

  .scan-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 14px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--bg-elevated);
    transition:
      border-color 0.25s ease,
      background 0.25s ease;
  }

  .scan-main {
    flex: 1;
    min-width: 200px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .scan-active {
    display: flex;
    align-items: flex-start;
    gap: 12px;
  }

  .scan-copy {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .scan-line {
    margin: 0;
    font-size: 14px;
    line-height: 1.4;
  }

  .scan-line.strong {
    font-weight: 600;
  }

  .scan-line.muted {
    color: var(--text-muted);
  }

  .scan-line.success {
    color: var(--success);
    font-weight: 600;
  }

  .scan-line.error {
    color: var(--danger);
    margin: 0;
  }

  .scan-sub {
    font-size: 12px;
    color: var(--text-muted);
  }

  .last-scan {
    margin: 0;
    font-size: 12px;
    color: var(--text-muted);
  }

  @keyframes scan-spin {
    to {
      transform: rotate(360deg);
    }
  }

  .spinner {
    width: 22px;
    height: 22px;
    flex-shrink: 0;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: scan-spin 0.75s linear infinite;
  }

  .scan-cta {
    flex-shrink: 0;
  }

  .scan-cta:disabled {
    opacity: 0.65;
    cursor: not-allowed;
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

  .filters {
    display: flex;
    flex-wrap: wrap;
    gap: 12px 20px;
    align-items: flex-end;
  }

  .filter {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 140px;
  }

  .filter-label {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }

  .filter input[type="text"],
  .filter input[type="date"] {
    padding: 8px 10px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: var(--bg-elevated);
    font-size: 14px;
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

  .primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .frame {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-elevated);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
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
    flex: 1;
    min-height: 120px;
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
    display: block;
    min-width: 0;
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
