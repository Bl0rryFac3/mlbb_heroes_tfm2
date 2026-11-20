#!/usr/bin/env python3
"""Recompile a Belerick VFX strip at a new on-screen size and placement.

`alpha_work/compile_vfx.py` does the keying and de-spill correctly, but it has
two hard limits that matter for the 2026-11-07 playtest fixes:

  * it resizes the crop to FILL the output cell, so the artwork always ends up
    centred in a canvas the same shape as itself;
  * it hardcodes `anchor_y: 0.5`.

Both of those are wrong for a head marker. The roster's head markers
(`arlott_mark`, `gusion_s1_mark`, `karrie_passive_marks_head_*`) all use a TALL
canvas with `anchor_y: 1.0` and the artwork drawn in the top few pixels, because
`view_buffs` carries no offset field — the offset has to be baked into the
canvas. This tool can do that; `compile_vfx.py` cannot.

Usage:
  recompile_effects.py NAME --frames N --pitch P --cell-h H [--grid G]
      --canvas WxH --max-content M [--place-y Y] [--anchor-y A] [--duration D]
      [--key COLOUR] [--outdir DIR]
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "alpha_work"))
from compile_vfx import key_chroma, parse_key  # noqa: E402

STRIPS = Path(__file__).resolve().parent / "vfx_strips"
OUT = Path(__file__).resolve().parent / "effects"


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("name")
    ap.add_argument("--frames", type=int, required=True)
    ap.add_argument("--pitch", type=int, required=True)
    ap.add_argument("--cell-h", type=int, required=True)
    ap.add_argument("--grid", type=int, help="cells in the source strip (default = frames)")
    ap.add_argument("--canvas", required=True, help="WxH of the output cell")
    ap.add_argument("--max-content", type=int, required=True,
                    help="longest edge of the artwork inside the canvas")
    ap.add_argument("--place-y", type=int,
                    help="y of the artwork CENTRE within the canvas (default: canvas centre)")
    ap.add_argument("--anchor-y", type=float, default=0.5)
    ap.add_argument("--tag", required=True,
                    help="anim tag. MUST match the existing one — the champion data "
                         "resolves effects by tag (taunt, wrath, eruption, ready), not "
                         "by filename, so a renamed tag silently unbinds the effect.")
    ap.add_argument("--duration", type=float, default=0.05)
    ap.add_argument("--key", default="magenta")
    ap.add_argument("--outdir", default=str(OUT))
    a = ap.parse_args()

    grid = a.grid or a.frames
    cw, ch = (int(x) for x in a.canvas.lower().split("x"))
    key = parse_key(a.key)

    src = Image.open(STRIPS / f"{a.name}.png").convert("RGB")
    rgb = np.array(src).astype(float)
    plain, alpha = key_chroma(rgb, key)

    # One shared vertical crop, centred on the sheet's centre of mass, so a
    # generator that drifts the art between frames cannot skew the baseline.
    weights = alpha.sum(axis=1)
    rows = np.arange(len(weights))
    cy = float((weights * rows).sum() / max(weights.sum(), 1e-6))
    y0 = int(round(cy - a.cell_h / 2))
    y0 = max(0, min(y0, rgb.shape[0] - a.cell_h))

    cells = []
    for i in range(grid):
        x0 = i * a.pitch
        c_rgb = plain[y0:y0 + a.cell_h, x0:x0 + a.pitch]
        c_a = alpha[y0:y0 + a.cell_h, x0:x0 + a.pitch]
        rgba = np.dstack([c_rgb, c_a * 255.0]).astype(np.uint8)
        im = Image.fromarray(rgba, "RGBA")
        if im.getbbox() is None:
            print(f"  cell {i}: empty, dropped")
            continue
        cells.append(im)
    if len(cells) < a.frames:
        raise SystemExit(f"only {len(cells)} non-empty cells, need {a.frames}")
    cells = cells[: a.frames]

    # ONE scale for the whole animation, derived from the union bbox across
    # frames. Scaling each frame by its own bbox would normalise them all to the
    # same size and flatten the growth - an eruption that is supposed to bloom
    # would instead sit at constant size and only change shape.
    ux0 = uy0 = 1 << 30
    ux1 = uy1 = -1
    for cell in cells:
        b = cell.getbbox()
        ux0, uy0 = min(ux0, b[0]), min(uy0, b[1])
        ux1, uy1 = max(ux1, b[2]), max(uy1, b[3])
    scale = a.max_content / max(ux1 - ux0, uy1 - uy0)
    ucx, ucy = (ux0 + ux1) / 2.0, (uy0 + uy1) / 2.0
    print(f"  union bbox {ux1-ux0}x{uy1-uy0} at ({ux0},{uy0}) -> uniform scale {scale:.3f}")

    out = Image.new("RGBA", (cw * a.frames, ch), (0, 0, 0, 0))
    place_y = a.place_y if a.place_y is not None else ch // 2
    stats = []
    for i, cell in enumerate(cells):
        sw, sh = max(1, round(cell.width * scale)), max(1, round(cell.height * scale))
        small = cell.resize((sw, sh), Image.LANCZOS)
        # Put the union bbox centre on the target point, so every frame keeps
        # its true size and offset relative to the animation as a whole.
        px = round((cw / 2.0) - ucx * scale)
        py = round(place_y - ucy * scale)
        # The scaled cell is usually larger than the canvas because of the
        # transparent margins around the art - that is fine, we only clip those.
        # What must NOT be clipped is the art itself, so check the bbox.
        b = small.getbbox()
        if b is not None:
            ax0, ay0, ax1, ay1 = b
            if (px + ax0 < 0 or py + ay0 < 0
                    or px + ax1 > cw or py + ay1 > ch):
                raise SystemExit(
                    f"canvas {cw}x{ch} clips the art (bbox {ax1-ax0}x{ay1-ay0} "
                    f"landing at {px+ax0},{py+ay0}) - enlarge the canvas or "
                    f"shrink --max-content")

        sx0, sy0 = max(0, -px), max(0, -py)
        sx1, sy1 = min(sw, cw - px), min(sh, ch - py)
        if sx1 > sx0 and sy1 > sy0:
            piece = small.crop((sx0, sy0, sx1, sy1))
            # dest is in `out` coordinates, so it must include this frame's cell offset.
            out.alpha_composite(piece, (i * cw + px + sx0, py + sy0))
        art = small.crop(b) if b else small
        arr = np.array(out.crop((i * cw, 0, (i + 1) * cw, ch)))
        al = arr[..., 3]
        aw2, ah2 = art.size
        stats.append((i, f"{aw2}x{ah2}", f"y {py}..{py+sh-1}",
                      100.0 * (al > 8).sum() / al.size))

    outdir = Path(a.outdir)
    outdir.mkdir(parents=True, exist_ok=True)
    sheet = outdir / f"{a.name}#sheet.png"
    out.save(sheet)
    anim = {
        "anchor_x": 0.5,
        "anchor_y": a.anchor_y,
        "anims": {a.tag: {
            "frames": [{"duration": a.duration,
                        "data": {"x": i * cw, "y": 0, "w": cw, "h": ch}}
                       for i in range(a.frames)],
        }},
    }
    (outdir / f"{a.name}#anim.fanim").write_text(json.dumps(anim, indent=2))

    print(f"wrote {sheet}  {out.size}  anchor_y={a.anchor_y}")
    for i, size, ys, cov in stats:
        print(f"  frame{i}: art {size} at {ys}, canvas coverage {cov:.1f}%")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
