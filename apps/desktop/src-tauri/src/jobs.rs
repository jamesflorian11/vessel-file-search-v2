use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::Context;
use chrono::Utc;
use globset::{Glob, GlobSetBuilder};
use log::{error, info};
use rusqlite::params;
use tauri::async_runtime::{self, JoinHandle};
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::config;
use crate::db;
use crate::dto::{JobProgress, JobProgressEvent};
use crate::index;
use crate::roots;
use crate::scan;

pub struct JobManager {
    inner: Mutex<JobManagerInner>,
}

struct JobManagerInner {
    cancel: HashMap<String, CancellationToken>,
    handles: HashMap<String, JoinHandle<()>>,
}

fn lock_jobs(
    inner: &Mutex<JobManagerInner>,
) -> anyhow::Result<std::sync::MutexGuard<'_, JobManagerInner>> {
    inner
        .lock()
        .map_err(|_| anyhow::anyhow!("job manager mutex poisoned"))
}

/// Mark orphaned queued/running rows after restart (no in-process tokens).
pub fn reconcile_stale_jobs_on_startup(db_path: &Path) -> anyhow::Result<usize> {
    let conn = db::open(db_path)?;
    let now = Utc::now().to_rfc3339();
    let msg = "Application restarted before this job finished.";
    let n = conn.execute(
        "UPDATE jobs SET status = 'failed', error = ?1, updated_at = ?2
         WHERE type = 'scan' AND status IN ('queued', 'running')",
        params![msg, now],
    )?;
    if n > 0 {
        info!(
            target: "vessel_jobs",
            "reconcile_stale_jobs_on_startup: marked {n} stale job(s) as failed"
        );
    }
    Ok(n)
}

/// Persist cancel immediately so the UI does not stay on queued/running until the worker observes the token.
pub fn persist_job_cancelled(db_path: &Path, job_id: &str) -> anyhow::Result<()> {
    let conn = db::open(db_path)?;
    let now = Utc::now().to_rfc3339();
    let n = conn.execute(
        "UPDATE jobs SET status = 'cancelled', error = NULL, updated_at = ?1
         WHERE id = ?2 AND status IN ('queued', 'running')",
        params![now, job_id],
    )?;
    info!(
        target: "vessel_jobs",
        "persist_job_cancelled job_id={job_id} rows_updated={n}"
    );
    Ok(())
}

pub enum ScanRunGate {
    Proceed,
    Stop,
}

/// Transition queued → running only. If cancel already persisted, no row matches and we stop.
pub fn try_begin_scan_run(
    db_path: &Path,
    job_id: &str,
    initial: &JobProgress,
) -> anyhow::Result<ScanRunGate> {
    let conn = db::open(db_path)?;
    let now = Utc::now().to_rfc3339();
    let progress_json = serde_json::to_string(initial)?;
    let n = conn.execute(
        "UPDATE jobs SET status = 'running', progress_json = ?1, updated_at = ?2
         WHERE id = ?3 AND status = 'queued'",
        params![progress_json, now, job_id],
    )?;
    if n == 1 {
        return Ok(ScanRunGate::Proceed);
    }
    let status: String = conn.query_row(
        "SELECT status FROM jobs WHERE id = ?1",
        params![job_id],
        |row| row.get(0),
    )?;
    info!(
        target: "vessel_jobs",
        "try_begin_scan_run: no transition job_id={job_id} db_status={status}"
    );
    Ok(ScanRunGate::Stop)
}

impl JobManager {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(JobManagerInner {
                cancel: HashMap::new(),
                handles: HashMap::new(),
            }),
        }
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

        let mgr = Arc::clone(self);
        let job_id_clone = job_id.clone();

        insert_job_row(&db_path, &job_id, "queued", None)?;

        info!(
            target: "vessel_jobs",
            "enqueue scan job_id={job_id} status=queued"
        );

        let db_path_blocking = db_path.clone();
        let config_path_blocking = config_path.clone();

        info!(
            target: "vessel_jobs",
            "spawn_scan scheduling async task job_id={job_id}"
        );

        let handle = async_runtime::spawn(async move {
            info!(
                target: "vessel_jobs",
                "scan async task started job_id={job_id_clone}"
            );

            let res = tokio::task::spawn_blocking({
                let job_id = job_id_clone.clone();
                let token = token.clone();
                let app = app.clone();
                move || {
                    info!(
                        target: "vessel_jobs",
                        "scan blocking worker started job_id={job_id}"
                    );
                    match run_scan_job(
                        &app,
                        &db_path_blocking,
                        &config_path_blocking,
                        &job_id,
                        &token,
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
                            if let Err(db_err) = insert_job_terminal(
                                &db_path_blocking,
                                &job_id,
                                "failed",
                                Some(&e.to_string()),
                            ) {
                                error!(
                                    target: "vessel_jobs",
                                    "failed to persist scan failure status job_id={job_id} err={db_err}"
                                );
                            }
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
                if let Err(db_err) = insert_job_terminal(
                    &db_path,
                    &job_id_clone,
                    "failed",
                    Some(&format!("join error: {e}")),
                ) {
                    error!(
                        target: "vessel_jobs",
                        "failed to persist join error status job_id={job_id_clone} err={db_err}"
                    );
                }
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
                return Err(e);
            }
        };
        g.handles.insert(job_id.clone(), handle);

        Ok(job_id)
    }
}

fn insert_job_row(
    db_path: &Path,
    job_id: &str,
    status: &str,
    progress: Option<&JobProgress>,
) -> anyhow::Result<()> {
    let conn = db::open(db_path)?;
    let now = Utc::now().to_rfc3339();
    let progress_json = match progress {
        Some(p) => serde_json::to_string(p)?,
        None => "null".to_string(),
    };
    conn.execute(
        "INSERT INTO jobs (id, type, status, payload_json, progress_json, error, created_at, updated_at)
         VALUES (?1, 'scan', ?2, NULL, ?3, NULL, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET
           status = excluded.status,
           progress_json = excluded.progress_json,
           updated_at = excluded.updated_at",
        params![job_id, status, progress_json, now, now],
    )?;
    Ok(())
}

fn insert_job_terminal(
    db_path: &Path,
    job_id: &str,
    status: &str,
    err: Option<&str>,
) -> anyhow::Result<()> {
    let conn = db::open(db_path)?;
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE jobs SET status = ?1, error = ?2, updated_at = ?3 WHERE id = ?4",
        params![status, err, now, job_id],
    )?;
    Ok(())
}

fn update_job_progress_db(db_path: &Path, job_id: &str, p: &JobProgress) -> anyhow::Result<()> {
    let conn = db::open(db_path)?;
    let now = Utc::now().to_rfc3339();
    let progress_json = serde_json::to_string(p)?;
    conn.execute(
        "UPDATE jobs SET progress_json = ?1, updated_at = ?2 WHERE id = ?3",
        params![progress_json, now, job_id],
    )?;
    Ok(())
}

fn emit_progress(app: &AppHandle, job_id: &str, p: &JobProgress) {
    let _ = app.emit(
        "job_progress",
        JobProgressEvent {
            job_id: job_id.to_string(),
            progress: p.clone(),
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
) -> anyhow::Result<()> {
    if cancel.is_cancelled() {
        insert_job_terminal(db_path, job_id, "cancelled", None)?;
        return Ok(());
    }

    let settings = config::load(config_path)?;
    let batch_size = settings.batch_size.clamp(200, 20_000);
    let globs = build_globset(&settings.exclusion_globs)?;

    let initial = JobProgress {
        phase: "starting".into(),
        files_seen: 0,
        files_upserted: 0,
        current_path: None,
        roots_total: 0,
        roots_done: 0,
    };

    match try_begin_scan_run(db_path, job_id, &initial)? {
        ScanRunGate::Stop => {
            info!(
                target: "vessel_jobs",
                "run_scan_job: abort before run (cancelled or not queued) job_id={job_id}"
            );
            return Ok(());
        }
        ScanRunGate::Proceed => {}
    }

    emit_progress(app, job_id, &initial);

    if cancel.is_cancelled() {
        insert_job_terminal(db_path, job_id, "cancelled", None)?;
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
            current_path: None,
            roots_total: 0,
            roots_done: 0,
        };
        update_job_progress_db(db_path, job_id, &p)?;
        emit_progress(app, job_id, &p);
        insert_job_terminal(db_path, job_id, "completed", None)?;
        return Ok(());
    }

    let mut files_seen: u64 = 0;
    let mut files_upserted: u64 = 0;
    let mut last_emit = Instant::now() - Duration::from_millis(500);

    for (ri, (root_id, root_path)) in root_rows.iter().enumerate() {
        if cancel.is_cancelled() {
            insert_job_terminal(db_path, job_id, "cancelled", None)?;
            return Ok(());
        }

        let path = PathBuf::from(root_path);
        let mut conn = db::open(db_path)?;

        scan::walk_files(
            &path,
            &globs,
            cancel,
            batch_size.min(4096).max(256),
            |batch| {
                if cancel.is_cancelled() {
                    return Ok(());
                }
                let n = index::apply_one_batch(&mut conn, *root_id, &batch)?;
                files_upserted += n;
                files_seen += batch.len() as u64;

                let p = JobProgress {
                    phase: "indexing".into(),
                    files_seen,
                    files_upserted,
                    current_path: batch
                        .last()
                        .map(|f| f.rel_path.chars().take(200).collect()),
                    roots_total,
                    roots_done: ri as u32,
                };

                if last_emit.elapsed() >= Duration::from_millis(100) {
                    let _ = update_job_progress_db(db_path, job_id, &p);
                    emit_progress(app, job_id, &p);
                    last_emit = Instant::now();
                }
                Ok(())
            },
        )?;

        let p = JobProgress {
            phase: "indexing".into(),
            files_seen,
            files_upserted,
            current_path: None,
            roots_total,
            roots_done: (ri + 1) as u32,
        };
        let _ = update_job_progress_db(db_path, job_id, &p);
        emit_progress(app, job_id, &p);
    }

    let final_p = JobProgress {
        phase: "completed".into(),
        files_seen,
        files_upserted,
        current_path: None,
        roots_total,
        roots_done: roots_total,
    };
    update_job_progress_db(db_path, job_id, &final_p)?;
    emit_progress(app, job_id, &final_p);
    insert_job_terminal(db_path, job_id, "completed", None)?;
    Ok(())
}
