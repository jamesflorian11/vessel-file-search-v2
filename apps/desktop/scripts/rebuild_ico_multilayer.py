"""
Rebuild src-tauri/icons/icon.ico as a true multi-resolution ICO with Tauri-correct
layer order (32×32 FIRST — see ico_write_tauri_windows.py).

Master raster (highest quality, no upscaling):
  1) icons/icon.png if present (expected 512×512)
  2) else largest bitmap extracted from icons/icon.ico

Embedded sizes: 32, 16, 24, 48, 64, 128, 256 (order matters for Tauri Windows runtime).
"""
from __future__ import annotations

import struct
import sys
from io import BytesIO
from pathlib import Path

from PIL import Image

from ico_write_tauri_windows import TAURI_FIRST_LAYER_ORDER, load_master_square, write_ico_layer_order

ICONS_DIR = Path(__file__).resolve().parents[1] / "src-tauri" / "icons"
ICO_PATH = ICONS_DIR / "icon.ico"
PNG_MASTER = ICONS_DIR / "icon.png"

SIZES_SET = frozenset(TAURI_FIRST_LAYER_ORDER)


def largest_bitmap_from_ico(data: bytes) -> Image.Image:
    """Load largest PNG/DIB blob from an ICO file."""
    n = struct.unpack_from("<H", data, 4)[0]
    best: tuple[int, int, bytes] | None = None
    off = 6
    for _ in range(n):
        w, h = data[off], data[off + 1]
        w = 256 if w == 0 else w
        h = 256 if h == 0 else h
        bi = struct.unpack_from("<I", data, off + 8)[0]
        io = struct.unpack_from("<I", data, off + 12)[0]
        blob = data[io : io + bi]
        area = w * h
        if best is None or area > best[0]:
            best = (area, w, blob)
        off += 16
    assert best is not None
    im = Image.open(BytesIO(best[2]))
    im.load()
    return im.convert("RGBA")


def load_master_rgba() -> Image.Image:
    if PNG_MASTER.is_file():
        return load_master_square(PNG_MASTER)
    data = ICO_PATH.read_bytes()
    return largest_bitmap_from_ico(data)


def list_ico_sizes(path: Path) -> list[tuple[int, int]]:
    d = path.read_bytes()
    n = struct.unpack_from("<H", d, 4)[0]
    out: list[tuple[int, int]] = []
    off = 6
    for _ in range(n):
        w, h = d[off], d[off + 1]
        w = 256 if w == 0 else w
        h = 256 if h == 0 else h
        out.append((w, h))
        off += 16
    return out


def main() -> int:
    master = load_master_rgba()
    print(f"Master: {master.size[0]}×{master.size[1]} RGBA (no upscale used)")
    write_ico_layer_order(ICO_PATH, master, TAURI_FIRST_LAYER_ORDER)
    found = list_ico_sizes(ICO_PATH)
    print(f"Embedded layers ({len(found)}), file order:", ", ".join(f"{w}×{h}" for w, h in found))
    if found[0] != (32, 32):
        print("ERROR: first ICO entry must be 32×32 for Tauri Windows window icon", file=sys.stderr)
        return 1
    if set(found) != {(s, s) for s in SIZES_SET}:
        print("WARNING: embedded set != expected", file=sys.stderr)
        return 1
    print("OK: icon.ico replaced; 32×32 is entries()[0] for Tauri CachedIcon")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
