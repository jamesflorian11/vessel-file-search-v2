use anyhow::Context;
use globset::GlobSet;
use log::warn;
use std::path::Path;
use tokio_util::sync::CancellationToken;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct ScannedFile {
    pub rel_path: String,
    pub size: i64,
    pub mtime_ns: i64,
}

#[derive(Debug, Clone, Default)]
pub struct WalkSummary {
    pub seen: u64,
    pub completed: bool,
    pub walk_errors: u64,
    pub metadata_errors: u64,
}

pub fn path_for_glob(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

/// Walk `root` and invoke `on_batch` with up to `batch_max` files per call.
pub fn walk_files(
    root: &Path,
    globs: &GlobSet,
    cancel: &CancellationToken,
    batch_max: usize,
    mut on_batch: impl FnMut(Vec<ScannedFile>) -> anyhow::Result<()>,
) -> anyhow::Result<WalkSummary> {
    let root = root.canonicalize().with_context(|| {
        format!(
            "Path does not exist or is not accessible: {}",
            root.display()
        )
    })?;
    let root_norm = path_for_glob(&root);

    let mut summary = WalkSummary {
        completed: true,
        ..WalkSummary::default()
    };
    let mut batch: Vec<ScannedFile> = Vec::with_capacity(batch_max.min(512));

    for entry in WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let p = e.path();
            let rel = match p.strip_prefix(&root) {
                Ok(r) => r,
                Err(_) => return true,
            };
            let rel_s = if rel.as_os_str().is_empty() {
                String::new()
            } else {
                path_for_glob(rel)
            };
            let full = if rel_s.is_empty() {
                root_norm.clone()
            } else {
                format!("{root_norm}/{rel_s}")
            };
            !globs.is_match(Path::new(&full))
        })
    {
        if cancel.is_cancelled() {
            summary.completed = false;
            break;
        }
        let entry = match entry {
            Ok(e) => e,
            Err(_) => {
                summary.walk_errors += 1;
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let rel = path.strip_prefix(&root).unwrap_or(path);
        let rel_path = if rel.as_os_str().is_empty() {
            String::new()
        } else {
            path_for_glob(rel)
        };

        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => {
                summary.metadata_errors += 1;
                continue;
            }
        };
        let size = meta.len() as i64;
        let mtime_ns = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos() as i64)
            .unwrap_or(0);

        batch.push(ScannedFile {
            rel_path,
            size,
            mtime_ns,
        });
        summary.seen += 1;

        if batch.len() >= batch_max {
            let take = std::mem::replace(&mut batch, Vec::with_capacity(batch_max.min(512)));
            on_batch(take)?;
            if cancel.is_cancelled() {
                summary.completed = false;
                break;
            }
        }
    }

    if !batch.is_empty() && summary.completed {
        on_batch(batch)?;
    } else if !batch.is_empty() {
        warn!(
            target: "vessel_scan",
            "walk_files: dropping trailing batch due to cancellation pending_count={}",
            batch.len()
        );
    }

    Ok(summary)
}
