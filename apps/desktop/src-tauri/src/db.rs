//! SQLite catalog for Vessel File Search.
//!
//! # Invariants
//!
//! - **`files.id`** is stable for the life of the row; pins, tags, policy, and audit reference
//!   `file_id`, not only paths.
//! - **`root_id` + `rel_path`** is canonical; absolute paths are derived for the OS.
//! - **FTS** (`fts_files`) is for search only — not retention or policy decisions.
//! - **Enrichment** belongs in side tables keyed by `file_id`; user-specific rows may use
//!   nullable `actor_profile_id` for future identity/sync.

use anyhow::Context;
use rusqlite::{params, Connection};

use crate::path_norm::join_root_rel;

pub fn open(db_path: &std::path::Path) -> anyhow::Result<Connection> {
    let conn = Connection::open(db_path).with_context(|| format!("open {}", db_path.display()))?;
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA foreign_keys = ON;
        PRAGMA busy_timeout = 5000;
    ",
    )?;
    migrate(&conn)?;
    apply_schema_fixups(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY
        );

        CREATE TABLE IF NOT EXISTS roots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL UNIQUE,
            display_name TEXT,
            enabled INTEGER NOT NULL DEFAULT 1,
            read_only INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS files (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            root_id INTEGER NOT NULL REFERENCES roots(id) ON DELETE CASCADE,
            rel_path TEXT NOT NULL,
            size INTEGER NOT NULL DEFAULT 0,
            mtime_ns INTEGER NOT NULL DEFAULT 0,
            quick_sig TEXT,
            file_state TEXT NOT NULL DEFAULT 'present',
            UNIQUE(root_id, rel_path)
        );

        CREATE INDEX IF NOT EXISTS idx_files_root ON files(root_id);
        CREATE INDEX IF NOT EXISTS idx_files_mtime ON files(mtime_ns);

        CREATE VIRTUAL TABLE IF NOT EXISTS fts_files USING fts5(
            path,
            tokenize = 'unicode61'
        );

        CREATE TABLE IF NOT EXISTS file_hashes (
            file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
            kind TEXT NOT NULL,
            hash BLOB NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (file_id, kind)
        );
    ",
    )?;

    Ok(())
}

fn column_exists(conn: &Connection, table: &str, col: &str) -> rusqlite::Result<bool> {
    let mut stmt = conn.prepare(&format!(
        "SELECT 1 FROM pragma_table_info('{table}') WHERE name = ?1 LIMIT 1"
    ))?;
    let mut rows = stmt.query(rusqlite::params![col])?;
    Ok(rows.next()?.is_some())
}

/// Upgrades databases created before `roots.read_only` existed.
fn ensure_roots_read_only_column(conn: &Connection) -> anyhow::Result<()> {
    if !column_exists(conn, "roots", "read_only")? {
        conn.execute(
            "ALTER TABLE roots ADD COLUMN read_only INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    Ok(())
}

/// Policy-as-data (Phase 5) and append-only audit trail — idempotent `CREATE IF NOT EXISTS`.
fn ensure_governance_tables(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS policy_rule_sets (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            version INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS policy_rules (
            id TEXT PRIMARY KEY,
            rule_set_id TEXT NOT NULL REFERENCES policy_rule_sets(id) ON DELETE CASCADE,
            priority INTEGER NOT NULL DEFAULT 0,
            match_json TEXT NOT NULL,
            action_json TEXT NOT NULL,
            UNIQUE(rule_set_id, priority)
        );

        CREATE INDEX IF NOT EXISTS idx_policy_rules_set ON policy_rules(rule_set_id);

        CREATE TABLE IF NOT EXISTS audit_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            occurred_at TEXT NOT NULL,
            action TEXT NOT NULL,
            actor_profile_id TEXT,
            payload_json TEXT,
            rule_id TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_audit_occurred ON audit_log(occurred_at);
    ",
    )?;
    Ok(())
}

fn apply_schema_fixups(conn: &Connection) -> anyhow::Result<()> {
    ensure_roots_read_only_column(conn)?;
    migrate_drop_legacy_jobs_table(conn)?;
    ensure_indexing_meta_table(conn)?;
    ensure_files_content_columns(conn)?;
    migrate_fts_files_v2(conn)?;
    ensure_governance_tables(conn)?;
    Ok(())
}

/// Extracted searchable text per file (capped during extraction). `content_sig` tracks which
/// `quick_sig` the stored text corresponds to.
fn ensure_files_content_columns(conn: &Connection) -> anyhow::Result<()> {
    if !column_exists(conn, "files", "content_text")? {
        conn.execute("ALTER TABLE files ADD COLUMN content_text TEXT", [])?;
    }
    if !column_exists(conn, "files", "content_sig")? {
        conn.execute("ALTER TABLE files ADD COLUMN content_sig TEXT", [])?;
    }
    Ok(())
}

/// FTS5 cannot be altered: replace legacy single-column `fts_files` with path + full_path + content.
fn migrate_fts_files_v2(conn: &Connection) -> anyhow::Result<()> {
    let sql: Option<String> = match conn.query_row(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name='fts_files'",
        [],
        |row| row.get(0),
    ) {
        Ok(s) => Some(s),
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => return Err(e.into()),
    };
    let Some(sql) = sql else {
        return Ok(());
    };
    if fts_files_has_v2_shape(conn, &sql)? {
        return Ok(());
    }
    conn.execute_batch(
        "
        DROP TABLE IF EXISTS fts_files;
        CREATE VIRTUAL TABLE fts_files USING fts5(
            path,
            full_path,
            content,
            tokenize = 'unicode61'
        );
    ",
    )?;
    backfill_fts_files(conn)?;
    Ok(())
}

fn fts_files_has_v2_shape(conn: &Connection, fts_sql: &str) -> anyhow::Result<bool> {
    if !(fts_sql.contains("path") && fts_sql.contains("full_path") && fts_sql.contains("content")) {
        return Ok(false);
    }
    let mut stmt = conn.prepare("SELECT name FROM pragma_table_info('fts_files')")?;
    let cols: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    let has_path = cols.iter().any(|c| c == "path");
    let has_full_path = cols.iter().any(|c| c == "full_path");
    let has_content = cols.iter().any(|c| c == "content");
    Ok(has_path && has_full_path && has_content)
}

fn backfill_fts_files(conn: &Connection) -> anyhow::Result<()> {
    let mut stmt = conn.prepare(
        "SELECT f.id, f.rel_path, r.path, COALESCE(f.content_text, '')
         FROM files f
         JOIN roots r ON r.id = f.root_id
         WHERE f.file_state = 'present'",
    )?;
    let mut rows = stmt.query([])?;
    let mut insert = conn.prepare(
        "INSERT INTO fts_files(rowid, path, full_path, content) VALUES (?1, ?2, ?3, ?4)",
    )?;
    while let Some(row) = rows.next()? {
        let id: i64 = row.get(0)?;
        let rel: String = row.get(1)?;
        let root: String = row.get(2)?;
        let content: String = row.get(3)?;
        let full = join_root_rel(&root, &rel);
        insert.execute(params![id, rel, full, content])?;
    }
    Ok(())
}

/// Clear stored body text and FTS `content` column (path / full_path columns unchanged).
/// Used when turning content indexing off or on so the next scan does not search stale text.
pub fn clear_all_file_content(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE files SET content_text = NULL, content_sig = NULL",
        [],
    )?;
    let ids: Vec<i64> = {
        let mut stmt = conn.prepare("SELECT id FROM files")?;
        let rows = stmt.query_map([], |row| row.get(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    for id in ids {
        conn.execute(
            "UPDATE fts_files SET content = '' WHERE rowid = ?1",
            [id],
        )?;
    }
    Ok(())
}

/// Legacy installs had a `jobs` table for scan history; it is no longer used.
fn migrate_drop_legacy_jobs_table(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch("DROP TABLE IF EXISTS jobs;")?;
    Ok(())
}

fn ensure_indexing_meta_table(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS indexing_meta (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            last_scan_at TEXT,
            last_scan_status TEXT NOT NULL DEFAULT 'idle',
            last_scan_error TEXT
        );

        INSERT OR IGNORE INTO indexing_meta (id, last_scan_status) VALUES (1, 'idle');
    ",
    )?;
    Ok(())
}

/// Rows in `files` with `file_state = 'present'` (live index size).
pub fn count_present_files(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM files WHERE file_state = 'present'",
        [],
        |row| row.get(0),
    )
}

pub fn read_indexing_meta(conn: &Connection) -> rusqlite::Result<(Option<String>, String, Option<String>)> {
    conn.query_row(
        "SELECT last_scan_at, last_scan_status, last_scan_error FROM indexing_meta WHERE id = 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
}

pub fn write_indexing_meta(
    conn: &Connection,
    last_scan_at: &str,
    last_scan_status: &str,
    last_scan_error: Option<&str>,
) -> rusqlite::Result<usize> {
    conn.execute(
        "UPDATE indexing_meta SET last_scan_at = ?1, last_scan_status = ?2, last_scan_error = ?3 WHERE id = 1",
        params![last_scan_at, last_scan_status, last_scan_error],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fts_v2_shape_detection_requires_all_columns() {
        let conn = Connection::open_in_memory().expect("open sqlite in-memory");
        conn.execute_batch(
            "CREATE VIRTUAL TABLE fts_files USING fts5(path, full_path, content, tokenize='unicode61');",
        )
        .expect("create fts");
        assert!(
            fts_files_has_v2_shape(
                &conn,
                "CREATE VIRTUAL TABLE fts_files USING fts5(path, full_path, content, tokenize='unicode61')"
            )
            .expect("shape check")
        );
    }

    #[test]
    fn fts_v2_shape_detection_rejects_legacy_shape() {
        let conn = Connection::open_in_memory().expect("open sqlite in-memory");
        conn.execute_batch("CREATE VIRTUAL TABLE fts_files USING fts5(path, tokenize='unicode61');")
            .expect("create legacy fts");
        assert!(
            !fts_files_has_v2_shape(
                &conn,
                "CREATE VIRTUAL TABLE fts_files USING fts5(path, tokenize='unicode61')"
            )
            .expect("shape check")
        );
    }
}
