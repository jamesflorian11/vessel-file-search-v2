//! Canonical scan job type label (in-memory workers; no job history table).
//!
//! Phase 3 Tier B will add workers for hash/extract types; keep constants stable to avoid migration.

/// Full index walk (implemented).
pub const SCAN: &str = "scan";

/// Compute content hash for dedup / integrity (reserved; Phase 3 Tier B).
#[allow(dead_code)]
pub const HASH_CONTENT: &str = "hash_content";

/// Extract document properties or selective text (reserved; Phase 3 Tier B).
#[allow(dead_code)]
pub const EXTRACT_METADATA: &str = "extract_metadata";
