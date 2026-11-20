#!/usr/bin/env python3
"""Prepare Hanabi's supplied sprite/VFX into the TFM2 fanim import format.

This script is deterministic and intentionally leaves the raw user assets under
source/hanabi/import_2026-10-28 untouched.
"""
from __future__ import annotations
import json
import math
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
RAW = ROOT / "source/hanabi/import_2026-10-28"
SPRITE = ROOT / "sprite"
EFFECTS = ROOT / "effects"


def write_json(path: Path, data: dict) -> None:
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def frame_ref(x: int, y: int, w: int, h: int, duration: float) -> dict:
    return {"duration": round(duration, 4), "data": {"x": x, "y": y, "w": w, "h": h}}


def prepare_character() -> None:
    atlas = Image.open(RAW / "hanabi_atlas.png").convert("RGBA")
    meta = json.loads((RAW / "hanabi_atlas.json").read_text(encoding="utf-8"))
    cw, ch = meta["cell_width"], meta["cell_height"]
    assert atlas.size == (cw * meta["columns"], ch * len(meta["rows"])), (atlas.size, meta)
    out_sheet = SPRITE / "hanabi#sheet.png"
    atlas.save(out_sheet, optimize=True)
    row_index = {name: i for i, name in enumerate(meta["rows"])}
    base_duration = {"idle": .12, "run": .06, "attack": .015, "skill": .015,
                     "skill2": .015, "ult": .02, "hit": .04, "dead": .035}
    anims = {}
    for name in meta["rows"]:
        frames = []
        for item in meta["animations"][name]["frames"]:
            x = int(item["col"]) * cw
            y = row_index[name] * ch
            frames.append(frame_ref(x, y, cw, ch, base_duration[name] * int(item["hold"])))
        anims[name] = {"frames": frames}
    pivot = meta["pivot"]
    write_json(SPRITE / "hanabi#anim.fanim", {
        "anims": anims,
        "anchor_x": round(float(pivot["feet_x"]) / cw, 6),
        "anchor_y": round(float(pivot["ground_y"]) / ch, 6),
    })


def grid_frames(path: Path, columns: int, rows: int) -> list[Image.Image]:
    im = Image.open(path).convert("RGBA")
    w, h = im.size
    frames: list[Image.Image] = []
    for r in range(rows):
        y0, y1 = round(r * h / rows), round((r + 1) * h / rows)
        for c in range(columns):
            x0, x1 = round(c * w / columns), round((c + 1) * w / columns)
            frames.append(im.crop((x0, y0, x1, y1)))
    return frames


def horizontal_frames_at_gutters(path: Path, boundaries: tuple[int, ...]) -> list[Image.Image]:
    """Crop source stages at measured transparent gutters (not equal-width guesses)."""
    im = Image.open(path).convert("RGBA")
    assert boundaries[0] == 0 and boundaries[-1] == im.width, (boundaries, im.size)
    assert tuple(sorted(boundaries)) == boundaries and len(boundaries) >= 3, boundaries
    alpha = im.getchannel("A")
    for boundary in boundaries[1:-1]:
        # The adjacent source columns are transparent, keeping artwork out of
        # both neighboring frames; the selected gutters were measured by image audit.
        assert alpha.crop((boundary - 1, 0, boundary + 1, im.height)).getbbox() is None, boundary
    return [im.crop((left, 0, right, im.height))
            for left, right in zip(boundaries, boundaries[1:])]


def fitted_frame(src: Image.Image, canvas: tuple[int, int], content: tuple[int, int], y_offset: int = 0) -> Image.Image:
    src = src.convert("RGBA")
    alpha = src.getchannel("A")
    # Retain soft antialias/glow while ignoring nearly transparent export dust.
    bbox = alpha.point(lambda p: 255 if p >= 5 else 0).getbbox()
    if bbox:
        src = src.crop(bbox)
    src.thumbnail(content, Image.Resampling.LANCZOS)
    out = Image.new("RGBA", canvas, (0, 0, 0, 0))
    x = (canvas[0] - src.width) // 2
    y = (canvas[1] - src.height) // 2 + y_offset
    out.alpha_composite(src, (x, y))
    return out


def pack_effect(name: str, frames: list[Image.Image], canvas: tuple[int, int],
                content: tuple[int, int], tag: str, durations: list[float] | float,
                anchor: tuple[float, float] = (.5, .5), repeat: bool = False,
                y_offset: int = 0, tag_frames: dict[str, list[tuple[int, float]]] | None = None,
                opacity: float = 1.0) -> None:
    if not 0.0 <= opacity <= 1.0:
        raise ValueError(f"opacity must be between 0 and 1, got {opacity}")
    if isinstance(durations, float):
        durations = [durations] * len(frames)
    assert len(durations) == len(frames), (name, len(frames), len(durations))
    sheet = Image.new("RGBA", (canvas[0] * len(frames), canvas[1]), (0, 0, 0, 0))
    refs = []
    for i, (src, duration) in enumerate(zip(frames, durations)):
        frame = fitted_frame(src, canvas, content, y_offset)
        if opacity < 1.0:
            alpha = frame.getchannel("A")
            frame.putalpha(alpha.point(lambda value: int(value * opacity + 0.5)))
        sheet.alpha_composite(frame, (i * canvas[0], 0))
        refs.append(frame_ref(i * canvas[0], 0, canvas[0], canvas[1], duration))
    sheet.save(EFFECTS / f"{name}#sheet.png", optimize=True)
    anims = {tag: {"frames": refs}}
    if tag_frames:
        for extra_tag, entries in tag_frames.items():
            anims[extra_tag] = {"frames": [frame_ref(index * canvas[0], 0, canvas[0], canvas[1], duration)
                                           for index, duration in entries]}
    anim = {"anims": anims, "anchor_x": anchor[0], "anchor_y": anchor[1]}
    write_json(EFFECTS / f"{name}#anim.fanim", anim)


def prepare_effects() -> None:
    # Raw VFX sheets are hand-supplied, full-color RGBA. The visual frame grids
    # are counted from the artwork: six horizontal frames except the 3x4 scroll.
    basic = grid_frames(RAW / "hanabi-basicprojectile.png", 6, 1)
    scroll = grid_frames(RAW / "hanabi-skill2projectile.png", 3, 4)
    ult = grid_frames(RAW / "hanabi-higanabanault.png", 6, 1)
    # Six visually distinct stages are separated by transparent vertical gutters.
    # Equal 362px slicing cut through several blooms, so use audited boundaries.
    bloom = horizontal_frames_at_gutters(
        RAW / "hanabi-ultbloom.png", (0, 270, 550, 945, 1440, 1790, 2172))
    shield = grid_frames(RAW / "hanabi-skill1shieldeffect.png", 6, 1)
    mark = grid_frames(RAW / "hanabi-skill2mark.png", 8, 1)

    pack_effect("hanabi_basic_petal", basic, (36, 36), (32, 32), "petal", .06, repeat=True)
    pack_effect("hanabi_soul_scroll", scroll, (56, 48), (50, 42), "scroll", .06, repeat=True)
    pack_effect("hanabi_ult_projectile", ult, (52, 52), (46, 46), "projectile", .06, repeat=True)
    pack_effect("hanabi_ult_bloom", bloom, (104, 104), (98, 98), "bloom", .07, repeat=False)
    # Match the shield canvas to Hanabi's 72px actor cell and soften its opacity.
    pack_effect("hanabi_equinox_shield", shield, (72, 72), (68, 68), "loop", .08,
                repeat=True, opacity=.85)

    # Eight source stages: five steady floral-seal frames, a bright flash, and
    # two breakup frames. The persistent marker is bottom-anchored above the
    # target like the existing Karrie head-marker; the one-shots stay centered.
    # Slightly reduce only the persistent overhead marker; retain its bottom anchor.
    pack_effect("hanabi_soul_mark", mark, (44, 128), (36, 36), "mark", .1,
                anchor=(.5, 1.0), repeat=False, y_offset=-31,
                tag_frames={"mark": [(2, .1)],
                            "apply": [(i, .055) for i in range(6)],
                            "consume": [(5, .055), (6, .055), (7, .055)]})
    pack_effect("hanabi_soul_mark_burst", mark, (52, 52), (46, 46), "unused", .1,
                anchor=(.5, .5), repeat=False,
                tag_frames={"apply": [(i, .055) for i in range(6)],
                            "consume": [(5, .055), (6, .055), (7, .055)]})


if __name__ == "__main__":
    SPRITE.mkdir(parents=True, exist_ok=True)
    EFFECTS.mkdir(parents=True, exist_ok=True)
    prepare_character()
    prepare_effects()
    print("Prepared Hanabi character sprite and six supplied VFX families.")
