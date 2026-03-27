# Phase 5 — Retention / governance

## Policy-as-data

Retention and categorization policies are stored as **versioned rule sets** and **rules** (see SQLite tables `policy_rule_sets`, `policy_rules` in [`db.rs`](../apps/desktop/src-tauri/src/db.rs)). The engine evaluates rules against catalog metadata (paths, tags, `file_id`, etc.) — **not** by embedding logic in FTS.

## Recommendations before automation

1. **Recommendations** — surface candidates with **human-readable rationale** (which rule, which field matched).
2. **Approvals** — explicit user or role sign-off before destructive actions.
3. **Automation** — optional late phase; requires audit and often external records-management integration.

## Append-only audit

The **`audit_log`** table is **append-only by convention**: new rows for each governed action; past rows are not updated in place. `actor_profile_id` may be null for local-only installs.

## Legal / safety

Maritime and safety documentation regimes vary by flag, company, and contract. This product layer **recommends**; organizational records management remains authoritative until formally integrated.
