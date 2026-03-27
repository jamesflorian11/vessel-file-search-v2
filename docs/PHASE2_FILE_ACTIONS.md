# Phase 2 — File action workspace (UX and capabilities)

## Capability model: read-only vs writable roots

Each indexed root has a **`read_only` flag** (persisted in app settings and mirrored in SQLite `roots`).

| Intent        | Index / search | Open / reveal | Create / upload / delete |
| ------------- | -------------- | ------------- | ------------------------ |
| Writable root | Yes            | Yes           | Yes (subject to OS ACL) |
| Read-only root | Yes           | Yes           | **Blocked in-app** (Phase 2 UI); show clear message |

Users mark a root read-only when the share is **policy-read**, **archive**, or **they must not accidentally write** there. The app does not probe OS writability on every path (expensive and flaky on SMB); the flag is the **product contract**.

## Failure UX (SMB / ACL / Windows)

When implementing create/upload/edit flows, surface failures explicitly:

- **Access denied** — distinguish “no permission” from “path not found”.
- **Network unavailable** — share offline or drive letter not mapped; suggest checking VPN / mapping.
- **File in use** — another process holds a lock; suggest retry or close the file.
- **Path too long** — Windows `MAX_PATH` / extended path rules; suggest shorter name or deeper root.
- **Partial success** — multi-file operations report per-file errors, not a single silent failure.

## Workspace pattern

Target UX: **selection → actions panel → recent actions history** (implementation deferred to Phase 2 feature work). This document captures the **contract** so Phase 1 settings and schema stay aligned.
