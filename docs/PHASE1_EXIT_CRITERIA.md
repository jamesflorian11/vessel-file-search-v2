# Phase 1 — Exit criteria (product checklist)

Use this checklist before treating “core search / access” as complete for V2 foundations.

## Trust

- [ ] **Index truth**: After a successful scan, indexed paths reflect what is on disk within documented limits (network latency, excluded paths).
- [ ] **Stale results**: The UI communicates when results may be stale and how to refresh (rescan from Search).
- [ ] **Errors are actionable**: Scan failures, locked paths, and permission issues surface with enough context to fix (path, root, phase).

## Reindex / recovery

- [ ] **Rescan path**: Users can trigger a full rescan without losing settings or roots configuration.
- [ ] **Indexing status**: A clear idle / scanning / error state; no infinite “running” after restart (in-memory scan state only; last outcome in `indexing_meta`).
- [ ] **Database health**: WAL + migrations apply cleanly on upgrade; no silent schema drift.

## Health UX

- [ ] **Roots**: Clear list of indexed roots, enabled/disabled state, and **read-only vs writable** intent (see Phase 2 capability model).
- [ ] **Progress**: Long scans show progress (files seen, current path, cancel).
- [ ] **Performance**: Search stays responsive on large indexes for typical queries (path FTS + filters).

## Documentation

- [ ] **Limits**: Users know the product indexes **paths** (FTS on path tokens), not full document content, unless/until content indexing ships in a later phase.
