#!/usr/bin/env python3
"""Create compact gameplay-sized Karrie VFX variants; preserve original art outputs.

Frames are resized independently with high-quality downsampling. Original prepared
sheets remain unchanged so the game-sized version can be adjusted or regenerated.
"""
from __future__ import annotations
import json
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
EFFECTS = ROOT / "effects"
# Frame sizes are chosen against Gusion's in-game asset frames (36x15 projectile,
# 40x42 empowered hit, 40x80 mark, 64x41 arrival) and Karrie's 72px sprite cells.
SPECS = {
    "karrie_basic_lightwheel": (40, 22),
    "karrie_ult_paired_lightwheels": (56, 36),
    "karrie_skill1_projectile": (60, 74),
    "karrie_skill1_impact": (64, 79),
    "karrie_passive_marks": (40, 46),
    "karrie_passive_proc": (56, 55),
    "karrie_ult_activation_pulse": (72, 53),
}
SUFFIX = "_game"


def convert(name: str, size: tuple[int, int]) -> tuple[int, int]:
    sheet_path = EFFECTS / f"{name}#sheet.png"
    anim_path = EFFECTS / f"{name}#anim.fanim"
    out_name = name + SUFFIX
    out_sheet_path = EFFECTS / f"{out_name}#sheet.png"
    out_anim_path = EFFECTS / f"{out_name}#anim.fanim"
    for output in (out_sheet_path, out_anim_path):
        if output.exists():
            raise FileExistsError(f"Refusing to overwrite {output}")

    sheet = Image.open(sheet_path).convert("RGBA")
    meta = json.loads(anim_path.read_text(encoding="utf-8"))
    new_w, new_h = size
    frames_out: list[tuple[str, int, Image.Image]] = []
    for tag, anim in meta["anims"].items():
        for idx, fr in enumerate(anim["frames"]):
            d = fr["data"]
            crop = sheet.crop((d["x"], d["y"], d["x"] + d["w"], d["y"] + d["h"]))
            scaled = crop.resize(size, Image.Resampling.LANCZOS)
            frames_out.append((tag, idx, scaled))

    out_sheet = Image.new("RGBA", (new_w * len(frames_out), new_h), (0, 0, 0, 0))
    new_anims: dict[str, dict] = {tag: {"frames": []} for tag in meta["anims"]}
    for index, (tag, _, image) in enumerate(frames_out):
        out_sheet.alpha_composite(image, (index * new_w, 0))
        original_tag_frames = meta["anims"][tag]["frames"]
        tag_idx = len(new_anims[tag]["frames"])
        new_anims[tag]["frames"].append({
            "duration": original_tag_frames[tag_idx]["duration"],
            "data": {"x": index * new_w, "y": 0, "w": new_w, "h": new_h},
        })

    out_meta = {"anims": new_anims, "anchor_x": meta.get("anchor_x", 0.5),
                "anchor_y": meta.get("anchor_y", 0.5)}
    if name == "karrie_passive_marks":
        # Match Gusion's overhead passive-rune anchor: center on X, bottom anchor on Y.
        out_meta["anchor_x"], out_meta["anchor_y"] = 0.5, 1.0
    out_sheet.save(out_sheet_path)
    out_anim_path.write_text(json.dumps(out_meta, indent=2) + "\n", encoding="utf-8")
    return new_w, new_h


if __name__ == "__main__":
    for asset, target in SPECS.items():
        print(f"{asset}: {convert(asset, target)}")
