<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import {
    cancelJob,
    getIndexingStatus,
    startScan,
    type JobProgress,
    type IndexingStatus,
  } from "$lib/tauri";
  import { formatLastScanTime } from "$lib/scanLifecycle";

  let { embedded = false }: { embedded?: boolean } = $props();

  let status = $state<IndexingStatus | null>(null);
  let busy = $state(false);
  let error = $state<string | null>(null);

  const uiLabel = $derived.by(() => {
    const s = status;
    if (!s) return "—";
    if (s.state === "scanning") return "Scanning";
    if (s.state === "error") return "Error";
    return "Idle";
  });

  const detailLine = $derived.by(() => {
    const s = status;
    if (!s) return "";
    if (s.state === "scanning") {
      const p = s.progress;
      const seen = p?.filesSeen ?? 0;
      const phase = p?.phase?.trim() || "";
      const textOn =
        p?.contentIndexingEnabled ?? s.contentIndexingEnabled ?? false;
      const suffix = textOn ? " · text indexing on" : "";
      if (phase === "queued") return `Queued…${suffix}`;
      return `Scanning… ${seen.toLocaleString()} files seen${suffix}`;
    }
    if (s.state === "error") {
      return s.lastError?.trim() || "Last scan failed.";
    }
    if (s.lastScanAt) {
      return `Last scan: ${formatLastScanTime(s.lastScanAt)} · ${s.filesIndexed.toLocaleString()} files in index`;
    }
    return "No scan completed yet.";
  });

  async function refresh() {
    try {
      status = await getIndexingStatus();
      error = null;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  onMount(() => {
    void refresh();
    let unlistenProgress: (() => void) | undefined;
    let unlistenTerminal: (() => void) | undefined;
    void listen<{ jobId: string; progress: JobProgress }>("job_progress", () => {
      void refresh();
    }).then((fn) => {
      unlistenProgress = fn;
    });
    void listen("job_terminal", () => {
      void refresh();
    }).then((fn) => {
      unlistenTerminal = fn;
    });
    return () => {
      unlistenProgress?.();
      unlistenTerminal?.();
    };
  });

  async function onStartScan() {
    busy = true;
    error = null;
    try {
      await startScan();
      await refresh();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  async function onCancel() {
    const id = status?.activeJobId;
    if (!id) return;
    try {
      await cancelJob(id);
      await refresh();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }
</script>

<div class="panel" class:embedded>
  {#if !embedded}
    <header class="head">
      <div>
        <h2 class="title">Indexing status</h2>
        <p class="hint">Background index refresh. Start a scan from Search when you need to update.</p>
      </div>
      <button
        type="button"
        class="primary"
        disabled={busy || status?.state === "scanning"}
        onclick={() => void onStartScan()}
      >
        {#if busy}
          Starting…
        {:else if status?.state === "scanning"}
          Scanning…
        {:else}
          Start scan
        {/if}
      </button>
    </header>
  {/if}

  {#if error}
    <p class="err">{error}</p>
  {/if}

  <div class="status-block" aria-live="polite">
    <div class="row">
      <span class="label">State</span>
      <span class="value" data-state={status?.state ?? "unknown"}>{uiLabel}</span>
    </div>
    {#if detailLine}
      <p class="detail">{detailLine}</p>
    {/if}
    {#if status?.state === "scanning" && status.activeJobId}
      <button type="button" class="ghost" onclick={() => void onCancel()}>Cancel</button>
    {/if}
  </div>
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 14px;
    min-height: 0;
  }

  .panel.embedded {
    gap: 10px;
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
  }

  .title {
    margin: 0 0 6px;
    font-size: 15px;
    font-weight: 600;
  }

  .hint {
    margin: 0;
    font-size: 13px;
    color: var(--text-muted);
    line-height: 1.45;
    max-width: 520px;
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

  .status-block {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px 16px;
    background: var(--bg);
  }

  .row {
    display: flex;
    align-items: baseline;
    gap: 12px;
    flex-wrap: wrap;
  }

  .label {
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }

  .value {
    font-size: 15px;
    font-weight: 600;
  }

  .value[data-state="scanning"] {
    color: var(--accent);
  }

  .value[data-state="error"] {
    color: var(--danger);
  }

  .detail {
    margin: 10px 0 0;
    font-size: 13px;
    color: var(--text-muted);
    line-height: 1.45;
  }

  .ghost {
    margin-top: 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 6px 12px;
    background: transparent;
    color: var(--text);
    font-size: 13px;
  }

  .err {
    color: var(--danger);
    margin: 0;
    font-size: 13px;
  }
</style>
