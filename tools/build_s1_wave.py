"""Run me FROM the workspace root.

arlott_s1_wave: forward ground wave, built FROM the original artwork.

Hand-drawing the wave looked worse than the art it replaced, so this keeps the
shipped crimson art and fixes only what the bug report complained about:

  * the old crescent spanned the whole 96 px cell in every frame, so it read as a
    sideways slash. Here each frame is the same art horizontally compressed to a
    finite width (26 -> 54 px) whose leading lip sits at a front position that
    walks forward across the 6 frames.
  * the tail behind the lip is alpha-faded, so the shape reads as travelling
    forward instead of being a bar.
  * the lip position never leaves the cell, and no frame touches both edges.

Because the source is the original sheet, the palette, the pixel density and the
hot-white leading edge are exactly what the kit already looked like.
"""
import os
from PIL import Image

CW, CH, N = 96, 48, 6
OLD = Image.open(os.environ.get('S1_WAVE_SOURCE', 'arlott_work/effects/arlott_s1_wave#sheet.png')).convert('RGBA')

SPANS = [(34, 20), (44, 30), (54, 42), (62, 54), (68, 66), (72, 78)]  # (width, lip_x)

sheet = Image.new('RGBA', (CW * N, CH), (0, 0, 0, 0))

for f, (w, lip) in enumerate(SPANS):
    cell = Image.new('RGBA', (CW, CH), (0, 0, 0, 0))
    # one source frame, squeezed horizontally to w px, full height
    src = OLD.crop((f * CW, 0, (f + 1) * CW, CH))
    # squash vertically so it hugs the ground like a shockwave, not a slash
    hh = int(CH * 0.86)
    squeezed = src.resize((w, hh), Image.LANCZOS)
    canvas = Image.new('RGBA', (w, CH), (0, 0, 0, 0))
    canvas.alpha_composite(squeezed, (0, CH - hh))     # anchor to the ground
    squeezed = canvas
    # fade the tail (left) so the wave has a direction
    px = squeezed.load()
    for x in range(w):
        u = x / max(1, w - 1)                 # 0 = tail, 1 = lip
        keep = min(1.0, 0.15 + 1.25 * u ** 1.4)
        for y in range(CH):
            r, g, b, a = px[x, y]
            if a:
                px[x, y] = (r, g, b, int(a * keep))
    cell.alpha_composite(squeezed, (max(0, lip - w + 1), 0))
    sheet.paste(cell, (f * CW, 0), cell)

sheet.save('arlott_work/effects/arlott_s1_wave#sheet.png')

# --- verification: direction, travel and edge hygiene, measured not assumed
print("frame  ink   x-range      height")
prev_right = None
moves_forward = True
touches_both = False
for f in range(N):
    xs, ys = [], []
    for x in range(f * CW, (f + 1) * CW):
        for y in range(CH):
            if sheet.getpixel((x, y))[3] > 40:
                xs.append(x - f * CW); ys.append(y)
    l, r = min(xs), max(xs)
    print(f"  {f}   {len(xs):4d}  {l:2d}..{r:2d}       {max(ys)-min(ys)+1:2d}")
    if l == 0 and r == CW - 1:
        touches_both = True
    if prev_right is not None and r < prev_right:
        moves_forward = False
    prev_right = r
print("\ntouches both cell edges in any frame:", touches_both)
print("leading edge advances every frame:", moves_forward)
assert not touches_both, "the wave spans the whole cell - that is the original bug"
assert moves_forward, "the wave must travel forward frame to frame"
print("OK: wave travels forward and never spans the cell")

# record the geometry the test verifies, so the guard sees what was drawn
import json
runs = []
for f in range(N):
    xs = [x for x in range(f*CW, (f+1)*CW)
          if any(sheet.getpixel((x, y))[3] > 40 for y in range(CH))]
    runs.append([min(xs) - f*CW, max(xs) - f*CW])
json.dump({"cell_w": CW, "height": CH, "frames": N, "runs": runs},
          open('arlott_work/effects/arlott_s1_wave#geometry.json', 'w'), indent=1)
print("sidecar:", runs)
