# Phase 4 — Favorites / dashboards: product decision

## Decision (for planning)

**Default: local-first profiles and dashboards.**

- **Pins, saved queries, and layout** are stored **per install** (and may later be scoped to **Windows user profile** or an in-app **named profile**).
- **No central sync** is required for the first dashboards release.

## When to add org-wide sync

Introduce a **thin backend** or existing **fleet / MDM** integration only when product requirements include:

- Identical pinned sets across **many PCs** without manual export/import, or
- **Mandatory** org-curated landing pages for roles, or
- **Audit** requirements that dashboards were shown or acknowledged.

## Schema implication

User-specific rows should carry a nullable **`actor_profile_id`** (or equivalent) so a future sync service can merge **metadata** without merging the full FTS index (indexes stay local per device).
