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
};

export type JobProgress = {
  phase: string;
  filesSeen: number;
  filesUpserted: number;
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
): Promise<SearchHit[]> {
  return invoke("search_files", { query, limit, offset });
}

export async function openFile(path: string): Promise<void> {
  return invoke("open_file", { path });
}

export async function revealInExplorer(path: string): Promise<void> {
  return invoke("reveal_in_explorer", { path });
}
