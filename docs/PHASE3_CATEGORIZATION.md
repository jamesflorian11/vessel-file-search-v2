# Phase 3 — Smart categorization (sequencing)

Ship value in **tiers**; do not jump to ML before cheaper signals work.

## Tier A — Rules and metadata (first)

- Path segments, file name patterns, extensions.
- Manual tags and labels (side tables keyed by `file_id`).
- Optional sidecar or embedded metadata where available without full content indexing.

**Exit**: Users get stable, explainable groupings.

## Tier B — Content signals (selective)

- **Hashes** — use `file_hashes` (see `job_types::HASH_*` in Rust) for dedup and “same bytes” edges.
- **Selective text / property extraction** — job-driven, never blocking basic search; respect privacy and policy.

**Prerequisite**: Tier A tagging so evaluation is possible.

## Tier C — Learned grouping (last)

- Embeddings, clustering, similarity graphs — only after Tier A/B provide labels and governance alignment.

## Planned job types (Rust)

See [`apps/desktop/src-tauri/src/job_types.rs`](../apps/desktop/src-tauri/src/job_types.rs) for canonical `jobs.type` string constants used by the job pipeline (`scan` today; hash/extract reserved for Tier B).
