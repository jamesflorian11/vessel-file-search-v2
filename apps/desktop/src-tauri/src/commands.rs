use std::path::{Path, PathBuf};
use std::sync::Arc;

use globset::Glob;
use log::{info, warn};
use tauri::State;

use crate::config;
use crate::db;
use crate::dto::{AppSettings, JobRecord, JobProgress, SearchHit};
use crate::jobs::JobManager;
use crate::path_norm;
use crate::roots;
use crate::search;

pub struct AppState {
    pub db_path: PathBuf,
    pub config_path: PathBuf,
    pub jobs: Arc<JobManager>,
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    config::load(&state.config_path).map_err(|e| e.to_string())
}

fn validate_globs(patterns: &[String]) -> Result<(), String> {
    for p in patterns {
        Glob::new(p).map_err(|e| format!("Invalid glob '{p}': {e}"))?;
    }
    Ok(())
}

#[tauri::command]
pub fn save_settings(mut settings: AppSettings, state: State<'_, AppState>) -> Result<(), String> {
    path_norm::normalize_app_settings(&mut settings);
    if settings.vessel_name.trim().is_empty() {
        return Err("Vessel name cannot be empty.".into());
    }
    if settings.vessel_name.chars().count() > 120 {
        return Err("Vessel name is too long.".into());
    }
    if settings.batch_size < 200 || settings.batch_size > 20_000 {
        return Err("batch_size must be between 200 and 20000".into());
    }
    for r in &settings.roots {
        if r.path.trim().is_empty() {
            return Err("root paths cannot be empty".into());
        }
    }
    validate_globs(&settings.exclusion_globs)?;

    config::save(&state.config_path, &settings).map_err(|e| e.to_string())?;

    let mut conn = db::open(&state.db_path).map_err(|e| e.to_string())?;
    roots::sync_roots(&mut conn, &settings.roots).map_err(|e| e.to_string())?;

    Ok(())
}

/// Validates the same enabled roots the scan worker will use (SQLite `roots` table), using
/// canonicalize like `scan::walk_files`. Call before enqueueing a job.
fn validate_start_scan(db_path: &Path, config_path: &Path) -> Result<(), String> {
    let settings = config::load(config_path).map_err(|e| e.to_string())?;
    let cfg_enabled = settings
        .roots
        .iter()
        .filter(|r| r.enabled && !r.path.trim().is_empty())
        .count();

    let conn = db::open(db_path).map_err(|e| e.to_string())?;
    let root_rows = roots::list_enabled_root_rows(&conn).map_err(|e| e.to_string())?;

    let paths: Vec<String> = root_rows.iter().map(|(_, p)| p.clone()).collect();
    info!(
        target: "vessel_jobs",
        "validate_start_scan: DB enabled roots count={} paths={paths:?} (config enabled count={cfg_enabled})",
        paths.len(),
    );

    if root_rows.is_empty() {
        if cfg_enabled > 0 {
            warn!(
                target: "vessel_jobs",
                "validate_start_scan: config lists enabled roots but DB has none — user should save Settings"
            );
            return Err(
                "Settings list folders but no enabled roots are stored yet. Open Settings and click Save."
                    .into(),
            );
        }
        return Err(
            "No indexed folders are enabled. Enable at least one root in Settings and save."
                .into(),
        );
    }

    for path in &paths {
        let p = path_norm::normalize_root_path(path);
        if Path::new(&p).canonicalize().is_err() {
            warn!(
                target: "vessel_jobs",
                "validate_start_scan: path failed canonicalize path={p}"
            );
            return Err(format!("Path does not exist or is not accessible: {p}"));
        }
    }

    let mut stmt = conn
        .prepare(
            "SELECT 1 FROM jobs WHERE type = 'scan' AND status IN ('queued', 'running') LIMIT 1",
        )
        .map_err(|e| e.to_string())?;
    let has_active = stmt.exists([]).map_err(|e| e.to_string())?;
    if has_active {
        warn!(target: "vessel_jobs", "validate_start_scan: rejected duplicate active scan");
        return Err("A scan is already queued or running.".into());
    }

    info!(target: "vessel_jobs", "validate_start_scan: ok");
    Ok(())
}

#[tauri::command]
pub fn start_scan(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    validate_start_scan(&state.db_path, &state.config_path)?;
    let job_id = state
        .jobs
        .spawn_scan(app, state.db_path.clone(), state.config_path.clone())
        .map_err(|e| e.to_string())?;
    info!(
        target: "vessel_jobs",
        "start_scan: enqueued job_id={job_id}"
    );
    Ok(job_id)
}

#[tauri::command]
pub fn cancel_job(job_id: String, state: State<'_, AppState>) -> Result<(), String> {
    info!(target: "vessel_jobs", "cancel_job invoke job_id={job_id}");
    state.jobs.cancel(&job_id).map_err(|e| e.to_string())?;
    crate::jobs::persist_job_cancelled(&state.db_path, &job_id).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn list_jobs(state: State<'_, AppState>) -> Result<Vec<JobRecord>, String> {
    let conn = db::open(&state.db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, type AS job_type, status, progress_json, error, created_at, updated_at
             FROM jobs ORDER BY datetime(created_at) DESC LIMIT 100",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            let progress_json: String = row.get(3)?;
            let progress: Option<JobProgress> = if progress_json.is_empty() || progress_json == "null" {
                None
            } else {
                serde_json::from_str(&progress_json).ok()
            };
            Ok(JobRecord {
                id: row.get(0)?,
                job_type: row.get(1)?,
                status: row.get(2)?,
                progress,
                error: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(rows)
}

#[tauri::command]
pub fn clear_job_history(state: State<'_, AppState>) -> Result<usize, String> {
    let conn = db::open(&state.db_path).map_err(|e| e.to_string())?;
    let n = conn
        .execute(
            "DELETE FROM jobs WHERE status IN ('completed', 'failed', 'cancelled')",
            [],
        )
        .map_err(|e| e.to_string())?;
    info!(
        target: "vessel_jobs",
        "clear_job_history: removed {n} terminal job row(s)"
    );
    Ok(n)
}

#[tauri::command]
pub fn search_files(
    query: String,
    limit: i64,
    offset: i64,
    state: State<'_, AppState>,
) -> Result<Vec<SearchHit>, String> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(vec![]);
    }
    let conn = db::open(&state.db_path).map_err(|e| e.to_string())?;
    let lim = limit.clamp(1, 2000);
    let off = offset.max(0);
    search::search_paths(&conn, q, lim, off).map_err(|e| e.to_string())
}

fn resolve_existing_file(path: &str) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("Path is empty.".into());
    }
    let p = Path::new(trimmed);
    let canon = p
        .canonicalize()
        .map_err(|e| format!("Path does not exist or is not accessible: {e}"))?;
    let meta = std::fs::metadata(&canon).map_err(|e| format!("Could not read path: {e}"))?;
    if !meta.is_file() {
        return Err("Path is not a file.".into());
    }
    Ok(canon)
}

#[tauri::command]
pub fn open_file(path: String) -> Result<(), String> {
    let pb = resolve_existing_file(&path)?;
    open::that(&pb).map_err(|e| format!("Could not open file: {e}"))
}

#[tauri::command]
pub fn reveal_in_explorer(path: String) -> Result<(), String> {
    let pb = resolve_existing_file(&path)?;

    #[cfg(windows)]
    {
        let s = pb.to_string_lossy();
        let arg = format!("/select,\"{}\"", s.replace('"', ""));
        std::process::Command::new("explorer")
            .arg(arg)
            .spawn()
            .map_err(|e| format!("Could not open Explorer: {e}"))?;
        return Ok(());
    }

    #[cfg(not(windows))]
    {
        let parent = pb
            .parent()
            .ok_or_else(|| "File has no parent directory.".to_string())?;
        open::that(parent).map_err(|e| format!("Could not open folder: {e}"))
    }
}
