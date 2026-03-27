---
name: Branding assets from project root
overview: Copy repo-root icon.ico and logo.png into the wired Tauri and Vite asset locations, switch the sidebar to logo.png only, remove redundant title-bar logo, polish sidebar framing, and update README/CHANGELOG.
todos:
  - id: copy-icon
    content: Copy c:/Dev/v2/icon.ico → apps/desktop/src-tauri/icons/icon.ico (replace)
    status: completed
  - id: copy-logo
    content: Copy c:/Dev/v2/logo.png → apps/desktop/src/assets/logo.png; remove unused logo.jpg/logo.svg
    status: completed
  - id: shell-titlebar
    content: Shell.svelte — import logo.png; simplify error fallback (no SVG); TitleBar.svelte — remove logo block
    status: completed
  - id: polish-css
    content: Sidebar .brand-logo — aspect-ratio 1/1, object-fit contain, rounded frame for dark UI
    status: completed
  - id: docs
    content: README.md + CHANGELOG.md — paths, rebuild note for .ico, Windows icon cache
    status: completed
isProject: true
---

# Integrate branding from project root (`icon.ico`, `logo.png`)

## Verified source files (repo root)

| File | Role |
|------|------|
| [`c:/Dev/v2/icon.ico`](c:/Dev/v2/icon.ico) | Windows / bundle app icon (replace Tauri `icons/icon.ico`) |
| [`c:/Dev/v2/logo.png`](c:/Dev/v2/logo.png) | In-app UI logo (replace `src/assets` raster) |

## Audit: current wiring (do not guess)

### App / taskbar / window / installer

- **Config:** [`apps/desktop/src-tauri/tauri.conf.json`](apps/desktop/src-tauri/tauri.conf.json) → `bundle.icon`: `["icons/icon.ico"]`.
- **File embedded at Rust compile:** [`apps/desktop/src-tauri/icons/icon.ico`](apps/desktop/src-tauri/icons/icon.ico).
- **Build hook:** [`apps/desktop/src-tauri/build.rs`](apps/desktop/src-tauri/build.rs) → `tauri_build::build()`.

No separate WiX/NSIS icon file in-repo; Tauri uses `bundle.icon` for the bundled Windows artifact.

### In-app logo (duplicate today)

- [`apps/desktop/src/lib/shell/Shell.svelte`](apps/desktop/src/lib/shell/Shell.svelte) — imports `logo.jpg` + `logo.svg`, sidebar `.brand-logo`.
- [`apps/desktop/src/lib/shell/TitleBar.svelte`](apps/desktop/src/lib/shell/TitleBar.svelte) — **same** imports, `.title-logo` (redundant with sidebar).

### Not used

- [`apps/desktop/index.html`](apps/desktop/index.html) — no favicon.
- Legacy `public/logo.*` — removed per CHANGELOG; no code refs.

### Other files under `src-tauri/icons/`

PNG/icns/android/ios trees are **not** listed in `tauri.conf.json` (only `icon.ico`). Safe to leave as-is or regenerate later with `npm run tauri icon` if cross-platform bundles matter.

---

## Implementation steps

1. **Icon:** Copy **repo root** `icon.ico` over **`apps/desktop/src-tauri/icons/icon.ico`**. Do **not** change `tauri.conf.json` unless switching to a multi-entry icon list intentionally.

2. **Logo:** Copy **repo root** `logo.png` to **`apps/desktop/src/assets/logo.png`**. Remove **`logo.jpg`** and **`logo.svg`** from `src/assets/` once nothing imports them.

3. **Shell.svelte:** Import only `logo.png`. Drop SVG fallback path (or keep a single `onerror` → letter fallback only). Update `logoSrc` / `onLogoError` accordingly.

4. **TitleBar.svelte:** Remove logo `<img>`, imports, and logo-specific state; keep drag region, **“Vessel File Search”** text, window controls.

5. **CSS (sidebar):** On `.brand-logo`, ensure `aspect-ratio: 1 / 1`, fixed width/height (existing 52 / 40 compact), `object-fit: contain`, `object-position: center`, rounded corners; minor tweaks so the PNG reads well on `--bg-elevated` / dark theme.

6. **Docs:** Update [`README.md`](README.md) branding table (paths: `src/assets/logo.png`, `src-tauri/icons/icon.ico`). [`CHANGELOG.md`](CHANGELOG.md) short note.

---

## Runtime / rebuild behavior

| Change | Dev (`tauri dev`) | Production |
|--------|-------------------|------------|
| `logo.png` in `src/assets/` | Vite HMR; restart dev if cached | `npm run build` / `tauri build` |
| `icon.ico` | **Restart `tauri dev`** (Rust rebuild) so `tauri-build` re-embeds | **`npm run tauri build`** for installer/portable |
| Windows shell | Old taskbar shortcut may show cached icon until refreshed / new shortcut | Document in README |

**Full rebuild required for icon:** Yes — **any** change to `.ico` requires recompiling the Rust/Tauri side (restart dev or full `tauri build`).

---

## Deliverables (post-implementation)

- **Files changed:** list in commit (expect: `icon.ico`, `logo.png` under app paths, `Shell.svelte`, `TitleBar.svelte`, deleted old assets, `README.md`, `CHANGELOG.md`).
- **Icon wired from:** `apps/desktop/src-tauri/icons/icon.ico` via `bundle.icon` in `tauri.conf.json` (source of truth at build time: copy from repo root `icon.ico`).
- **Logo wired from:** `apps/desktop/src/assets/logo.png` imported only from `Shell.svelte`.
- **Title bar logo:** removed (redundant).
- **Full rebuild for icon:** confirmed — required for taskbar/exe/installer to pick up new `.ico`.

## Design intent

Separate **icon** (`.ico`, OS chrome) vs **logo** (`logo.png`, sidebar identity only). Minimal, non-duplicative branding.
