#!/usr/bin/env python3
"""Validate Chou's user-supplied 72px atlas and emit active TFM2 sprite assets.

The original atlas and JSON are preserved. Skill/Ultimate phases are split into
separate animation tags from their combined rows; baked VFX remain in the frames.
Durations are preliminary visual timings and require in-game review.
"""
from __future__ import annotations

import json
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
SOURCE = Path(__file__).resolve().parent
ACTIVE_SHEET = ROOT / "sprite" / "chou#sheet.png"
ACTIVE_ANIM = ROOT / "sprite" / "chou#anim.fanim"

# Seconds per source-frame hold. Keep the manifest's repeated-frame timings.
HOLD_SECONDS = {
    "idle": 0.080,
    "run": 0.060,
    "attack": 0.020,
    "skill": 0.040,
    "skill_recast": 0.040,
    "skill_finisher": 0.040,
    "skill2": 0.040,
    "ult": 0.060,
    "ult_recast": 0.040,
    "hit": 0.020,
    "dead": 0.050,
}


def frame_rect(row_index: int, col: int, cell_w: int, cell_h: int) -> dict[str, int]:
    return {"x": col * cell_w, "y": row_index * cell_h, "w": cell_w, "h": cell_h}


def main() -> None:
    manifest = json.loads((SOURCE / "chou_atlas.json").read_text())
    sheet_path = SOURCE / "chou_atlas.png"
    image = Image.open(sheet_path).convert("RGBA")
    cw, ch, columns = manifest["cell_width"], manifest["cell_height"], manifest["columns"]
    rows = manifest["rows"]
    if image.size != (cw * columns, ch * len(rows)):
        raise ValueError(f"Atlas dimensions {image.size} do not match manifest grid")
    if image.getchannel("A").getextrema()[0] != 0:
        raise ValueError("Expected a transparent alpha channel")

    # Map source row ranges into the exact animation clips supplied by the user.
    clips: dict[str, tuple[str, int, int]] = {
        "idle": ("idle", 0, 8),
        "run": ("run", 0, 6),
        "attack": ("attack", 0, 12),
        "skill": ("skill", 0, 16),                 # S1 cast 1: rising kick
        "skill_recast": ("skill", 17, 33),         # S1 cast 2: dash uppercut
        "skill_finisher": ("skill", 34, 50),       # S1 cast 3: ground-slam finisher
        "skill2": ("skill2", 0, 7),                # Shunpo
        "ult": ("ult", 0, 9),                      # Ultimate first kick
        "ult_recast": ("ult", 10, 34),             # recast aerial kick flurry
        "hit": ("hit", 0, 7),
        "dead": ("dead", 0, 18),
    }

    anims: dict[str, dict[str, list[dict[str, object]]]] = {}
    row_lookup = {name: index for index, name in enumerate(rows)}
    for tag, (source_row, start_col, end_col) in clips.items():
        row_data = manifest["animations"][source_row]["frames"]
        by_col = {frame["col"]: frame for frame in row_data}
        frames = []
        for col in range(start_col, end_col + 1):
            source_frame = by_col[col]
            hold = int(source_frame["hold"])
            frames.append({
                "duration": round(HOLD_SECONDS[tag] * hold, 8),
                "data": frame_rect(row_lookup[source_row], col, cw, ch),
            })
        anims[tag] = {"frames": frames}

    ACTIVE_SHEET.parent.mkdir(parents=True, exist_ok=True)
    ACTIVE_SHEET.write_bytes(sheet_path.read_bytes())
    ACTIVE_ANIM.write_text(json.dumps({"anims": anims}, indent=2) + "\n")
    print(f"Wrote {ACTIVE_SHEET.relative_to(ROOT)} ({image.size[0]}x{image.size[1]})")
    print(f"Wrote {ACTIVE_ANIM.relative_to(ROOT)} with {len(anims)} tags")
    for tag, entry in anims.items():
        total = sum(float(f["duration"]) for f in entry["frames"])
        print(f"  {tag:16} {len(entry['frames']):2} frames, {total:.3f}s")


if __name__ == "__main__":
    main()
