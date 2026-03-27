# Schema invariants (preserve for V2+)

These conventions keep favorites, dashboards, categorization, and governance from requiring painful rewrites.

## Identity and scope

1. **`files.id` is stable**  
   Once assigned, an ID identifies a row in the local catalog. Favorites, pins, policy evaluations, and audit rows should reference **`file_id`**, not only string paths.

2. **`root_id` + `rel_path` is canonical**  
   Absolute paths are derived for display and OS calls. Moving a share mount may change the absolute path without changing `root_id` + `rel_path` semantics.

3. **`roots` define scope**  
   Departments and shares map naturally to roots. Policy and dashboards should filter by `root_id` when needed.

4. **`file_state` and reconcile**  
   The catalog may retain tombstones or “not seen this scan” states; search and governance must respect that instead of pretending missing files still exist.

## Enrichment (future tables)

5. **Side tables keyed by `file_id`**  
   Tags, labels, extracts, embeddings, and policy evaluation results live in **separate tables** with foreign keys to `files(id)`, not opaque JSON blobs in `files` unless strictly internal.

6. **`profile_id` / `principal_id` (nullable)**  
   New user-specific or role-specific rows should include a nullable **`actor_profile_id`** (or equivalent) from the start so local single-user mode and future SSO both fit without renaming columns.

## Jobs

7. **Heavy work is asynchronous**  
   Hashing, content extraction, retention scans, and ML-style enrichment run as **job types** with progress and cancellation—not blocking the UI thread or a single giant transaction.

## Governance

8. **Policy is data**  
   Retention and categorization rules are stored as versioned **rule sets** and rows, not scattered hardcoded conditions.

9. **Audit is append-only**  
   Destructive or sensitive actions append to **`audit_log`** (no in-place edits of past audit rows).

## Full-text search

10. **FTS is for query, not policy**  
    FTS tables rank and match; they do not encode retention legality or deletion approval.
