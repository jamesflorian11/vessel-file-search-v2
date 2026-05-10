# Changelog

## Unreleased

### Added

- **Search results:** Each hit shows a **primary** display label (filename-based cleanup and a few safe path-aware titles) and a **secondary** condensed path for scanning long locations. **Open File**, **Open Folder**, and **Copy File Path** still use the real `fullPath` from the index—no renaming or path changes on disk. Helpers: [`condensePath.ts`](apps/desktop/src/lib/search/condensePath.ts), [`displayLabel.ts`](apps/desktop/src/lib/search/displayLabel.ts).

### Fixed

- **Settings:** The “Limit indexing to extensions” text field no longer crashes on focus (it used `contentIndexExtensions.join()` when the array could be missing from loaded settings). The field now binds to a dedicated string, normalizes settings on load, and parses extensions safely.
- **Branding:** In-app logo remains [`apps/desktop/src/assets/logo.png`](apps/desktop/src/assets/logo.png) (optional sync from repo root [`logo.png`](logo.png)). **App icons** are regenerated from [`apps/desktop/src-tauri/app-icon-source.png`](apps/desktop/src-tauri/app-icon-source.png) via `npm run tauri -- icon src-tauri/app-icon-source.png` into [`apps/desktop/src-tauri/icons/`](apps/desktop/src-tauri/icons/); `bundle.icon` in `tauri.conf.json` now lists the full desktop set (`32x32`, `128x128`, `128x128@2x`, `icon.png`, `icon.icns`, `icon.ico`). Rebuild the app after icon changes (`tauri dev` restart or `tauri build`); Windows may cache the old taskbar icon.
- **Scan / FTS:** Replaced invalid FTS5 delete-with-`INSERT` usage with `DELETE FROM fts_files WHERE rowid = ?`, fixing “SQL logic error” during scans when updating indexed files. FTS insert/delete steps now include clearer error context in logs.
- **Open Folder / Explorer:** Search hits include a server-built `fullPath` with correct path separators, so “Open Folder” opens the selected file’s folder and selects the file in File Explorer on Windows instead of misbehaving with mixed `/` and `\` paths.
- **Shell:** Settings are no longer re-fetched on every navigation; branding and onboarding state refresh on launch and when settings change (or after first-run setup).
- **Search results:** Result list height follows the window using measured viewport size; scrolling, lazy loading, and keyboard selection stay correct after resize.
- **Search (a11y):** Result rows use `tabindex="-1"` on `role="option"` and a keyboard handler for Enter/Space (same as click-to-select); listbox declares `aria-multiselectable="false"`.

### Added

- **Content-aware search (optional):** Settings include **Index file text for search** (default off), optional extension allowlist, and max bytes read per file. On scan, supported types (`.pdf`, `.txt`, `.md`, `.csv`, `.json`, `.log` by default) are read with caps and normalization; extracted text is stored in SQLite (`files.content_text`) and indexed in FTS5 alongside relative path and full path. Search ranks path matches above body text (BM25 column weights). Toggling the setting clears stored text so a rescan is needed. New Rust dependency: **`pdf-extract`** for PDF text (no OCR).
- **Database / migration:** `files` gains `content_text` and `content_sig`; `fts_files` is recreated with `path`, `full_path`, and `content` columns and backfilled on upgrade (no manual DB reset).
- **Search results:** Arrow keys to move the selection (with scrolling), Enter to open the file, Escape to clear selection or move focus to the search field, double-click to open, and a right-click menu (Open File, Open Folder, Copy File Path). Home, End, Page Up, and Page Down are supported in the results list.
- **Vessel name:** Configurable display name in the sidebar (replacing hardcoded “Vessel”), editable in Settings.
- **First-run setup:** When no settings file exists, a short welcome flow collects the vessel name and an optional first folder to index, using the same indexed-locations mechanism as Settings.

### Changed

- **Window:** Main window opens maximized by default (still resizable).
- **Search:** Result ordering uses BM25 plus stable tie-breakers (path length and path name) for more predictable rankings. Non-empty queries use weighted BM25 across indexed path, full path, and body columns when the FTS schema includes body text.
- **Result actions:** Clearer messages when a file is missing, inaccessible, or when Explorer or the default app cannot open it; clipboard errors are explained in plain language.
- **First-run setup:** Wording updated for the vessel workflow (no “this computer” phrasing).
- User-facing copy on Search, Library, and Settings was revised for clearer, less technical wording.
- **Indexing UX:** Removed indexing **job history** (no “Past runs” list, no “Clear history”, no persisted scan job rows). Replaced with a minimal **indexing status** (idle / scanning / error), optional last scan time, and live file count via `get_indexing_status`. The legacy `jobs` SQLite table is dropped on upgrade; last scan outcome is stored in `indexing_meta` only.
