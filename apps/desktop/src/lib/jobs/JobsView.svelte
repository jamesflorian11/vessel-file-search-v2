<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import {
    cancelJob,
    clearJobHistory,
    listJobs,
    startScan,
    type JobRecord,
    type JobProgress,
  } from "$lib/tauri";
  import { throttle } from "$lib/throttle";
  import { syncScanFromJobs, scanPhase, lastScanSummary } from "$lib/scanLifecycle";

  let { embedded = false }: { embedded?: boolean } = $props();

  let jobs = $state<JobRecord[]>([]);
  let busy = $state(false);
  let error = $state<string | null>(null);

  const activeJobs = $derived(
    jobs.filter(
      (j) =>
        j.jobType === "scan" &&
        (j.status === "queued" || j.status === "running"),
    ),
  );

  const historyJobs = $derived(
    jobs.filter(
      (j) =>
        !(
          j.jobType === "scan" &&
          (j.status === "queued" || j.status === "running")
        ),
    ),
  );

  const hasActiveScan = $derived(activeJobs.length > 0);

  const progressById = new Map<string, JobProgress>();

  async function refresh() {
    try {
      jobs = await listJobs();
      for (const j of jobs) {
        if (j.progress) progressById.set(j.id, j.progress);
      }
      syncScanFromJobs(jobs);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  const applyProgressThrottled = throttle((jobId: string, p: JobProgress) => {
    progressById.set(jobId, p);
    jobs = jobs.map((j) =>
      j.id === jobId ? { ...j, progress: { ...p } } : j,
    );
  }, 100);

  onMount(() => {
    void refresh();
    let unlisten: (() => void) | undefined;
    let unlistenTerminal: (() => void) | undefined;
    void listen<{ jobId: string; progress: JobProgress }>(
      "job_progress",
      (ev) => {
        applyProgressThrottled(ev.payload.jobId, ev.payload.progress);
      },
    ).then((fn) => {
      unlisten = fn;
    });
    void listen("job_terminal", () => {
      void refresh();
    }).then((fn) => {
      unlistenTerminal = fn;
    });
    return () => {
      unlisten?.();
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

  async function onCancel(id: string) {
    try {
      await cancelJob(id);
      await refresh();
      setTimeout(() => void refresh(), 200);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function onClearHistory() {
    if (
      !confirm(
        "Remove all completed, failed, and cancelled jobs from the list?",
      )
    ) {
      return;
    }
    try {
      await clearJobHistory();
      await refresh();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  function formatJobStatus(status: string): string {
    switch (status) {
      case "completed":
        return "Completed";
      case "failed":
        return "Failed";
      case "cancelled":
        return "Cancelled";
      case "running":
        return "Running";
      case "queued":
        return "Queued";
      default:
        return status;
    }
  }

  function progressLabel(j: JobRecord): string {
    const p = j.progress ?? progressById.get(j.id);
    if (!p) return formatJobStatus(j.status);
    const seen = p.filesSeen.toLocaleString();
    const updated = p.filesUpserted.toLocaleString();
    const del =
      p.filesDeleted != null && p.filesDeleted > 0
        ? ` · ${p.filesDeleted.toLocaleString()} removed from index`
        : "";
    if (j.status === "running" || j.status === "queued") {
      return `${seen} files checked so far · ${updated} added or updated in the index${del}`;
    }
    return `${seen} files checked · ${updated} added or updated in the index${del}`;
  }
</script>

<div class="view" class:embedded>
  {#if !embedded}
    <header class="header">
      <div>
        <h1>Activity</h1>
        <p class="lede">
          Start a scan to update your index, or review past runs below. The window stays responsive while
          work is in progress.
        </p>
        {#if hasActiveScan}
          <p class="scan-hint">A scan is already queued or running.</p>
        {/if}
      </div>
      <button
        type="button"
        class="primary"
        disabled={busy || $scanPhase === "scanning"}
        onclick={() => void onStartScan()}
      >
        {#if busy}
          Starting…
        {:else if $scanPhase === "scanning"}
          Scanning…
        {:else if $lastScanSummary}
          Rescan
        {:else}
          Start scan
        {/if}
      </button>
    </header>
  {:else if hasActiveScan}
    <p class="embed-hint">A scan is already queued or running.</p>
  {/if}

  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#if jobs.length === 0}
    <div class="empty">
      {#if embedded}
        No jobs yet. Start a scan from the Search screen.
      {:else}
        No jobs yet. Start a scan to index your roots.
      {/if}
    </div>
  {:else}
    {#if activeJobs.length > 0}
      <h2 class="section-heading">In progress</h2>
      <ul class="jobs">
        {#each activeJobs as j}
          <li class="job">
            <div class="job-top">
              <span class="job-id">{j.id.slice(0, 8)}…</span>
              <span class="badge" data-status={j.status}>{j.status}</span>
              {#if j.status === "running" || j.status === "queued"}
                <button type="button" class="ghost" onclick={() => void onCancel(j.id)}>Cancel</button>
              {/if}
            </div>
            <div class="job-type">{j.jobType}</div>
            <div class="job-progress">{progressLabel(j)}</div>
            {#if j.error}
              <pre class="job-error">{j.error}</pre>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}

    {#if historyJobs.length > 0}
      <div class="history-head">
        <h2 class="section-heading">Past runs</h2>
        <button type="button" class="secondary small" onclick={() => void onClearHistory()}>
          Clear history
        </button>
      </div>
      <ul class="jobs">
        {#each historyJobs as j}
          <li class="job">
            <div class="job-top">
              <span class="job-id">{j.id.slice(0, 8)}…</span>
              <span class="badge" data-status={j.status}>{j.status}</span>
            </div>
            <div class="job-type">{j.jobType}</div>
            <div class="job-progress">{progressLabel(j)}</div>
            {#if j.error}
              <pre class="job-error">{j.error}</pre>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</div>

<style>
  .view {
    display: flex;
    flex-direction: column;
    gap: 20px;
    min-height: 0;
    overflow: auto;
  }

  .view.embedded {
    gap: 14px;
  }

  .embed-hint {
    margin: 0;
    font-size: 13px;
    color: var(--text-muted);
  }

  .header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
  }

  .header h1 {
    margin: 0 0 8px;
    font-size: 22px;
    font-weight: 600;
  }

  .lede {
    margin: 0;
    color: var(--text-muted);
    max-width: 520px;
    line-height: 1.5;
  }

  .scan-hint {
    margin: 8px 0 0;
    font-size: 13px;
    color: var(--text-muted);
  }

  .section-heading {
    margin: 0 0 10px;
    font-size: 15px;
    font-weight: 600;
    color: var(--text);
    letter-spacing: 0.01em;
  }

  .history-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-top: 8px;
    margin-bottom: 10px;
    flex-wrap: wrap;
  }

  .history-head .section-heading {
    margin: 0;
  }

  .secondary.small {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 6px 12px;
    background: transparent;
    color: var(--text);
    font-size: 13px;
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

  .ghost {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 6px 10px;
    background: transparent;
    color: var(--text);
    font-size: 12px;
    margin-left: auto;
  }

  .empty {
    border: 1px dashed var(--border);
    border-radius: var(--radius);
    padding: 20px;
    color: var(--text-muted);
  }

  .jobs {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .job {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px 16px;
    background: var(--bg-elevated);
  }

  .job-top {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 6px;
  }

  .job-id {
    font-family: ui-monospace, monospace;
    font-size: 12px;
    color: var(--text-muted);
  }

  .badge {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    padding: 4px 8px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-muted);
  }

  .badge[data-status="running"] {
    color: var(--accent);
    background: rgba(79, 140, 255, 0.12);
  }

  .badge[data-status="failed"] {
    color: var(--danger);
    background: rgba(240, 113, 120, 0.12);
  }

  .badge[data-status="completed"] {
    color: var(--success);
    background: rgba(127, 216, 143, 0.12);
  }

  .badge[data-status="cancelled"] {
    color: var(--text-muted);
    background: rgba(255, 255, 255, 0.06);
  }

  .job-type {
    font-size: 13px;
    font-weight: 500;
    margin-bottom: 4px;
  }

  .job-progress {
    font-size: 12px;
    color: var(--text-muted);
  }

  .job-error {
    margin: 10px 0 0;
    padding: 10px;
    background: rgba(240, 113, 120, 0.08);
    border-radius: 8px;
    font-size: 12px;
    white-space: pre-wrap;
    color: var(--danger);
  }

  .error {
    color: var(--danger);
    margin: 0;
  }
</style>
