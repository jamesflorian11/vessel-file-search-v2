use anyhow::Context;
use rusqlite::{params, Connection};

use crate::scan::ScannedFile;

fn quick_sig(size: i64, mtime_ns: i64) -> String {
    format!("{size}:{mtime_ns}")
}

fn fts_delete(conn: &Connection, file_id: i64) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO fts_files(fts_files, rowid) VALUES('delete', ?1)",
        [file_id],
    )?;
    Ok(())
}

fn fts_insert(conn: &Connection, file_id: i64, path: &str) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO fts_files(rowid, path) VALUES (?1, ?2)",
        params![file_id, path],
    )?;
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
