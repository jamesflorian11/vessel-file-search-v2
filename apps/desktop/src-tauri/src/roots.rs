use std::collections::HashSet;

use crate::dto::RootConfig;
use crate::path_norm;
use anyhow::Context;
use log::info;
use rusqlite::{params, Connection};

/// One-time cleanup of quoted paths already stored in SQLite.
pub fn migrate_normalize_paths(conn: &mut Connection) -> anyhow::Result<usize> {
    let mut stmt = conn.prepare("SELECT id, path FROM roots ORDER BY id")?;
    let rows: Vec<(i64, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    let mut seen: HashSet<String> = HashSet::new();
    let mut updated = 0usize;

    for (id, path) in rows {
        let n = path_norm::normalize_root_path(&path);
        if seen.contains(&n) {
            conn.execute("DELETE FROM roots WHERE id = ?1", params![id])?;
            info!(
                target: "vessel_paths",
                "migrate_normalize_paths: removed duplicate root id={id} path={path:?}"
            );
            updated += 1;
            continue;
        }
        seen.insert(n.clone());
        if n != path {
            conn.execute(
                "UPDATE roots SET path = ?1 WHERE id = ?2",
                params![&n, id],
            )?;
            info!(
                target: "vessel_paths",
                "migrate_normalize_paths: updated id={id} path={path:?} -> {n:?}"
            );
            updated += 1;
        }
    }

    Ok(updated)
}

pub fn sync_roots(conn: &mut Connection, roots: &[RootConfig]) -> anyhow::Result<()> {
    let roots: Vec<RootConfig> = roots
        .iter()
        .map(|r| RootConfig {
            path: path_norm::normalize_root_path(&r.path),
            display_name: r.display_name.clone(),
            enabled: r.enabled,
            read_only: r.read_only,
        })
        .collect();

    let tx = conn.transaction()?;

    if roots.is_empty() {
        tx.execute("DELETE FROM roots", [])
            .context("clear roots")?;
    } else {
        let mut paths: Vec<&str> = roots.iter().map(|r| r.path.as_str()).collect();
        paths.sort();
        paths.dedup();
        let placeholders = paths.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!("DELETE FROM roots WHERE path NOT IN ({placeholders})");
        tx.execute(&sql, rusqlite::params_from_iter(paths.iter().copied()))?;
    }

    for r in roots {
        let enabled = if r.enabled { 1i32 } else { 0i32 };
        let read_only = if r.read_only { 1i32 } else { 0i32 };
        tx.execute(
            "INSERT INTO roots (path, display_name, enabled, read_only) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(path) DO UPDATE SET
               display_name = excluded.display_name,
               enabled = excluded.enabled,
               read_only = excluded.read_only",
            params![&r.path, &r.display_name, enabled, read_only],
        )?;
    }

    tx.commit()?;
    Ok(())
}

pub fn list_enabled_root_rows(conn: &Connection) -> anyhow::Result<Vec<(i64, String)>> {
    let mut stmt = conn.prepare("SELECT id, path FROM roots WHERE enabled = 1 ORDER BY id")?;
    let rows = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}
