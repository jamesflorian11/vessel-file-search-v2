# Vessel File Search (V2)

Desktop file search for Windows built with **Tauri**, **Svelte**, and **SQLite** (full-text index). You choose folders to index, run scans from Activity, then search from the Search screen.

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

## Search

The result list grows with the window. Use arrow keys, **Home** / **End**, and **Page Up** / **Page Down** to move the selection; **Enter** opens the selected file. If a result is stale (file moved or deleted), use **Activity** to run a scan and refresh the index.

## Configuration

Settings are stored under the application data directory as `config.json`, alongside the SQLite database used for the index.

## License

See the project’s license file if one is provided.
