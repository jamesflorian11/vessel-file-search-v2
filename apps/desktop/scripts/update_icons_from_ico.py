"""
Regenerate apps/desktop/src-tauri/icons raster assets from a master .ico file.
Preserves filenames and directory layout; overwrites image bytes only.

Source of truth: src-tauri/icons/icon.ico (pass path as argv[1] or use that default).

If icons/ is missing most files (e.g. only icon.ico remains), first rebuild the tree:
  1) Export a 1024 PNG from the ICO (Lanczos upscale).
  2) npm run tauri -- icon <that.png>   (from apps/desktop)
  3) Restore the original icon.ico bytes if the CLI overwrote them.
  4) Run this script so every PNG + icon.icns matches icon.ico exactly.
"""
from __future__ import annotations

import shutil
import sys
from pathlib import Path

from PIL import Image

ICONS_DIR = Path(__file__).resolve().parents[1] / "src-tauri" / "icons"


def load_master_rgba(ico_path: Path) -> Image.Image:
    im = Image.open(ico_path)
    im.load()
    return im.convert("RGBA")


def main() -> int:
    default_ico = ICONS_DIR / "icon.ico"
    src_ico = Path(sys.argv[1] if len(sys.argv) > 1 else str(default_ico)).resolve()
    if not src_ico.is_file():
        print(f"Missing source file: {src_ico}", file=sys.stderr)
        return 1

    dest_ico = ICONS_DIR / "icon.ico"
    if src_ico.resolve() != dest_ico.resolve():
        shutil.copy2(src_ico, dest_ico)
    master = load_master_rgba(dest_ico)

    png_paths = sorted(ICONS_DIR.rglob("*.png"))
    for png_path in png_paths:
        with Image.open(png_path) as old:
            w, h = old.size
        if w < 1 or h < 1:
            continue
        out = master.resize((w, h), Image.Resampling.LANCZOS)
        out.save(png_path, format="PNG")

    # macOS bundle icon: single ICNS; upscale from ICO master for best available quality
    icns_path = ICONS_DIR / "icon.icns"
    hi = master.resize((1024, 1024), Image.Resampling.LANCZOS)
    hi.save(icns_path, format="ICNS")

    print(f"OK: icon.ico copied; {len(png_paths)} PNGs updated; icon.icns written")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
