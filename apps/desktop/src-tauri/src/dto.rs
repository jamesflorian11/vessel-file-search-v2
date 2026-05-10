use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RootConfig {
    pub path: String,
    pub display_name: Option<String>,
    pub enabled: bool,
    /// When true, Phase 2+ file actions should not create/upload/delete under this root (user intent).
    #[serde(default = "default_root_read_only")]
    pub read_only: bool,
}

pub(crate) fn default_root_read_only() -> bool {
    false
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub roots: Vec<RootConfig>,
    pub exclusion_globs: Vec<String>,
    pub batch_size: usize,
    /// Display name for this install (sidebar branding).
    #[serde(default = "default_vessel_name")]
    pub vessel_name: String,
    /// When false, the app shows first-run setup. Missing in old configs defaults to true.
    #[serde(default = "default_onboarding_completed_true")]
    pub onboarding_completed: bool,
    /// UI theme: `light` or `dark`.
    #[serde(default = "default_theme")]
    pub theme: String,
    /// When true, scans read text from supported file types into the index (off by default for safety).
    #[serde(default = "default_content_indexing_enabled")]
    pub content_indexing_enabled: bool,
    /// If empty, built-in defaults apply (pdf, txt, md, csv, json, log). Otherwise only these extensions (lowercase, with or without leading dot).
    #[serde(default)]
    pub content_index_extensions: Vec<String>,
    /// Max bytes read from disk per file for body extraction (caps memory and IO).
    #[serde(default = "default_content_max_bytes_per_file")]
    pub content_max_bytes_per_file: u32,
}

pub(crate) fn default_vessel_name() -> String {
    "Vessel".to_string()
}

pub(crate) fn default_onboarding_completed_true() -> bool {
    true
}

pub(crate) fn default_theme() -> String {
    "dark".to_string()
}

pub(crate) fn default_content_indexing_enabled() -> bool {
    false
}

pub(crate) fn default_content_max_bytes_per_file() -> u32 {
    10 * 1024 * 1024
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobProgress {
    pub phase: String,
    pub files_seen: u64,
    pub files_upserted: u64,
    /// Files removed from the index during a scan (paths no longer on disk).
    #[serde(default)]
    pub files_deleted: u64,
    pub current_path: Option<String>,
    pub roots_total: u32,
    pub roots_done: u32,
    /// Mirrors scan config: whether the active run is indexing file bodies.
    #[serde(default)]
    pub content_indexing_enabled: bool,
}

/// Current indexing UI state: idle, scanning, or error (last scan failed).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexingStatus {
    pub state: String,
    pub progress: Option<JobProgress>,
    pub last_scan_at: Option<String>,
    /// Last terminal outcome: `idle`, `completed`, `failed`, or `cancelled`.
    pub last_scan_status: String,
    pub last_error: Option<String>,
    /// Live count of indexed file rows (`file_state = present`).
    pub files_indexed: i64,
    pub active_job_id: Option<String>,
    /// From config: whether content (body) indexing is enabled for the next scan / search.
    #[serde(default)]
    pub content_indexing_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub id: i64,
    pub path: String,
    /// Absolute path to the file (normalized separators for the OS).
    pub full_path: String,
    pub size: i64,
    pub mtime_ns: i64,
    pub root_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobProgressEvent {
    pub job_id: String,
    pub progress: JobProgress,
}

/// Emitted when a job reaches a terminal status (completed, failed, cancelled).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobTerminalEvent {
    pub job_id: String,
    pub status: String,
    pub progress: Option<JobProgress>,
}
