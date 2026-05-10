"""
Write a multi-size .ico where directory entry order is controlled.

Tauri 2 (tauri-codegen) loads `icon_dir.entries()[0]` for `default_window_icon` on Windows
(see CachedIcon::new_ico). That must NOT be a 16×16 bitmap.

Pillow's ICO writer sorts layers by size, so the first entry becomes 16×16 — wrong.

Order used here: 32, 16, 24, 48, 64, 128, 256 — **32×32 first** (matches Tauri icon docs).
Each image is embedded as PNG (RGBA).
"""
from __future__ import annotations

import struct
from io import BytesIO
from pathlib import Path

from PIL import Image

# First entry = what Tauri embeds for WindowBuilder::icon() on Windows
TAURI_FIRST_LAYER_ORDER = (32, 16, 24, 48, 64, 128, 256)


def _png_for_square(master: Image.Image, size: int) -> bytes:
    im = master.resize((size, size), Image.Resampling.LANCZOS).convert("RGBA")
    buf = BytesIO()
    im.save(buf, format="PNG", compress_level=6)
    return buf.getvalue()


def write_ico_png_ordered(out: Path, ordered_pngs: list[tuple[int, bytes]]) -> None:
    """Write ICO from (size, png_bytes) in order — first entry is Tauri's `entries()[0]`."""
    pngs = ordered_pngs
    count = len(pngs)
    header = struct.pack("<HHH", 0, 1, count)
    dir_size = 6 + 16 * count
    offset = dir_size
    dir_entries = bytearray()
    blobs: list[bytes] = []
    for s, png in pngs:
        bw = 0 if s == 256 else s
        bh = 0 if s == 256 else s
        n = len(png)
        dir_entries.extend(struct.pack("<BBBBHHII", bw, bh, 0, 0, 1, 32, n, offset))
        blobs.append(png)
        offset += n

    out.write_bytes(header + bytes(dir_entries) + b"".join(blobs))


def write_ico_layer_order(out: Path, master_rgba: Image.Image, sizes: tuple[int, ...] | None = None) -> None:
    """Downscale master to each size in order and write ICO."""
    order = sizes or TAURI_FIRST_LAYER_ORDER
    mw, mh = master_rgba.size
    if mw != mh:
        raise ValueError("master must be square")
    if max(order) > mw:
        raise ValueError(f"master {mw}px cannot downscale to {max(order)} without upscale")

    pngs: list[tuple[int, bytes]] = []
    for s in order:
        pngs.append((s, _png_for_square(master_rgba, s)))
    write_ico_png_ordered(out, pngs)


def load_master_square(path: Path) -> Image.Image:
    im = Image.open(path)
    im.load()
    im = im.convert("RGBA")
    w, h = im.size
    if w != h:
        raise ValueError("source must be square")
    return im
