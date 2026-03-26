use anyhow::Context;
use rusqlite::{params, Connection};
use std::path::Path;

use crate::dto::SearchHit;

pub fn build_fts_query(raw: &str) -> anyhow::Result<String> {
    let parts: Vec<&str> = raw.split_whitespace().filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        anyhow::bail!("empty query");
    }
    let escaped: Vec<String> = parts
        .iter()
        .map(|p| {
            let q = p.replace('\"', "\"\"");
            format!("\"{q}\"")
        })
        .collect();
    Ok(escaped.join(" AND "))
}

pub fn search_paths(
    conn: &Connection,
    raw_query: &str,
    limit: i64,
    offset: i64,
) -> anyhow::Result<Vec<SearchHit>> {
    let q = build_fts_query(raw_query)?;
    let mut stmt = conn
        .prepare(
            "
            SELECT f.id, f.rel_path, f.size, f.mtime_ns, r.path
            FROM fts_files
            JOIN files f ON f.id = fts_files.rowid
            JOIN roots r ON r.id = f.root_id
            WHERE fts_files MATCH ?1
            ORDER BY bm25(fts_files)
            LIMIT ?2 OFFSET ?3
        ",
        )
        .context("prepare search")?;

    let rows = stmt
        .query_map(params![q, limit, offset], |row| {
            let rel: String = row.get(1)?;
            let root: String = row.get(4)?;
            let full_path = Path::new(&root)
                .join(rel.replace('/', std::path::MAIN_SEPARATOR_STR))
                .to_string_lossy()
                .to_string();
            Ok(SearchHit {
                id: row.get(0)?,
                path: rel,
                full_path,
                size: row.get(2)?,
                mtime_ns: row.get(3)?,
                root_path: root,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows)
}
