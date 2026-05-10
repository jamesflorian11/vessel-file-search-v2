use anyhow::Context;
use rusqlite::{params, Connection};

use crate::dto::SearchHit;
use crate::path_norm::join_root_rel;

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
    let full_path = join_root_rel(&root, &rel);
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
            ORDER BY bm25(fts_files, 5.0, 3.0, 1.0), length(f.rel_path) ASC, f.rel_path COLLATE NOCASE
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

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_search_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open sqlite in-memory");
        conn.execute_batch(
            "
            CREATE TABLE roots (
                id INTEGER PRIMARY KEY,
                path TEXT NOT NULL
            );
            CREATE TABLE files (
                id INTEGER PRIMARY KEY,
                root_id INTEGER NOT NULL,
                rel_path TEXT NOT NULL,
                size INTEGER NOT NULL,
                mtime_ns INTEGER NOT NULL
            );
            CREATE VIRTUAL TABLE fts_files USING fts5(path, full_path, content, tokenize='unicode61');
            ",
        )
        .expect("create schema");
        conn.execute("INSERT INTO roots(id, path) VALUES (1, '/tmp/root')", [])
            .expect("insert root");
        conn.execute(
            "INSERT INTO files(id, root_id, rel_path, size, mtime_ns) VALUES (1, 1, 'docs/logbook.txt', 10, 100)",
            [],
        )
        .expect("insert file one");
        conn.execute(
            "INSERT INTO files(id, root_id, rel_path, size, mtime_ns) VALUES (2, 1, 'reports/engine.pdf', 20, 200)",
            [],
        )
        .expect("insert file two");
        conn.execute(
            "INSERT INTO fts_files(rowid, path, full_path, content) VALUES (1, 'docs/logbook.txt', '/tmp/root/docs/logbook.txt', 'captain daily notes')",
            [],
        )
        .expect("insert fts one");
        conn.execute(
            "INSERT INTO fts_files(rowid, path, full_path, content) VALUES (2, 'reports/engine.pdf', '/tmp/root/reports/engine.pdf', 'engine inspection report')",
            [],
        )
        .expect("insert fts two");
        conn
    }

    #[test]
    fn build_fts_query_quotes_and_ands_terms() {
        let q = build_fts_query("captain report").expect("query should build");
        assert_eq!(q, "\"captain\" AND \"report\"");
    }

    #[test]
    fn extension_like_pattern_handles_dots_and_empty() {
        assert_eq!(extension_like_pattern(Some(".PDF")), "%.pdf");
        assert_eq!(extension_like_pattern(Some("  ")), "");
        assert_eq!(extension_like_pattern(None), "");
    }

    #[test]
    fn search_paths_filters_and_orders_browse_mode() {
        let conn = setup_search_conn();
        let hits = search_paths(&conn, "", 10, 0, Some("pdf"), None, None).expect("browse search");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].path, "reports/engine.pdf");
    }
}
