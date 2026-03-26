use std::collections::HashSet;

use anyhow::Context;
use log::info;
use rusqlite::{params, Connection};

use crate::scan::ScannedFile;

/// Batch size for populating the temp table of seen paths (keeps statements small).
const RECONCILE_SEEN_INSERT_CHUNK: usize = 500;

fn quick_sig(size: i64, mtime_ns: i64) -> String {
    format!("{size}:{mtime_ns}")
}

fn fts_delete(conn: &Connection, file_id: i64) -> anyhow::Result<()> {
    conn.execute(
        "DELETE FROM fts_files WHERE rowid = ?1",
        [file_id],
    )
    .with_context(|| format!("fts_delete(DELETE FROM fts_files WHERE rowid={file_id})"))?;
    Ok(())
}

fn fts_insert(conn: &Connection, file_id: i64, path: &str) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO fts_files(rowid, path) VALUES (?1, ?2)",
        params![file_id, path],
    )
    .with_context(|| format!("fts_insert(rowid={file_id}, path_len={})", path.len()))?;
    Ok(())
}

/// Apply a batch of scanned files for one root inside an existing transaction.
pub fn apply_batch(
    tx: &rusqlite::Transaction<'_>,
    root_id: i64,
    batch: &[ScannedFile],
) -> anyhow::Result<u64> {
    let mut upserted: u64 = 0;

    for f in batch {
        let sig = quick_sig(f.size, f.mtime_ns);
        let existing: Option<(i64, Option<String>)> = match tx.query_row(
            "SELECT id, quick_sig FROM files WHERE root_id = ?1 AND rel_path = ?2",
            params![root_id, &f.rel_path],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?)),
        ) {
            Ok(v) => Some(v),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => return Err(e.into()),
        };

        match existing {
            Some((_id, Some(prev))) if prev == sig => {}
            Some((id, _)) => {
                tx.execute(
                    "UPDATE files SET size = ?1, mtime_ns = ?2, quick_sig = ?3, file_state = 'present' WHERE id = ?4",
                    params![f.size, f.mtime_ns, &sig, id],
                )?;
                fts_delete(tx, id)?;
                fts_insert(tx, id, &f.rel_path)?;
                upserted += 1;
            }
            None => {
                tx.execute(
                    "INSERT INTO files (root_id, rel_path, size, mtime_ns, quick_sig, file_state)
                     VALUES (?1, ?2, ?3, ?4, ?5, 'present')",
                    params![root_id, &f.rel_path, f.size, f.mtime_ns, &sig],
                )?;
                let id = tx.last_insert_rowid();
                fts_insert(tx, id, &f.rel_path)?;
                upserted += 1;
            }
        }
    }

    Ok(upserted)
}

pub fn apply_one_batch(
    conn: &mut Connection,
    root_id: i64,
    batch: &[ScannedFile],
) -> anyhow::Result<u64> {
    let tx = conn.transaction().context("begin apply_one_batch")?;
    let n = apply_batch(&tx, root_id, batch)?;
    tx.commit()?;
    Ok(n)
}

/// Remove DB + FTS rows for paths under `root_id` that were not seen during this scan walk.
///
/// `walk_completed` must be false when the walk was cancelled or failed partway through; in that
/// case reconciliation is skipped so partial `seen_rel_paths` cannot delete valid index rows.
///
/// Strategy: populate a temp table with every `rel_path` seen on disk for this root, then delete
/// in one transaction using `NOT EXISTS` (set-based, suitable for large indexes).
pub fn reconcile_root_after_scan(
    conn: &mut Connection,
    root_id: i64,
    seen_rel_paths: &HashSet<String>,
    walk_completed: bool,
) -> anyhow::Result<u64> {
    if !walk_completed {
        info!(
            target: "vessel_index",
            "reconcile_root_after_scan: skipped (walk did not complete for root_id={root_id})"
        );
        return Ok(0);
    }

    let tx = conn
        .transaction()
        .context("reconcile: begin transaction (temp table + deletes)")?;

    tx.execute_batch(
        "
        DROP TABLE IF EXISTS _reconcile_seen;
        CREATE TEMP TABLE _reconcile_seen (rel_path TEXT NOT NULL PRIMARY KEY);
        ",
    )
    .context("reconcile: create temp table _reconcile_seen")?;

    let seen_vec: Vec<&String> = seen_rel_paths.iter().collect();
    for chunk in seen_vec.chunks(RECONCILE_SEEN_INSERT_CHUNK) {
        if chunk.is_empty() {
            continue;
        }
        let values_clause = chunk.iter().map(|_| "(?)").collect::<Vec<_>>().join(",");
        let sql = format!(
            "INSERT OR IGNORE INTO _reconcile_seen(rel_path) VALUES {values_clause}"
        );
        tx.execute(
            &sql,
            rusqlite::params_from_iter(chunk.iter().map(|s| s.as_str())),
        )
        .context("reconcile: insert batch into _reconcile_seen")?;
    }

    let stale_count: i64 = tx
        .query_row(
            "
            SELECT COUNT(*) FROM files f
            WHERE f.root_id = ?1
              AND NOT EXISTS (
                SELECT 1 FROM _reconcile_seen s WHERE s.rel_path = f.rel_path
              )
            ",
            params![root_id],
            |row| row.get(0),
        )
        .context("reconcile: count stale rows")?;

    info!(
        target: "vessel_index",
        "reconcile_root_after_scan: root_id={root_id} stale_candidates={stale_count} seen_on_disk={}",
        seen_rel_paths.len()
    );

    if stale_count == 0 {
        tx.execute_batch("DROP TABLE IF EXISTS _reconcile_seen;")
            .context("reconcile: drop temp table (no stale rows)")?;
        tx.commit().context("reconcile: commit (no deletes)")?;
        return Ok(0);
    }

    let fts_deleted = tx
        .execute(
            "
            DELETE FROM fts_files
            WHERE rowid IN (
                SELECT f.id FROM files f
                WHERE f.root_id = ?1
                  AND NOT EXISTS (
                    SELECT 1 FROM _reconcile_seen s WHERE s.rel_path = f.rel_path
                  )
            )
            ",
            params![root_id],
        )
        .context("reconcile: delete stale fts_files rows")?;

    let files_deleted = tx
        .execute(
            "
            DELETE FROM files
            WHERE root_id = ?1
              AND NOT EXISTS (
                SELECT 1 FROM _reconcile_seen s WHERE s.rel_path = files.rel_path
              )
            ",
            params![root_id],
        )
        .context("reconcile: delete stale files rows")?;

    tx.execute_batch("DROP TABLE IF EXISTS _reconcile_seen;")
        .context("reconcile: drop temp table")?;

    tx.commit()
        .context("reconcile: commit after stale deletes")?;

    info!(
        target: "vessel_index",
        "reconcile_root_after_scan: root_id={root_id} deleted_files={files_deleted} deleted_fts_rows={fts_deleted}"
    );

    Ok(files_deleted as u64)
}
