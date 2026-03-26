# Changelog

## Unreleased

### Fixed

- **Open Folder / Explorer:** Search hits now include a server-built `fullPath` with correct path separators, so “Open Folder” opens the selected file’s folder and selects the file in File Explorer on Windows instead of misbehaving with mixed `/` and `\` paths.

### Added

- **Search results:** Arrow keys to move the selection (with scrolling), Enter to open the file, Escape to clear selection or move focus to the search field, double-click to open, and a right-click menu (Open File, Open Folder, Copy File Path).
- **Vessel name:** Configurable display name in the sidebar (replacing hardcoded “Vessel”), editable in Settings.
- **First-run setup:** When no settings file exists, a short welcome flow collects the vessel name and an optional first folder to index, using the same indexed-locations mechanism as Settings.

### Changed

- User-facing copy on Search, Activity, Library, and Settings was revised for clearer, less technical wording.
