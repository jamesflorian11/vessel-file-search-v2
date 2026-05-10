//! Trim and strip accidental wrapping quotes from user-entered paths.

use std::path::Path;

use crate::dto::AppSettings;

/// Absolute path string for FTS `full_path` and search display (matches `SearchHit.full_path` logic).
pub fn join_root_rel(root: &str, rel_path: &str) -> String {
    Path::new(root)
        .join(rel_path.replace('/', std::path::MAIN_SEPARATOR_STR))
        .to_string_lossy()
        .to_string()
}

pub fn normalize_root_path(s: &str) -> String {
    let mut t = s.trim().to_string();
    for _ in 0..4 {
        let trimmed = t.trim();
        if trimmed.len() < 2 {
            return trimmed.to_string();
        }
        let first = trimmed.chars().next().unwrap();
        let last = trimmed.chars().last().unwrap();
        if (first == '"' || first == '\'') && first == last {
            let inner = &trimmed[first.len_utf8()..trimmed.len() - last.len_utf8()];
            t = inner.trim().to_string();
        } else {
            return trimmed.to_string();
        }
    }
    t
}

/// Returns true if any root path changed.
pub fn normalize_app_settings(settings: &mut AppSettings) -> bool {
    let mut changed = false;
    let vn = settings.vessel_name.trim().to_string();
    if vn != settings.vessel_name {
        settings.vessel_name = vn;
        changed = true;
    }
    for r in &mut settings.roots {
        let n = normalize_root_path(&r.path);
        if n != r.path {
            r.path = n;
            changed = true;
        }
    }
    changed
}
