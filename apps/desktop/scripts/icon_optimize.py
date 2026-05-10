"""
Small-size–friendly raster passes for Windows ICO / PNG icons.
Master should be RGBA (e.g. 256×256 from icon.ico).
"""
from __future__ import annotations

from PIL import Image, ImageEnhance, ImageFilter


def optimize_size(master_rgba: Image.Image, size: int) -> Image.Image:
    """Resize from master with tweaks that read better at taskbar / list sizes."""
    s = size
    im = master_rgba.resize((s, s), Image.Resampling.LANCZOS)

    if s >= 512:
        im = ImageEnhance.Sharpness(im).enhance(1.08)
        return im

    if s <= 256:
        im = ImageEnhance.Contrast(im).enhance(1.08)

    if s <= 128:
        im = ImageEnhance.Sharpness(im).enhance(1.22)

    if s <= 48:
        im = ImageEnhance.Sharpness(im).enhance(1.12)
        # Thicken silhouette slightly so edges survive 16–32 px (reduces muddy fringe)
        r, g, b, a = im.split()
        a = a.filter(ImageFilter.MaxFilter(3))
        im = Image.merge("RGBA", (r, g, b, a))

    if s <= 32:
        im = ImageEnhance.Sharpness(im).enhance(1.15)
        im = ImageEnhance.Contrast(im).enhance(1.05)

    if s <= 24:
        im = ImageEnhance.Sharpness(im).enhance(1.08)

    return im
