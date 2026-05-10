"""
Rebuild src-tauri/icons from icons/icon.ico using small-size–optimized pipeline.

ICO layer order uses ico_write_tauri_windows.TAURI_FIRST_LAYER_ORDER (32×32 first) so Tauri’s
Windows `default_window_icon` (entries()[0]) stays correct — do not use Pillow’s ICO save.
"""
from __future__ import annotations

import sys
from io import BytesIO
from pathlib import Path

from PIL import Image

_SCRIPT_DIR = Path(__file__).resolve().parent
if str(_SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPT_DIR))
from icon_optimize import optimize_size
from ico_write_tauri_windows import TAURI_FIRST_LAYER_ORDER, write_ico_png_ordered

ICONS_DIR = Path(__file__).resolve().parents[1] / "src-tauri" / "icons"


def load_master_from_ico(ico_path: Path) -> Image.Image:
    im = Image.open(ico_path)
    im.load()
    return im.convert("RGBA")


def build_ico(ico_path: Path, master: Image.Image) -> None:
    ordered: list[tuple[int, bytes]] = []
    for s in TAURI_FIRST_LAYER_ORDER:
        im = optimize_size(master, s)
        buf = BytesIO()
        im.save(buf, format="PNG", compress_level=6)
        ordered.append((s, buf.getvalue()))
    write_ico_png_ordered(ico_path, ordered)


def rebuild_pngs_and_icns(master: Image.Image) -> int:
    n = 0
    for png_path in sorted(ICONS_DIR.rglob("*.png")):
        with Image.open(png_path) as old:
            w, h = old.size
        if w < 1 or h < 1 or w != h:
            continue
        out = optimize_size(master, w)
        out.save(png_path, format="PNG")
        n += 1

    icns_path = ICONS_DIR / "icon.icns"
    hi = optimize_size(master, 1024)
    hi.save(icns_path, format="ICNS")
    return n


def main() -> int:
    ico_path = ICONS_DIR / "icon.ico"
    if not ico_path.is_file():
        print(f"Missing {ico_path}", file=sys.stderr)
        return 1

    master = load_master_from_ico(ico_path)
    mw, mh = master.size
    if mw < 256 or mh < 256:
        master = master.resize((256, 256), Image.Resampling.LANCZOS)
    elif mw != 256 or mh != 256:
        master = master.resize((256, 256), Image.Resampling.LANCZOS)

    build_ico(ico_path, master)
    n = rebuild_pngs_and_icns(master)
    print(f"OK: icon.ico layers {len(TAURI_FIRST_LAYER_ORDER)}; {n} PNGs + icon.icns updated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
