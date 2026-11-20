#!/usr/bin/env python3
"""
Build Gord's travelling basic-attack bolt from the basic-hit sheet.

    python3 gord_work/build_basic_bolt.py [--size N] [--rotate]

Why this script exists
----------------------
The bolt did not have one. In rc.1 it was derived ad hoc from
`gord_basic_hit` (48x48, 9 frames) — scaled to 34 and rotated 180 — with no
script, so the transformation was unreproducible and, as it turned out,
wrong. This makes it explicit and re-runnable.

THE ORIENTATION FIX (2026-11-06)
--------------------------------
rc.1 rotated the bolt 180 degrees. That flip was made blind: PROJECT_CONTEXT
records "I cannot see the art ... The 180 flip implements the owner's report
literally". The owner then played rc.1 and reported the tail was STILL facing
the enemy. Shown both orientations side by side at 22 px, he chose the
UN-FLIPPED one. So the flip is reverted and must not be re-applied.

    --rotate is OFF by default. Passing it re-introduces the rc.1 bug.

WHY ONE SPRITE AND NOT EIGHT
----------------------------
The owner asked for "different rotation sprites ... coding it where he's
aiming". That is not needed and would require a DLL rebuild. No hero in the
roster ships directional projectile variants — zhask, estes, obsidia, karrie
and hanabi all fire through 360 degrees with a single sprite each — so the
engine rotates the projectile to face its travel direction itself. One
correctly-oriented sprite therefore gets the tail right in every direction.
If it ever looks wrong again, the fix is this sprite's forward axis, not
eight variants.

WHY 22
------
Owner's pick (was 48, then 34). 22 sits in the roster's basic-projectile band
(zhask 32x12, estes 32x16, gusion 36x15, karrie 40x22).

CAVEAT
------
The source is an impact burst, not a purpose-drawn bolt: it fills its 48x48
cell (~50% coverage) and has no clean head/tail. Scaling it down makes it read
as a compact projectile, but if it still reads as a burst in game, the real fix
is new directional art, not another flip.
"""
from __future__ import annotations

import argparse
import json
import pathlib

import numpy as np
from PIL import Image

BUILD = pathlib.Path("build_v2081/effects")
WORK = pathlib.Path("gord_work/effects")

SRC_NAME = "gord_basic_hit"
OUT_NAME = "gord_basic_bolt"
SRC_CELL = 48          # gord_basic_hit is 48x48
FRAMES = 9
DURATION = 0.04        # matches the shipped bolt's frame duration
ANIM_KEY = "bolt"      # must match the `tag` in view_projectiles


def build(size: int, rotate: bool) -> tuple[Image.Image, dict]:
    src = Image.open(BUILD / f"{SRC_NAME}#sheet.png").convert("RGBA")
    arr = np.array(src)

    cells = []
    for i in range(FRAMES):
        cell = Image.fromarray(arr[:, i * SRC_CELL:(i + 1) * SRC_CELL, :])
        # LANCZOS: the owner's standing rule is frames are never re-scaled
        # casually, but this is a genuine 48 -> 22 downscale of delivered art,
        # and nearest would alias the glow into hard speckle.
        cell = cell.resize((size, size), Image.LANCZOS)
        if rotate:
            cell = cell.transpose(Image.ROTATE_180)
        cells.append(cell)

    sheet = Image.new("RGBA", (size * FRAMES, size), (0, 0, 0, 0))
    for i, c in enumerate(cells):
        sheet.paste(c, (i * size, 0), c)

    fanim = {
        "anchor_x": 0.5,
        "anchor_y": 0.5,
        "anims": {
            ANIM_KEY: {
                "frames": [
                    {"duration": DURATION,
                     "data": {"x": i * size, "y": 0, "w": size, "h": size}}
                    for i in range(FRAMES)
                ]
            }
        },
    }
    return sheet, fanim


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--size", type=int, default=22)
    ap.add_argument("--rotate", action="store_true",
                    help="re-apply rc.1's 180 flip (the bug) - do not use")
    a = ap.parse_args()

    sheet, fanim = build(a.size, a.rotate)

    for d in (BUILD, WORK):
        d.mkdir(parents=True, exist_ok=True)
        sheet.save(d / f"{OUT_NAME}#sheet.png")
        with open(d / f"{OUT_NAME}#anim.fanim", "w", encoding="utf-8") as fh:
            json.dump(fanim, fh, indent=1)
            fh.write("\n")

    where = "ROTATED 180 (rc.1 bug)" if a.rotate else "un-flipped (correct)"
    print(f"{OUT_NAME}: {a.size}x{a.size} x{FRAMES} frames, {where}")
    print(f"  sheet {sheet.size}  -> build_v2081/effects/ + gord_work/effects/")
    print(f"  fanim anim key '{ANIM_KEY}' (must match the view_projectiles tag)")


if __name__ == "__main__":
    main()
