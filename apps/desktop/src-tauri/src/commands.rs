use std::path::{Path, PathBuf};
use std::sync::Arc;

use globset::Glob;
use log::{info, warn};
use tauri::State;

use crate::config;
use crate::db;
use crate::dto::{AppSettings, IndexingStatus, SearchHit};
use crate::jobs::JobManager;
use crate::path_norm;
use crate::roots;
use crate::search;

#[cfg(windows)]
use crate::windows_explorer::{path_string_for_explorer_select, reveal_file_in_explorer};

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
    let prev = config::load(&state.config_path).ok();
    let prev_content_on = prev.as_ref().map(|p| p.content_indexing_enabled);

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

    let th = settings.theme.trim();
    if th != "light" && th != "dark" {
        return Err("theme must be \"light\" or \"dark\".".into());
    }
    settings.theme = th.to_string();

    settings.content_max_bytes_per_file = settings
        .content_max_bytes_per_file
        .clamp(256 * 1024, 100 * 1024 * 1024);

    config::save(&state.config_path, &settings).map_err(|e| e.to_string())?;

    let mut conn = db::open(&state.db_path).map_err(|e| e.to_string())?;
    roots::sync_roots(&mut conn, &settings.roots).map_err(|e| e.to_string())?;

    if prev_content_on != Some(settings.content_indexing_enabled) {
        db::clear_all_file_content(&conn).map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Validates the same enabled roots the scan worker will use (SQLite `roots` table), using
/// canonicalize like `scan::walk_files`. Call before enqueueing a job.
fn validate_start_scan(db_path: &Path, config_path: &Path, jobs: &JobManager) -> Result<(), String> {
    if jobs.has_active_scan() {
        return Err("A scan is already queued or running.".into());
    }
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

    info!(target: "vessel_jobs", "validate_start_scan: ok");
    Ok(())
}

#[tauri::command]
pub fn start_scan(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    validate_start_scan(&state.db_path, &state.config_path, &state.jobs)?;
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
    Ok(())
}

#[tauri::command]
pub fn get_indexing_status(state: State<'_, AppState>) -> Result<IndexingStatus, String> {
    let content_indexing_enabled = config::load(&state.config_path)
        .map(|s| s.content_indexing_enabled)
        .unwrap_or(false);

    let conn = db::open(&state.db_path).map_err(|e| e.to_string())?;
    let files_indexed = db::count_present_files(&conn).map_err(|e| e.to_string())?;
    let (last_scan_at, last_scan_status, last_scan_error) =
        db::read_indexing_meta(&conn).map_err(|e| e.to_string())?;

    if let Some((job_id, progress)) = state.jobs.get_scan_snapshot() {
        return Ok(IndexingStatus {
            state: "scanning".to_string(),
            progress: Some(progress),
            last_scan_at,
            last_scan_status: last_scan_status.clone(),
            last_error: None,
            files_indexed,
            active_job_id: Some(job_id),
            content_indexing_enabled,
        });
    }

    let ui_state = if last_scan_status == "failed" {
        "error"
    } else {
        "idle"
    };

    Ok(IndexingStatus {
        state: ui_state.to_string(),
        progress: None,
        last_scan_at,
        last_scan_status,
        last_error: last_scan_error,
        files_indexed,
        active_job_id: None,
        content_indexing_enabled,
    })
}

#[tauri::command]
pub fn search_files(
    query: String,
    limit: i64,
    offset: i64,
    extension_filter: Option<String>,
    modified_from_ns: Option<i64>,
    modified_to_ns: Option<i64>,
    state: State<'_, AppState>,
) -> Result<Vec<SearchHit>, String> {
    let conn = db::open(&state.db_path).map_err(|e| e.to_string())?;
    let lim = limit.clamp(1, 2000);
    let off = offset.max(0);
    let ext = extension_filter
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    search::search_paths(
        &conn,
        &query,
        lim,
        off,
        ext,
        modified_from_ns,
        modified_to_ns,
    )
    .map_err(|e| e.to_string())
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
    info!(
        target: "vessel_explorer",
        "reveal_in_explorer: raw argument from frontend len={} bytes",
        path.len()
    );
    info!(target: "vessel_explorer", "reveal_in_explorer: raw argument={path:?}");

    let pb = resolve_existing_file(&path)?;
    info!(
        target: "vessel_explorer",
        "reveal_in_explorer: canonicalized path={}",
        pb.display()
    );

    #[cfg(windows)]
    {
        let select_path = path_string_for_explorer_select(&pb);
        info!(
            target: "vessel_explorer",
            "reveal_in_explorer: using ShellExecuteW select_path={select_path:?}"
        );

        match reveal_file_in_explorer(&select_path) {
            Ok(()) => return Ok(()),
            Err(e) => {
                warn!(
                    target: "vessel_explorer",
                    "reveal_in_explorer: ShellExecuteW failed ({e}), opening parent folder"
                );
                let parent = pb
                    .parent()
                    .ok_or_else(|| "File has no parent directory.".to_string())?;
                let parent_display = path_string_for_explorer_select(parent);
                info!(
                    target: "vessel_explorer",
                    "reveal_in_explorer: fallback open folder path={parent_display:?}"
                );
                open::that(parent_display).map_err(|e2| {
                    format!("Could not reveal in Explorer ({e}); could not open folder: {e2}")
                })
            }
        }
    }

    #[cfg(not(windows))]
    {
        let parent = pb
            .parent()
            .ok_or_else(|| "File has no parent directory.".to_string())?;
        open::that(parent).map_err(|e| format!("Could not open folder: {e}"))
    }
}
