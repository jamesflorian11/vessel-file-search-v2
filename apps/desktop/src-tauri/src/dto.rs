use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RootConfig {
    pub path: String,
    pub display_name: Option<String>,
    pub enabled: bool,
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
}

pub(crate) fn default_vessel_name() -> String {
    "Vessel".to_string()
}

pub(crate) fn default_onboarding_completed_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobProgress {
    pub phase: String,
    pub files_seen: u64,
    pub files_upserted: u64,
    pub current_path: Option<String>,
    pub roots_total: u32,
    pub roots_done: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobRecord {
    pub id: String,
    pub job_type: String,
    pub status: String,
    pub progress: Option<JobProgress>,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
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
