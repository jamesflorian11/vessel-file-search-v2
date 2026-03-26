import { invoke } from "@tauri-apps/api/core";

export type RootConfig = {
  path: string;
  displayName: string | null;
  enabled: boolean;
};

export type AppSettings = {
  roots: RootConfig[];
  exclusionGlobs: string[];
  batchSize: number;
  vesselName: string;
  onboardingCompleted: boolean;
  /** `"light"` | `"dark"` */
  theme: string;
};

export type JobProgress = {
  phase: string;
  filesSeen: number;
  filesUpserted: number;
  filesDeleted?: number;
  currentPath: string | null;
  rootsTotal: number;
  rootsDone: number;
};

export type JobRecord = {
  id: string;
  jobType: string;
  status: string;
  progress: JobProgress | null;
  error: string | null;
  createdAt: string;
  updatedAt: string;
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

export async function listJobs(): Promise<JobRecord[]> {
  return invoke("list_jobs");
}

export async function clearJobHistory(): Promise<number> {
  return invoke("clear_job_history");
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
  const payload = { path };
  // Temporary debug: remove after verifying Open Folder on Windows
  console.log("[Vessel debug] revealInExplorer invoke payload:", JSON.stringify(payload));
  return invoke("reveal_in_explorer", payload);
}
