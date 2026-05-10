use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Context;
use chrono::Utc;
use globset::{Glob, GlobSetBuilder};
use log::{error, info};
use tauri::async_runtime::{self, JoinHandle};
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::config;
use crate::content_extract;
use crate::db;
use crate::dto::{JobProgress, JobProgressEvent, JobTerminalEvent};
use crate::index;
use crate::job_types;
use crate::roots;
use crate::scan;

struct ActiveScan {
    job_id: String,
    progress: JobProgress,
}

pub struct JobManager {
    inner: Mutex<JobManagerInner>,
}

struct JobManagerInner {
    cancel: HashMap<String, CancellationToken>,
    handles: HashMap<String, JoinHandle<()>>,
    active_scan: Option<ActiveScan>,
}

fn lock_jobs(
    inner: &Mutex<JobManagerInner>,
) -> anyhow::Result<std::sync::MutexGuard<'_, JobManagerInner>> {
    inner
        .lock()
        .map_err(|_| anyhow::anyhow!("job manager mutex poisoned"))
}

impl JobManager {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(JobManagerInner {
                cancel: HashMap::new(),
                handles: HashMap::new(),
                active_scan: None,
            }),
        }
    }

    /// True while a scan task is queued/running (in-memory; no job history persisted).
    pub fn has_active_scan(&self) -> bool {
        lock_jobs(&self.inner)
            .map(|g| g.active_scan.is_some() || !g.handles.is_empty())
            .unwrap_or(false)
    }

    pub fn register_active_scan(&self, job_id: &str, progress: JobProgress) -> anyhow::Result<()> {
        let mut g = lock_jobs(&self.inner)?;
        g.active_scan = Some(ActiveScan {
            job_id: job_id.to_string(),
            progress,
        });
        Ok(())
    }

    fn set_scan_progress(&self, job_id: &str, progress: JobProgress) -> anyhow::Result<()> {
        let mut g = lock_jobs(&self.inner)?;
        if let Some(ref mut a) = g.active_scan {
            if a.job_id == job_id {
                a.progress = progress;
            }
        }
        Ok(())
    }

    pub fn clear_active_scan(&self, job_id: &str) -> anyhow::Result<()> {
        let mut g = lock_jobs(&self.inner)?;
        if g.active_scan.as_ref().is_some_and(|a| a.job_id == job_id) {
            g.active_scan = None;
        }
        Ok(())
    }

    pub fn get_scan_snapshot(&self) -> Option<(String, JobProgress)> {
        lock_jobs(&self.inner).ok().and_then(|g| {
            g.active_scan
                .as_ref()
                .map(|a| (a.job_id.clone(), a.progress.clone()))
        })
    }

    pub fn cancel(&self, job_id: &str) -> anyhow::Result<()> {
        let g = lock_jobs(&self.inner)?;
        if let Some(t) = g.cancel.get(job_id) {
            t.cancel();
        }
        Ok(())
    }

    pub fn spawn_scan(
        self: &Arc<Self>,
        app: AppHandle,
        db_path: PathBuf,
        config_path: PathBuf,
    ) -> anyhow::Result<String> {
        let job_id = Uuid::new_v4().to_string();
        let token = CancellationToken::new();
        {
            let mut g = lock_jobs(&self.inner)?;
            g.cancel.insert(job_id.clone(), token.clone());
        }

        let content_flag = config::load(&config_path)
            .map(|s| s.content_indexing_enabled)
            .unwrap_or(false);
        let queued = JobProgress {
            phase: "queued".into(),
            files_seen: 0,
            files_upserted: 0,
            files_deleted: 0,
            current_path: None,
            roots_total: 0,
            roots_done: 0,
            content_indexing_enabled: content_flag,
        };
        self.register_active_scan(&job_id, queued)?;

        let mgr = Arc::clone(self);
        let job_id_clone = job_id.clone();

        let db_path_blocking = db_path.clone();
        let config_path_blocking = config_path.clone();

        info!(
            target: "vessel_jobs",
            "enqueue scan type={} job_id={job_id} (in-memory status only)",
            job_types::SCAN
        );

        let handle = async_runtime::spawn(async move {
            let app_for_join_err = app.clone();

            let res = tokio::task::spawn_blocking({
                let job_id = job_id_clone.clone();
                let token = token.clone();
                let app = app.clone();
                let mgr = Arc::clone(&mgr);
                move || {
                    match run_scan_job(
                        &app,
                        &db_path_blocking,
                        &config_path_blocking,
                        &job_id,
                        &token,
                        &mgr,
                    ) {
                        Ok(()) => {
                            info!(
                                target: "vessel_jobs",
                                "scan blocking worker finished ok job_id={job_id}"
                            );
                        }
                        Err(e) => {
                            error!(
                                target: "vessel_jobs",
                                "scan job failed job_id={job_id} err={e}"
                            );
                            let _ = finalize_scan_terminal(
                                &app,
                                &db_path_blocking,
                                &mgr,
                                &job_id,
                                "failed",
                                Some(&e.to_string()),
                                None,
                            );
                        }
                    }
                }
            })
            .await;

            if let Ok(mut g) = lock_jobs(&mgr.inner) {
                g.cancel.remove(&job_id_clone);
                g.handles.remove(&job_id_clone);
            } else {
                error!(
                    target: "vessel_jobs",
                    "could not lock job manager after scan job_id={job_id_clone}"
                );
            }

            if let Err(e) = res {
                error!(
                    target: "vessel_jobs",
                    "scan spawn_blocking join error job_id={job_id_clone} err={e}"
                );
                let _ = finalize_scan_terminal(
                    &app_for_join_err,
                    &db_path,
                    &mgr,
                    &job_id_clone,
                    "failed",
                    Some(&format!("join error: {e}")),
                    None,
                );
            }
        });

        let mut g = match lock_jobs(&self.inner) {
            Ok(g) => g,
            Err(e) => {
                error!(
                    target: "vessel_jobs",
                    "failed to register scan job handle job_id={job_id} err={e}"
                );
                handle.abort();
                let _ = self.clear_active_scan(&job_id);
                return Err(e);
            }
        };
        g.handles.insert(job_id.clone(), handle);

        Ok(job_id)
    }
}

fn finalize_scan_terminal(
    app: &AppHandle,
    db_path: &Path,
    mgr: &Arc<JobManager>,
    job_id: &str,
    status: &str,
    err: Option<&str>,
    progress: Option<JobProgress>,
) -> anyhow::Result<()> {
    let now = Utc::now().to_rfc3339();
    let meta_status = match status {
        "completed" => "completed",
        "failed" => "failed",
        "cancelled" => "cancelled",
        _ => "idle",
    };
    let conn = db::open(db_path)?;
    db::write_indexing_meta(&conn, &now, meta_status, err)?;
    drop(conn);
    mgr.clear_active_scan(job_id)?;
    emit_job_terminal(app, job_id, status, progress);
    Ok(())
}

fn emit_progress(app: &AppHandle, job_id: &str, p: &JobProgress, mgr: &Arc<JobManager>) {
    let _ = mgr.set_scan_progress(job_id, p.clone());
    let _ = app.emit(
        "job_progress",
        JobProgressEvent {
            job_id: job_id.to_string(),
            progress: p.clone(),
        },
    );
}

fn emit_job_terminal(
    app: &AppHandle,
    job_id: &str,
    status: &str,
    progress: Option<JobProgress>,
) {
    let _ = app.emit(
        "job_terminal",
        JobTerminalEvent {
            job_id: job_id.to_string(),
            status: status.to_string(),
            progress,
        },
    );
}

fn build_globset(patterns: &[String]) -> anyhow::Result<globset::GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for p in patterns {
        builder.add(Glob::new(p).with_context(|| format!("invalid glob: {p}"))?);
    }
    Ok(builder.build()?)
}

fn run_scan_job(
    app: &AppHandle,
    db_path: &Path,
    config_path: &Path,
    job_id: &str,
    cancel: &CancellationToken,
    mgr: &Arc<JobManager>,
) -> anyhow::Result<()> {
    if cancel.is_cancelled() {
        finalize_scan_terminal(app, db_path, mgr, job_id, "cancelled", None, None)?;
        return Ok(());
    }

    let settings = config::load(config_path)?;
    let batch_size = settings.batch_size.clamp(200, 20_000);
    let globs = build_globset(&settings.exclusion_globs)?;

    let initial = JobProgress {
        phase: "starting".into(),
        files_seen: 0,
        files_upserted: 0,
        files_deleted: 0,
        current_path: None,
        roots_total: 0,
        roots_done: 0,
        content_indexing_enabled: settings.content_indexing_enabled,
    };
    emit_progress(app, job_id, &initial, mgr);

    if cancel.is_cancelled() {
        finalize_scan_terminal(app, db_path, mgr, job_id, "cancelled", None, None)?;
        return Ok(());
    }

    let conn = db::open(db_path)?;
    let root_rows = roots::list_enabled_root_rows(&conn)?;
    let roots_total = root_rows.len() as u32;
    drop(conn);

    if roots_total == 0 {
        let p = JobProgress {
            phase: "completed".into(),
            files_seen: 0,
            files_upserted: 0,
            files_deleted: 0,
            current_path: None,
            roots_total: 0,
            roots_done: 0,
            content_indexing_enabled: settings.content_indexing_enabled,
        };
        emit_progress(app, job_id, &p, mgr);
        finalize_scan_terminal(app, db_path, mgr, job_id, "completed", None, Some(p))?;
        return Ok(());
    }

    let mut files_seen: u64 = 0;
    let mut files_upserted: u64 = 0;
    let mut files_deleted: u64 = 0;
    let mut last_emit = Instant::now() - Duration::from_millis(500);

    for (ri, (root_id, root_path)) in root_rows.iter().enumerate() {
        if cancel.is_cancelled() {
            finalize_scan_terminal(app, db_path, mgr, job_id, "cancelled", None, None)?;
            return Ok(());
        }

        let path = PathBuf::from(root_path);
        let root_canon = path
            .canonicalize()
            .with_context(|| format!("canonicalize root {}", root_path))?;
        let root_abs_str = root_canon.to_string_lossy().to_string();
        let mut conn = db::open(db_path)?;
        let mut seen_paths: HashSet<String> = HashSet::new();

        scan::walk_files(
            &root_canon,
            &globs,
            cancel,
            batch_size.min(4096).max(256),
            |batch| {
                if cancel.is_cancelled() {
                    return Ok(());
                }
                for f in &batch {
                    seen_paths.insert(f.rel_path.clone());
                }
                let n = index::apply_one_batch(&mut conn, *root_id, &root_abs_str, &batch)?;
                content_extract::index_batch_file_contents(
                    &mut conn,
                    &root_canon,
                    *root_id,
                    &batch,
                    &settings,
                )?;
                files_upserted += n;
                files_seen += batch.len() as u64;

                let p = JobProgress {
                    phase: "indexing".into(),
                    files_seen,
                    files_upserted,
                    files_deleted,
                    current_path: batch
                        .last()
                        .map(|f| f.rel_path.chars().take(200).collect()),
                    roots_total,
                    roots_done: ri as u32,
                    content_indexing_enabled: settings.content_indexing_enabled,
                };

                if last_emit.elapsed() >= Duration::from_millis(100) {
                    emit_progress(app, job_id, &p, mgr);
                    last_emit = Instant::now();
                }
                Ok(())
            },
        )?;

        if cancel.is_cancelled() {
            finalize_scan_terminal(app, db_path, mgr, job_id, "cancelled", None, None)?;
            return Ok(());
        }

        let removed = index::reconcile_root_after_scan(
            &mut conn,
            *root_id,
            &seen_paths,
            !cancel.is_cancelled(),
        )?;
        files_deleted += removed;

        let p = JobProgress {
            phase: "indexing".into(),
            files_seen,
            files_upserted,
            files_deleted,
            current_path: None,
            roots_total,
            roots_done: (ri + 1) as u32,
            content_indexing_enabled: settings.content_indexing_enabled,
        };
        emit_progress(app, job_id, &p, mgr);
    }

    let final_p = JobProgress {
        phase: "completed".into(),
        files_seen,
        files_upserted,
        files_deleted,
        current_path: None,
        roots_total,
        roots_done: roots_total,
        content_indexing_enabled: settings.content_indexing_enabled,
    };
    emit_progress(app, job_id, &final_p, mgr);
    finalize_scan_terminal(
        app,
        db_path,
        mgr,
        job_id,
        "completed",
        None,
        Some(final_p.clone()),
    )?;
    Ok(())
}
