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

/// `None` or empty → no filter (`""` bind). Otherwise `%.ext` for SQL `LIKE`.
pub fn extension_like_pattern(extension_filter: Option<&str>) -> String {
    let Some(raw) = extension_filter.map(str::trim) else {
        return String::new();
    };
    if raw.is_empty() {
        return String::new();
    }
    let e = raw.strip_prefix('.').unwrap_or(raw).to_lowercase();
    format!("%.{e}")
}

fn row_to_hit(row: &rusqlite::Row<'_>) -> rusqlite::Result<SearchHit> {
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
}

/// Empty `raw_query` lists indexed files (browse). Non-empty uses FTS5 `MATCH`.
pub fn search_paths(
    conn: &Connection,
    raw_query: &str,
    limit: i64,
    offset: i64,
    extension_filter: Option<&str>,
    modified_from_ns: Option<i64>,
    modified_to_ns: Option<i64>,
) -> anyhow::Result<Vec<SearchHit>> {
    let ext_pat = extension_like_pattern(extension_filter);
    let q = raw_query.trim();

    if q.is_empty() {
        let mut stmt = conn
            .prepare(
                "
                SELECT f.id, f.rel_path, f.size, f.mtime_ns, r.path
                FROM files f
                JOIN roots r ON r.id = f.root_id
                WHERE (?1 = '' OR lower(f.rel_path) LIKE ?1)
                  AND (?2 IS NULL OR f.mtime_ns >= ?2)
                  AND (?3 IS NULL OR f.mtime_ns <= ?3)
                ORDER BY f.mtime_ns DESC, f.rel_path COLLATE NOCASE
                LIMIT ?4 OFFSET ?5
            ",
            )
            .context("prepare browse")?;

        let rows = stmt
            .query_map(
                params![
                    ext_pat.as_str(),
                    modified_from_ns,
                    modified_to_ns,
                    limit,
                    offset
                ],
                row_to_hit,
            )?
            .collect::<Result<Vec<_>, _>>()?;

        return Ok(rows);
    }

    let fts_q = build_fts_query(q)?;
    let mut stmt = conn
        .prepare(
            "
            SELECT f.id, f.rel_path, f.size, f.mtime_ns, r.path
            FROM fts_files
            JOIN files f ON f.id = fts_files.rowid
            JOIN roots r ON r.id = f.root_id
            WHERE fts_files MATCH ?1
              AND (?2 = '' OR lower(f.rel_path) LIKE ?2)
              AND (?3 IS NULL OR f.mtime_ns >= ?3)
              AND (?4 IS NULL OR f.mtime_ns <= ?4)
            ORDER BY bm25(fts_files), length(f.rel_path) ASC, f.rel_path COLLATE NOCASE
            LIMIT ?5 OFFSET ?6
        ",
        )
        .context("prepare search")?;

    let rows = stmt
        .query_map(
            params![
                fts_q,
                ext_pat.as_str(),
                modified_from_ns,
                modified_to_ns,
                limit,
                offset
            ],
            row_to_hit,
        )?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows)
}
