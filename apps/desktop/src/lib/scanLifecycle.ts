import { writable } from "svelte/store";
import type { IndexingStatus } from "$lib/tauri";

export type ScanPhase = "idle" | "scanning" | "completed" | "failed";

export const scanPhase = writable<ScanPhase>("idle");
export const scanLiveFilesSeen = writable(0);
export const scanTerminalMessage = writable<string | null>(null);
export const lastScanSummary = writable<{
  filesIndexed: number;
  completedAtIso: string;
} | null>(null);

export const indexingStatus = writable<IndexingStatus | null>(null);

/**
 * Maps `get_indexing_status` into sidebar/search scan UX state (no job history).
 */
export function syncFromIndexingStatus(s: IndexingStatus) {
  indexingStatus.set(s);

  if (s.state === "scanning") {
    scanPhase.set("scanning");
    scanTerminalMessage.set(null);
    scanLiveFilesSeen.set(s.progress?.filesSeen ?? 0);
    return;
  }

  scanLiveFilesSeen.set(0);

  if (s.state === "error") {
    scanPhase.set("failed");
    scanTerminalMessage.set(s.lastError?.trim() || "Scan failed.");
    lastScanSummary.set(null);
    return;
  }

  scanTerminalMessage.set(null);

  if (s.lastScanStatus === "completed" && s.lastScanAt) {
    scanPhase.set("completed");
    lastScanSummary.set({
      filesIndexed: s.filesIndexed,
      completedAtIso: s.lastScanAt,
    });
    return;
  }

  scanPhase.set("idle");
  lastScanSummary.set(null);
}

export function formatLastScanTime(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString();
}
