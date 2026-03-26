# Changelog

## Unreleased

### Fixed

- **Scan / FTS:** Replaced invalid FTS5 delete-with-`INSERT` usage with `DELETE FROM fts_files WHERE rowid = ?`, fixing “SQL logic error” during scans when updating indexed files. FTS insert/delete steps now include clearer error context in logs.
- **Open Folder / Explorer:** Search hits include a server-built `fullPath` with correct path separators, so “Open Folder” opens the selected file’s folder and selects the file in File Explorer on Windows instead of misbehaving with mixed `/` and `\` paths.
- **Shell:** Settings are no longer re-fetched on every navigation; branding and onboarding state refresh on launch and when settings change (or after first-run setup).
- **Search results:** Result list height follows the window using measured viewport size; scrolling, lazy loading, and keyboard selection stay correct after resize.
- **Search (a11y):** Result rows use `tabindex="-1"` on `role="option"` and a keyboard handler for Enter/Space (same as click-to-select); listbox declares `aria-multiselectable="false"`.

### Added

- **Search results:** Arrow keys to move the selection (with scrolling), Enter to open the file, Escape to clear selection or move focus to the search field, double-click to open, and a right-click menu (Open File, Open Folder, Copy File Path). Home, End, Page Up, and Page Down are supported in the results list.
- **Vessel name:** Configurable display name in the sidebar (replacing hardcoded “Vessel”), editable in Settings.
- **First-run setup:** When no settings file exists, a short welcome flow collects the vessel name and an optional first folder to index, using the same indexed-locations mechanism as Settings.

### Changed

- **Window:** Main window opens maximized by default (still resizable).
- **Search:** Result ordering uses BM25 plus stable tie-breakers (path length and path name) for more predictable rankings.
- **Result actions:** Clearer messages when a file is missing, inaccessible, or when Explorer or the default app cannot open it; clipboard errors are explained in plain language.
- **First-run setup:** Wording updated for the vessel workflow (no “this computer” phrasing).
- User-facing copy on Search, Activity, Library, and Settings was revised for clearer, less technical wording.
- **Activity:** Section titles use sentence case (“In progress”, “Past runs”) with calmer styling; job progress lines use clearer wording (files checked / added or updated in the index) with formatted numbers.
