import { writable } from "svelte/store";
import type { JobRecord } from "$lib/tauri";

export type ScanPhase = "idle" | "scanning" | "completed" | "failed";

export const scanPhase = writable<ScanPhase>("idle");
export const scanLiveFilesSeen = writable(0);
export const scanTerminalMessage = writable<string | null>(null);
export const lastScanSummary = writable<{
  filesIndexed: number;
  completedAtIso: string;
} | null>(null);

let trackedActiveJobId: string | null = null;

function applyLastCompletedFromHistory(jobs: JobRecord[]) {
  const lastCompleted = jobs.find(
    (j) => j.jobType === "scan" && j.status === "completed",
  );
  if (lastCompleted) {
    const p = lastCompleted.progress;
    lastScanSummary.set({
      filesIndexed: p?.filesSeen ?? 0,
      completedAtIso: lastCompleted.updatedAt,
    });
  }
}

/**
 * Derive scan phase and last-scan info from the jobs list (source of truth after refresh).
 */
export function syncScanFromJobs(jobs: JobRecord[]) {
  const active = jobs.find(
    (j) =>
      j.jobType === "scan" &&
      (j.status === "queued" || j.status === "running"),
  );

  if (active) {
    trackedActiveJobId = active.id;
    scanPhase.set("scanning");
    scanTerminalMessage.set(null);
    scanLiveFilesSeen.set(active.progress?.filesSeen ?? 0);
    return;
  }

  if (trackedActiveJobId) {
    const ended = jobs.find((j) => j.id === trackedActiveJobId);
    trackedActiveJobId = null;
    scanLiveFilesSeen.set(0);

    if (!ended) {
      applyLastCompletedFromHistory(jobs);
      scanPhase.set("idle");
      scanTerminalMessage.set(null);
      return;
    }

    if (ended.status === "completed") {
      const p = ended.progress;
      const files = p?.filesSeen ?? 0;
      lastScanSummary.set({
        filesIndexed: files,
        completedAtIso: ended.updatedAt,
      });
      scanPhase.set("completed");
      scanTerminalMessage.set(null);
      return;
    }
    if (ended.status === "failed") {
      scanPhase.set("failed");
      scanTerminalMessage.set(ended.error?.trim() || "Scan failed.");
    } else {
      scanPhase.set("idle");
      scanTerminalMessage.set(null);
    }
    applyLastCompletedFromHistory(jobs);
    return;
  }

  applyLastCompletedFromHistory(jobs);
  scanPhase.set("idle");
  scanTerminalMessage.set(null);
}

export function formatLastScanTime(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString();
}
