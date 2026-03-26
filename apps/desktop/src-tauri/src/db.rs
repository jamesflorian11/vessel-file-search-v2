use anyhow::Context;
use rusqlite::Connection;

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
            enabled INTEGER NOT NULL DEFAULT 1
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

        CREATE TABLE IF NOT EXISTS jobs (
            id TEXT PRIMARY KEY,
            type TEXT NOT NULL,
            status TEXT NOT NULL,
            payload_json TEXT,
            progress_json TEXT,
            error TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
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
