//! Extract searchable plain text from supported file types during scans.
//!
//! - **Stored cap:** [`MAX_STORED_CONTENT_CHARS`] — normalized UTF-8 length after whitespace collapse.
//! - **Read cap:** comes from settings (`content_max_bytes_per_file`); larger files skip body extraction.
//! - **PDF:** wall-clock timeout so pathological files cannot stall the scan thread.
//! - **OCR:** not implemented (images inside PDFs may not contribute text).
//!
//! On failure or unsupported type, the file remains indexed with empty body text and `content_sig`
//! set so the same file is not retried every batch.

use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::Context;
use rusqlite::{params, Connection};

use crate::dto::AppSettings;
use crate::index;
use crate::scan::ScannedFile;

/// Max characters stored per file after normalization (approximate `char` count).
pub const MAX_STORED_CONTENT_CHARS: usize = 512 * 1024;

/// Wall-clock budget for PDF text extraction.
const PDF_EXTRACT_TIMEOUT: Duration = Duration::from_secs(45);

/// Default extensions when `content_index_extensions` is empty (lowercase, no dot).
pub const DEFAULT_CONTENT_EXTENSIONS: &[&str] =
    &["pdf", "txt", "md", "csv", "json", "log"];

pub fn normalized_extension_list(settings: &AppSettings) -> Vec<String> {
    if settings.content_index_extensions.is_empty() {
        return DEFAULT_CONTENT_EXTENSIONS.iter().map(|s| (*s).to_string()).collect();
    }
    settings
        .content_index_extensions
        .iter()
        .map(|s| {
            let t = s.trim().to_lowercase();
            t.strip_prefix('.').unwrap_or(&t).to_string()
        })
        .filter(|s| !s.is_empty())
        .collect()
}

fn quick_sig(size: i64, mtime_ns: i64) -> String {
    format!("{size}:{mtime_ns}")
}

pub fn normalize_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len().min(MAX_STORED_CONTENT_CHARS + 64));
    let mut last_space = false;
    for ch in s.chars() {
        if out.len() >= MAX_STORED_CONTENT_CHARS {
            break;
        }
        if ch.is_whitespace() {
            if !last_space && !out.is_empty() {
                out.push(' ');
                last_space = true;
            }
        } else {
            last_space = false;
            out.push(ch);
        }
    }
    out.trim().to_string()
}

fn ext_matches(path: &Path, allowed: &[String]) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };
    let e = ext.to_lowercase();
    allowed.iter().any(|a| a == &e)
}

/// Read up to `max_read` bytes and decode as UTF-8 (lossy), then normalize.
fn read_plain_text_capped(path: &Path, max_read: u64) -> anyhow::Result<String> {
    let f = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let meta = f.metadata().with_context(|| format!("stat {}", path.display()))?;
    if meta.len() > max_read {
        anyhow::bail!("file exceeds read cap");
    }
    let mut reader = BufReader::new(f);
    let max = max_read as usize;
    let mut buf: Vec<u8> = Vec::new();
    let mut line = Vec::new();
    while buf.len() < max {
        line.clear();
        let n = reader.read_until(b'\n', &mut line)?;
        if n == 0 {
            break;
        }
        let take = (max - buf.len()).min(line.len());
        buf.extend_from_slice(&line[..take]);
    }
    let s = String::from_utf8_lossy(&buf);
    Ok(normalize_text(&s))
}

fn extract_pdf_text(path: &Path) -> anyhow::Result<String> {
    let path = path.to_path_buf();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let r = pdf_extract::extract_text(&path).map_err(|e| anyhow::anyhow!("{e}"));
        let _ = tx.send(r);
    });
    match rx.recv_timeout(PDF_EXTRACT_TIMEOUT) {
        Ok(Ok(s)) => Ok(normalize_text(&s)),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(anyhow::anyhow!("pdf extract timeout")),
    }
}

/// After `apply_one_batch`, fill `content_text` / FTS `content` for rows that need it.
pub fn index_batch_file_contents(
    conn: &mut Connection,
    root_abs: &Path,
    root_id: i64,
    batch: &[ScannedFile],
    settings: &AppSettings,
) -> anyhow::Result<()> {
    if !settings.content_indexing_enabled {
        return Ok(());
    }

    let allowed = normalized_extension_list(settings);
    let max_read = u64::from(settings.content_max_bytes_per_file);

    for f in batch {
        let sig = quick_sig(f.size, f.mtime_ns);

        let (file_id, db_sig, content_sig): (i64, Option<String>, Option<String>) = match conn.query_row(
            "SELECT id, quick_sig, content_sig FROM files WHERE root_id = ?1 AND rel_path = ?2",
            params![root_id, &f.rel_path],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ) {
            Ok(x) => x,
            Err(rusqlite::Error::QueryReturnedNoRows) => continue,
            Err(e) => return Err(e.into()),
        };

        if db_sig.as_deref() != Some(&sig) {
            continue;
        }

        if content_sig.as_deref() == Some(&sig) {
            continue;
        }

        let abs = root_abs.join(f.rel_path.replace('/', std::path::MAIN_SEPARATOR_STR));

        if !ext_matches(&abs, &allowed) {
            index::set_file_content_indexed(conn, file_id, "", &sig)?;
            continue;
        }

        if f.size < 0 || (f.size as u64) > max_read {
            index::set_file_content_indexed(conn, file_id, "", &sig)?;
            continue;
        }

        let ext = abs
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        let text = match ext.as_str() {
            "pdf" => extract_pdf_text(&abs).unwrap_or_default(),
            "txt" | "md" | "log" | "csv" | "json" => {
                read_plain_text_capped(&abs, max_read).unwrap_or_default()
            }
            _ => String::new(),
        };

        index::set_file_content_indexed(conn, file_id, &text, &sig)?;
        thread::yield_now();
    }

    Ok(())
}
