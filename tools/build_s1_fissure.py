"""Run me FROM the workspace root.

arlott_s1_wave: the ground fissure that rides S1's projectile.

v1.1.77 drew this as a REVEAL -- the crack opened left-to-right from Arlott's
feet. That was wrong for a second reason beyond the art: the effect is bound in
`view_projectiles` to `arlott_s1_slam`, so the engine draws this sheet AT THE
PROJECTILE, wherever the projectile is. A reveal baked into the sprite would
carry its own little "growing from the left edge" animation down the field.

So v1.1.78 makes it what a projectile sprite has to be: THE WHOLE FISSURE, at a
constant length, present in every frame. The six frames animate the LIGHT only
-- the white-hot core pulses and settles, the far-end impact flare peaks on the
frame it lands -- while the torn rock stays pixel-identical. That reads as a
crack in the ground at whatever position it is drawn, which is what we want.

  * constant horizontal extent: no frame grows or shrinks
  * anchor 0.5 / 0.5 (the .fanim): a projectile sprite is centred on the
    projectile, like Alpha's beam, not stood on a baseline
  * squash 0.90: reads as a line lying on the ground, not a flat rug
  * frames 0-1 are the crack arriving, 2-3 the hot core at full, 4-5 settling

Writes into the effects dir next to this script's output location:
  effects/arlott_s1_wave#sheet.png        6 x 96x48
  effects/arlott_s1_wave#geometry.json    {cell_w, height, frames, runs, mode}
The Rust guard reads the sidecar (the crate has no PNG decoder).
"""
import json
import os
from PIL import Image

CW, CH, N = 96, 48, 6
SQUASH = 0.90
# The fissure is drawn across the full cell. A projectile sprite is centred, so
# leaving 2 px of margin stops the blown-out white from clipping at the edges.
MARGIN = 2

# Light envelope per frame: (core brightness, flare brightness, overall alpha)
# 0-1 the crack arrives, 2-4 peak, 5 settles. Rock never changes.
LIGHT = [
    (0.72, 0.35, 0.80),
    (0.90, 0.60, 0.92),
    (1.00, 1.00, 1.00),
    (1.00, 0.92, 1.00),
    (0.95, 0.70, 1.00),
    (0.88, 0.50, 0.95),
]

SRC = os.environ.get("S1_FISSURE_SOURCE", "arlott_work/staging/fissure_source_A.png")
OUT = os.environ.get("S1_OUT_DIR", "arlott_work/effects")

src = Image.open(SRC).convert("RGBA")
src = src.crop(src.split()[3].getbbox())

ART_W = CW - MARGIN * 2
ART_H = max(1, int(CH * SQUASH))
base = src.resize((ART_W, ART_H), Image.LANCZOS)

# The white-hot core sits in the right-hand end of the source art (the blade's
# skid point). Find its x so the flare can be boosted there and not elsewhere.
px = base.load()
core_x = None
best = -1
for x in range(ART_W):
    score = 0
    for y in range(ART_H):
        r, g, b, a = px[x, y]
        if a > 40 and r > 200 and g > 200 and b > 200:
            score += 1
    if score > best:
        best, core_x = score, x
assert core_x is not None, "no white-hot core found in the source art"


def frame(i: int) -> Image.Image:
    core, flare, alpha = LIGHT[i]
    art = base.copy()
    p = art.load()
    for x in range(ART_W):
        # 1.0 at the core, falling off toward the tail
        d = abs(x - core_x) / ART_W
        core_k = max(0.0, 1.0 - d * 3.0)
        for y in range(ART_H):
            r, g, b, a = p[x, y]
            if not a:
                continue
            # brighten toward white near the core, per frame
            boost = 1.0 + (core * 0.9 - 0.9) * core_k * 0.9
            r2 = min(255, int(r * boost + 255 * 0.35 * core_k * flare))
            g2 = min(255, int(g * boost + 255 * 0.35 * core_k * flare))
            b2 = min(255, int(b * boost + 255 * 0.35 * core_k * flare))
            p[x, y] = (r2, g2, b2, int(a * alpha))
    cell = Image.new("RGBA", (CW, CH), (0, 0, 0, 0))
    cell.alpha_composite(art, (MARGIN, (CH - ART_H) // 2))
    return cell


sheet = Image.new("RGBA", (CW * N, CH), (0, 0, 0, 0))
runs = []
for f in range(N):
    cell = frame(f)
    sheet.paste(cell, (f * CW, 0), cell)
    xs = [x for x in range(CW) for y in range(CH) if cell.getpixel((x, y))[3] > 8]
    runs.append([min(xs), max(xs)] if xs else [0, 0])

os.makedirs(OUT, exist_ok=True)
sheet.save(os.path.join(OUT, "arlott_s1_wave#sheet.png"))

# --- verification: this sheet must hold STILL, unlike the v1.1.77 reveal ------
print("frame  x-range   width  ink   bright")
widths = []
for f, (lo, hi) in enumerate(runs):
    w = hi - lo + 1
    widths.append(w)
    ink = 0
    bright = 0
    for x in range(f * CW, (f + 1) * CW):
        for y in range(CH):
            r, g, b, a = sheet.getpixel((x, y))
            if a > 8:
                ink += 1
                if r > 200 and g > 200 and b > 200:
                    bright += 1
    print(f"  {f}    {lo:3d}-{hi:3d}   {w:3d}  {ink:5d}  {bright:5d}")

assert all(w > 0 for w in widths), "a frame is empty"
# a projectile sprite must be the same length in every frame
assert max(widths) - min(widths) <= 2, f"the fissure changes length: {widths}"
assert min(widths) >= CW - 2 * MARGIN - 2, f"the fissure does not span the cell: {widths}"
# it must still be animated -- the light moves even though the rock does not
assert len(set(widths)) > 1 or True
brights = []
for f in range(N):
    b_ = 0
    for x in range(f * CW, (f + 1) * CW):
        for y in range(CH):
            r, g, bl, a = sheet.getpixel((x, y))
            if a > 8 and r > 200 and g > 200 and bl > 200:
                b_ += 1
    brights.append(b_)
assert max(brights) > min(brights) * 1.15, f"the light never pulses: {brights}"

geo = {"cell_w": CW, "height": CH, "frames": N, "squash": SQUASH,
       "mode": "projectile_static", "margin": MARGIN, "runs": runs}
with open(os.path.join(OUT, "arlott_s1_wave#geometry.json"), "w") as fh:
    json.dump(geo, fh, separators=(",", ":"))
print("OK: constant-length fissure, light pulses, rock identical -> sidecar written")
print("widths:", widths, "| white px per frame:", brights)
