# Vessel File Search (V2)

Desktop file search for Windows built with **Tauri**, **Svelte**, and **SQLite** (full-text index). You choose folders to index, run scans from the **Search** screen (with a simple indexing status line), and review status in **Settings** if needed.

## First launch

On a new install (no saved settings yet), the app asks for:

- A **vessel name** (shown in the sidebar)
- An optional **folder to index** (added to the same indexed-folder list as in Settings)

You can change the name and folders later under **Settings**.

## Window behavior

The main window starts **maximized** on launch (you can restore or resize it like any desktop app).

## Development

Prerequisites: **Node.js**, **Rust**, and the usual Tauri dependencies for your platform.

From the desktop app package:

```bash
cd apps/desktop
npm install
npm run tauri dev
```

Production build:

```bash
npm run tauri build
```

### Branding assets (logo and app icon)

These are **different files** and are wired separately:

| What | Files | Config / usage |
|------|--------|----------------|
| **In-app logo** (sidebar identity) | [`apps/desktop/src/assets/logo.png`](apps/desktop/src/assets/logo.png) (copied from repo root [`logo.png`](logo.png) when refreshing branding) | Imported only in [`Shell.svelte`](apps/desktop/src/lib/shell/Shell.svelte). Vite emits a **content-hashed** copy under `dist/assets/` on build. The custom title bar shows the app name only (no duplicate logo). |
| **App icon set** (window, taskbar, executable, installers, other targets) | Source PNG: [`apps/desktop/src-tauri/app-icon-source.png`](apps/desktop/src-tauri/app-icon-source.png). Generated output under [`apps/desktop/src-tauri/icons/`](apps/desktop/src-tauri/icons/) (PNG sizes, `icon.ico`, `icon.icns`, MSIX `Square*.png`, iOS, Android, etc.) | Regenerate with **`npm run tauri -- icon src-tauri/app-icon-source.png`** from `apps/desktop` ([Tauri icon](https://v2.tauri.app/develop/icons/)). [`bundle.icon`](apps/desktop/src-tauri/tauri.conf.json) lists the desktop entries: `32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.png`, `icon.icns`, `icon.ico`. Embedded at **Rust compile** / bundle time; do not hand-edit only `icon.ico`. |

**After changing branding:**

- **Logo only:** Updating [`logo.png`](logo.png) at the repo root and copying it to `apps/desktop/src/assets/logo.png` (or editing that file directly) is enough for `tauri dev` in most cases; if the image does not update, restart the Vite dev server (`Ctrl+C` and `npm run tauri dev` again).
- **App icons:** After changing [`app-icon-source.png`](apps/desktop/src-tauri/app-icon-source.png), run **`npm run tauri -- icon src-tauri/app-icon-source.png`** from `apps/desktop`, then restart **`tauri dev`** or run **`npm run tauri build`**. Installers (MSI/NSIS) pick up the same generated assets; Windows may cache the old taskbar icon until the shortcut is recreated or the cache refreshes.
- Do not rely on editing `dist/` by hand; run `npm run build` or `tauri build` so the frontend and icons stay in sync.

## Search

The result list grows with the window. Use arrow keys, **Home** / **End**, and **Page Up** / **Page Down** to move the selection; **Enter** opens the selected file. If a result is stale (file moved or deleted), run **Rescan** from the Search screen to refresh the index.

**Indexing:** There is no scan history list. The app shows a compact **indexing status** (idle / scanning / error), optional last scan time, and how many files are in the index. Scans are a background concern, not a log you maintain.

## Configuration

Settings are stored under the application data directory as `config.json`, alongside the SQLite database used for the index.

## Product strategy (V2)

Phased roadmap, schema invariants, and phase notes live under [`docs/`](docs/):

- [Phase 1 exit criteria](docs/PHASE1_EXIT_CRITERIA.md) — trust, reindex, health UX checklist
- [Schema invariants](docs/SCHEMA_INVARIANTS.md) — `file_id`, roots, side tables, audit
- [Phase 2 file actions](docs/PHASE2_FILE_ACTIONS.md) — read-only roots, SMB/ACL failure UX
- [Phase 3 categorization](docs/PHASE3_CATEGORIZATION.md) — tier sequencing, job types
- [Phase 4 dashboards decision](docs/PHASE4_DASHBOARDS_DECISION.md) — local-first vs org sync
- [Phase 5 governance](docs/PHASE5_GOVERNANCE.md) — policy-as-data, append-only audit

## License

See the project’s license file if one is provided.
