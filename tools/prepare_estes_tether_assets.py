#!/usr/bin/env python3
"""Build TFM2-ready directional tether sprite frames from the supplied Estes art."""
from pathlib import Path
from PIL import Image
import json
import numpy as np

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "source/estes/tether_art_original.png"
EFFECTS = ROOT / "effects"
SHEET_OUT = EFFECTS / "estes_link_wire#sheet.png"
FANIM_OUT = EFFECTS / "estes_link_wire#anim.fanim"
TILE_OUT = ROOT / "source/estes/tether_art_repeat_tile.png"

# This crop of the source has a naturally matching wave at its edges. Feather
# only the boundary pixels in premultiplied alpha to avoid a visible hard join.
x0, period = 904, 900
y0, y1 = 292, 434
feather = 28


def premul_rgba(image: Image.Image) -> np.ndarray:
    arr = np.asarray(image.convert("RGBA"), dtype=np.float32) / 255.0
    arr[:, :, :3] *= arr[:, :, 3:4]
    return arr


def unpremul_image(arr: np.ndarray) -> Image.Image:
    alpha = arr[:, :, 3:4]
    rgb = np.divide(arr[:, :, :3], alpha, out=np.zeros_like(arr[:, :, :3]), where=alpha > 1e-6)
    rgba = np.concatenate((np.clip(rgb, 0, 1), np.clip(alpha, 0, 1)), axis=2)
    return Image.fromarray(np.round(rgba * 255).astype(np.uint8), "RGBA")


def resize_premul(image: Image.Image, size: tuple[int, int]) -> Image.Image:
    arr = premul_rgba(image)
    channels = []
    for c in range(4):
        plane = Image.fromarray(np.clip(arr[:, :, c] * 255, 0, 255).astype(np.uint8), "L")
        channels.append(np.asarray(plane.resize(size, Image.Resampling.LANCZOS), dtype=np.float32) / 255.0)
    resized = np.stack(channels, axis=2)
    return unpremul_image(resized)


source = Image.open(SOURCE).convert("RGBA")
if source.size != (2172, 724):
    raise SystemExit(f"Unexpected source dimensions: {source.size}")
arr = premul_rgba(source.crop((x0, y0, x0 + period, y1)))
edge = (arr[:, 0, :] + arr[:, -1, :]) * 0.5
for i in range(feather):
    w = 1.0 - i / feather
    arr[:, i, :] = arr[:, i, :] * (1.0 - w) + edge * w
    j = period - 1 - i
    arr[:, j, :] = arr[:, j, :] * (1.0 - w) + edge * w
arr[:, 0, :] = edge
arr[:, -1, :] = edge
tile = unpremul_image(arr)
TILE_OUT.parent.mkdir(parents=True, exist_ok=True)
tile.save(TILE_OUT)

# Match Fanny's known 48x48 directional sprite convention. The original beam
# is downsampled as a thin horizontal band, then rotated into 12 15-degree bins.
strip = resize_premul(tile, (48, 8))
base = Image.new("RGBA", (48, 48), (0, 0, 0, 0))
base.alpha_composite(strip, (0, 20))
sheet = Image.new("RGBA", (48 * 12, 48), (0, 0, 0, 0))
anims = {}
for bucket in range(12):
    angle = bucket * 15
    frame = base.rotate(-angle, resample=Image.Resampling.BICUBIC, expand=False)
    sheet.alpha_composite(frame, (bucket * 48, 0))
    anims[f"wire_{bucket}"] = {"frames": [{"duration": 0.05, "data": {
        "x": bucket * 48, "y": 0, "w": 48, "h": 48
    }}]}
SHEET_OUT.parent.mkdir(parents=True, exist_ok=True)
sheet.save(SHEET_OUT)
FANIM_OUT.write_text(json.dumps({"anims": anims}, separators=(",", ":")) + "\n")
print(f"Wrote {SHEET_OUT} ({sheet.size})")
print(f"Wrote {FANIM_OUT}")
print(f"Wrote {TILE_OUT} ({tile.size})")
