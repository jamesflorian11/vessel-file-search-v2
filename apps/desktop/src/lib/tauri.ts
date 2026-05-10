import { invoke } from "@tauri-apps/api/core";

export type RootConfig = {
  path: string;
  displayName: string | null;
  enabled: boolean;
  /** When true, future create/upload/delete actions should be blocked for this root (Phase 2). */
  readOnly: boolean;
};

export type AppSettings = {
  roots: RootConfig[];
  exclusionGlobs: string[];
  batchSize: number;
  vesselName: string;
  onboardingCompleted: boolean;
  /** `"light"` | `"dark"` */
  theme: string;
  /** When true, scans extract searchable text from supported file types (default off). */
  contentIndexingEnabled: boolean;
  /** If empty, built-in extensions apply. Otherwise only these (e.g. pdf, txt). */
  contentIndexExtensions: string[];
  /** Max bytes read per file for text extraction (256 KiB–100 MiB). */
  contentMaxBytesPerFile: number;
};

export type JobProgress = {
  phase: string;
  filesSeen: number;
  filesUpserted: number;
  filesDeleted?: number;
  currentPath: string | null;
  rootsTotal: number;
  rootsDone: number;
  contentIndexingEnabled?: boolean;
};

export type IndexingStatus = {
  state: string;
  progress: JobProgress | null;
  lastScanAt: string | null;
  lastScanStatus: string;
  lastError: string | null;
  filesIndexed: number;
  activeJobId: string | null;
  contentIndexingEnabled: boolean;
};

export type SearchHit = {
  id: number;
  path: string;
  fullPath: string;
  size: number;
  mtimeNs: number;
  rootPath: string;
};

export type SearchFilesOptions = {
  extensionFilter?: string | null;
  modifiedFromNs?: number | null;
  modifiedToNs?: number | null;
};

export async function getSettings(): Promise<AppSettings> {
  return invoke("get_settings");
}

export async function saveSettings(settings: AppSettings): Promise<void> {
  return invoke("save_settings", { settings });
}

export async function startScan(): Promise<string> {
  return invoke("start_scan");
}

export async function cancelJob(jobId: string): Promise<void> {
  return invoke("cancel_job", { jobId });
}

export async function getIndexingStatus(): Promise<IndexingStatus> {
  return invoke("get_indexing_status");
}

export async function searchFiles(
  query: string,
  limit: number,
  offset: number,
  options?: SearchFilesOptions | null,
): Promise<SearchHit[]> {
  return invoke("search_files", {
    query,
    limit,
    offset,
    extensionFilter: options?.extensionFilter ?? null,
    modifiedFromNs: options?.modifiedFromNs ?? null,
    modifiedToNs: options?.modifiedToNs ?? null,
  });
}

export async function openFile(path: string): Promise<void> {
  return invoke("open_file", { path });
}

export async function revealInExplorer(path: string): Promise<void> {
  return invoke("reveal_in_explorer", { path });
}
