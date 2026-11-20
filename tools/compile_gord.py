#!/usr/bin/env python3
"""
Compile Gord's PixelLab/ChatGPT frames into the shipped sheet + .fanim.

THE ONE RULE: frames are placed, never scaled. If a frame is already 47x47 it
goes in as 47x47. Re-scaling a crushed sprite is how detail dies.

Placement (per frame, per cell):
  - horizontal : centred  -> anchor_x 0.5
  - vertical   : the frame's LOWEST OPAQUE PIXEL lands on row 55
                 -> anchor_y 0.7639 (55/72)

Anchoring on the lowest opaque pixel enforces the board rule by construction:
if the board is the lowest thing in every frame, every frame aligns on the
board's underside automatically. It also means a frame whose lowest pixel is
something else (a thruster, a dropped shadow) will align on THAT instead --
which is why the script reports the lowest-pixel row for every frame so you
can see the disagreement instead of guessing.

Usage:
    python3 gord_work/compile_gord.py --frames gord_work/frames --out gord_work/compile

Frames layout (either works):
    gord_work/frames/<motion>/<anything-sorted>.png
    gord_work/frames/<motion>_NN.png

Motions and order are fixed by the 8-row sheet contract:
    idle, run, attack, skill, skill2, ult, hit, dead
"""
import argparse, json, os, re, sys
from PIL import Image
import numpy as np

CELL = 72
ROWS = 8
COLS = 23
FEET_ROW = 55                      # inside a 72 px cell
ANCHOR_Y = FEET_ROW / CELL         # 0.7638888888888888
MOTIONS = ["idle", "run", "attack", "skill", "skill2", "ult", "hit", "dead"]
# Virtual anims that reuse cells from a sheet row rather than owning one.
EXTRA_ANIMS = ["ult_loop"]
# First ult-row frame that is sustained channelling rather than wind-up. Frame
# deltas settle at 3-5 (<=1.0), jump hard at 5->6 (25.1) and stay high through
# the rest, so the beam pose runs from 6 to the end.
ULT_LOOP_FROM = 6

# Total motion time in seconds; divided across however many frames arrive.
# Defaults: idle/run are loops, the rest one-shot. ult = 1.28 s, inside the
# shipped roster's 0.41-1.60 s range -- NOT the canon 3.2 s channel length, which
# no shipped hero animates in full.
DEFAULT_TOTALS = {
    "idle":   1.60,
    "run":    0.60,
    "attack": 0.50,
    "skill":  0.40,
    "skill2": 0.40,
    "ult":    1.28,
    "hit":    0.21,
    "dead":   0.84,
}

MAGENTA = (255, 0, 255)


def natural_key(s):
    return [int(t) if t.isdigit() else t.lower() for t in re.split(r"(\d+)", s)]


def collect_frames(frames_root, motion):
    """Return a sorted list of paths for one motion."""
    d = os.path.join(frames_root, motion)
    if os.path.isdir(d):
        files = [os.path.join(d, f) for f in os.listdir(d)
                 if f.lower().endswith(".png")]
        return sorted(files, key=lambda p: natural_key(os.path.basename(p)))
    flat = [os.path.join(frames_root, f) for f in os.listdir(frames_root)
            if f.lower().endswith(".png")
            and os.path.basename(f).split("_")[0] == motion]
    return sorted(flat, key=lambda p: natural_key(os.path.basename(p)))


def analyse(path):
    """Open a frame, return (rgba array, bbox, lowest opaque row)."""
    im = Image.open(path)
    if im.mode != "RGBA":
        im = im.convert("RGBA")
    a = np.array(im)
    al = a[..., 3]
    op = al > 0
    if not op.any():
        return a, None, None
    ys, xs = np.nonzero(op)
    bbox = (int(xs.min()), int(ys.min()), int(xs.max()), int(ys.max()))
    return a, bbox, int(ys.max())


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--frames", default="gord_work/frames")
    ap.add_argument("--out", default="gord_work/compile")
    ap.add_argument("--name", default="gord")
    ap.add_argument("--key", default=None,
                    help="paint the cell background with this hex (e.g. ff00ff). "
                         "Default: leave transparent, like the shipped Obsidia sheet.")
    ap.add_argument("--dur", default=None,
                    help="override total seconds per motion, e.g. 'idle=2.0,ult=3.2'")
    args = ap.parse_args()

    if not os.path.isdir(args.frames):
        sys.exit("frames folder not found: %s" % args.frames)

    totals = dict(DEFAULT_TOTALS)
    if args.dur:
        for part in args.dur.split(","):
            k, v = part.split("=")
            totals[k.strip()] = float(v)

    bg = None
    if args.key:
        h = args.key.lstrip("#")
        bg = tuple(int(h[i:i + 2], 16) for i in (0, 2, 4)) + (255,)

    os.makedirs(args.out, exist_ok=True)
    W, H = COLS * CELL, ROWS * CELL           # 1656 x 576
    sheet = np.zeros((H, W, 4), dtype=np.uint8)
    if bg:
        sheet[:] = bg

    anims = {}
    report = []
    problems = []

    for r, motion in enumerate(MOTIONS):
        paths = collect_frames(args.frames, motion)
        if not paths:
            problems.append("row %d %-7s : NO FRAMES FOUND" % (r, motion))
            continue
        if len(paths) > COLS:
            problems.append("row %d %-7s : %d frames exceeds the %d-column limit"
                            % (r, motion, len(paths), COLS))

        dur = totals[motion] / len(paths)
        frames_json = []
        lowest_rows = []
        widths, heights = [], []

        for c, p in enumerate(paths):
            a, bbox, lowest = analyse(p)
            if bbox is None:
                problems.append("row %d %-7s frame %d : completely empty" % (r, motion, c))
                continue
            x0, y0, x1, y1 = bbox
            fw, fh = x1 - x0 + 1, y1 - y0 + 1
            widths.append(fw); heights.append(fh)
            lowest_rows.append(lowest)

            if fw > CELL or fh > CELL:
                problems.append("row %d %-7s frame %d : %dx%d exceeds the %d px cell"
                                % (r, motion, c, fw, fh, CELL))
                continue

            # --- placement: centred horizontally, lowest opaque pixel on row 55
            off_x = (CELL - fw) // 2 - x0
            off_y = FEET_ROW - lowest
            tx = c * CELL + off_x
            ty = r * CELL + off_y
            if tx < 0 or ty < 0 or tx + a.shape[1] > (c + 1) * CELL \
               or ty + a.shape[0] > (r + 1) * CELL:
                problems.append("row %d %-7s frame %d : placement overflows the cell"
                                % (r, motion, c))
                continue

            # paste with the frame's own alpha
            src = a
            dst = sheet[ty:ty + src.shape[0], tx:tx + src.shape[1]]
            alpha = src[..., 3:4].astype(np.float32) / 255.0
            dst[..., :3] = (src[..., :3].astype(np.float32) * alpha
                            + dst[..., :3].astype(np.float32) * (1 - alpha)).astype(np.uint8)
            dst[..., 3] = np.maximum(dst[..., 3], src[..., 3])

            frames_json.append({"duration": round(dur, 6),
                                "data": {"x": c * CELL, "y": r * CELL,
                                         "w": CELL, "h": CELL}})

        anims[motion] = {"frames": frames_json}
        if lowest_rows:
            spread = max(lowest_rows) - min(lowest_rows)
            report.append((motion, len(paths), min(lowest_rows), max(lowest_rows),
                           spread, min(widths), max(widths), min(heights), max(heights)))
            # THE board check: the anchored row must agree across frames
            # after placement, i.e. every frame's lowest pixel row is the same.
            if spread:
                problems.append(
                    "row %d %-7s : lowest opaque row varies by %d px across frames "
                    "(%d..%d) -- the sprite will jitter on the cell"
                    % (r, motion, spread, min(lowest_rows), max(lowest_rows)))

    # ---- write sheet
    sheet_path = os.path.join(args.out, "%s#sheet.png" % args.name)
    Image.fromarray(sheet, "RGBA").save(sheet_path)

    # ---- the ult channel loop -------------------------------------------
    # The data schedules `CasterAnimation ult_loop` while the beam is firing,
    # but `ult_loop` is not a sheet row of its own — the ult row already holds
    # wind-up (frames 0-5) then the sustained channelling pose (frames 6-16).
    # Emit it as a virtual anim over the sustain frames so the CasterAnimation
    # resolves to real cells: naming an anim that does not exist blanks the
    # sprite, which is how Gord shipped invisible for the whole ult.
    if "ult" in anims:
        ult_frames = anims["ult"]["frames"]
        if len(ult_frames) > ULT_LOOP_FROM:
            anims["ult_loop"] = {"frames": ult_frames[ULT_LOOP_FROM:]}

    # ---- write fanim (order matters: sheet rows are positional)
    fanim = {"anchor_x": 0.5, "anchor_y": ANCHOR_Y,
             "anims": {m: anims[m] for m in (MOTIONS + EXTRA_ANIMS) if m in anims}}
    fanim_path = os.path.join(args.out, "%s#anim.fanim" % args.name)
    with open(fanim_path, "w") as f:
        json.dump(fanim, f)

    # ---- report
    print("sheet : %s  (%dx%d, %d rows x %d cols of %d px)"
          % (sheet_path, W, H, ROWS, COLS, CELL))
    print("fanim : %s  anchor_x 0.5  anchor_y %.16f (feet row %d)"
          % (fanim_path, ANCHOR_Y, FEET_ROW))
    print()
    print("%-8s %6s %14s %7s %12s" % ("motion", "frames", "lowest-row", "spread", "size w x h"))
    print("-" * 62)
    for (m, n, lo, hi, sp, wmin, wmax, hmin, hmax) in report:
        sz = "%d x %d" % (wmin, hmin)
        if (wmin, hmin) != (wmax, hmax):
            sz = "%d-%d x %d-%d" % (wmin, wmax, hmin, hmax)
        print("%-8s %6d %8d..%-4d %7d %12s" % (m, n, lo, hi, sp, sz))
    print()
    missing = [m for m in MOTIONS if m not in anims]
    if missing:
        problems.append("missing rows: %s" % ", ".join(missing))
    if problems:
        print("PROBLEMS (%d):" % len(problems))
        for p in problems:
            print("  !", p)
    else:
        print("No problems. Run alpha_work/verify_compiled_sheet.py next.")


if __name__ == "__main__":
    main()
