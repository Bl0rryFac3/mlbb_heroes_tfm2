#!/usr/bin/env python3
"""Minor Karrie VFX tune: smaller basic shot and correctly packed head marks.

Writes new suffixed assets without replacing existing sheets. Passive mark stages
are cropped directly from the supplied source atlas to avoid reusing the previously
overlaid/blank-frame derivative. Transparent vertical padding raises each icon.
"""
import json
from pathlib import Path
from PIL import Image

MOD_ROOT = Path(__file__).resolve().parents[2]
ROOT = MOD_ROOT / "effects"
IMPORT = MOD_ROOT / "source/karrie/import_2026-10-24"


def frames(base: str):
    sheet = Image.open(ROOT / f"{base}#sheet.png").convert("RGBA")
    meta = json.loads((ROOT / f"{base}#anim.fanim").read_text())
    result = []
    for tag, anim in meta["anims"].items():
        for item in anim["frames"]:
            d = item["data"]
            crop = sheet.crop((d["x"], d["y"], d["x"] + d["w"], d["y"] + d["h"]))
            result.append((tag, item["duration"], crop))
    return result


def source_mark_frames():
    source_path = IMPORT / "karrie-Lightwheel Mark and passive proc.png"
    sheet = Image.open(source_path).convert("RGBA")
    result = []
    for i in range(6):
        x0 = round(i * sheet.width / 6)
        x1 = round((i + 1) * sheet.width / 6)
        y0, y1 = 0, round(sheet.height / 2)
        image = sheet.crop((x0, y0, x1, y1))
        alpha = image.getchannel("A")
        image.putalpha(alpha.point([0 if a < 6 else a for a in range(256)]))
        bbox = image.getchannel("A").point([0 if a < 10 else 255 for a in range(256)]).getbbox()
        if not bbox:
            raise ValueError(f"Source mark frame {i + 1} is unexpectedly blank")
        result.append((f"mark_{i + 1}", 0.1, image.crop(bbox)))
    return result


def make_basic_small():

    out = "karrie_basic_lightwheel_game_small"
    outputs = [ROOT / f"{out}#sheet.png", ROOT / f"{out}#anim.fanim"]
    if any(p.exists() for p in outputs):
        raise FileExistsError(f"Refusing to overwrite existing refined basic projectile: {outputs}")
    source = frames("karrie_basic_lightwheel_game")
    w, h = 36, 20  # about 10% smaller than v1.1.18; near Gusion's 36x15 dagger frame
    sheet = Image.new("RGBA", (w * len(source), h), (0, 0, 0, 0))
    anim_frames = []
    for i, (_, duration, image) in enumerate(source):
        sheet.alpha_composite(image.resize((w, h), Image.Resampling.LANCZOS), (i * w, 0))
        anim_frames.append({"duration": duration, "data": {"x": i * w, "y": 0, "w": w, "h": h}})
    sheet.save(outputs[0])
    outputs[1].write_text(json.dumps({"anims": {"basic_projectile": {"frames": anim_frames}},
                                      "anchor_x": 0.5, "anchor_y": 0.5}, indent=2) + "\n")
    print(f"{out}: {w}x{h}")


def make_skill1_projectile_larger():
    out = "karrie_skill1_projectile_game_larger"
    outputs = [ROOT / f"{out}#sheet.png", ROOT / f"{out}#anim.fanim"]
    if any(p.exists() for p in outputs):
        raise FileExistsError(f"Refusing to overwrite existing Skill 1 sheet: {outputs}")
    source = frames("karrie_skill1_projectile_game")
    w, h = 66, 81  # about 10% larger than the current 60x74 ring projectile
    sheet = Image.new("RGBA", (w * len(source), h), (0, 0, 0, 0))
    anim_frames = []
    for i, (_, duration, image) in enumerate(source):
        sheet.alpha_composite(image.resize((w, h), Image.Resampling.LANCZOS), (i * w, 0))
        anim_frames.append({"duration": duration, "data": {"x": i * w, "y": 0, "w": w, "h": h}})
    sheet.save(outputs[0])
    outputs[1].write_text(json.dumps({"anims": {"effect": {"frames": anim_frames}},
                                      "anchor_x": 0.5, "anchor_y": 0.5}, indent=2) + "\n")
    print(f"{out}: {w}x{h}")


def make_marks_higher():
    out = "karrie_passive_marks_head_fixed"
    outputs = [ROOT / f"{out}#sheet.png", ROOT / f"{out}#anim.fanim"]
    if any(p.exists() for p in outputs):
        raise FileExistsError(f"Refusing to overwrite existing raised mark sheet: {outputs}")
    source = source_mark_frames()
    art_w, art_h = 36, 42  # about 10% smaller than the current visible 40x46 art
    frame_w, frame_h, inset_x, inset_y = 48, 142, 6, 2
    sheet = Image.new("RGBA", (frame_w * len(source), frame_h), (0, 0, 0, 0))
    anims = {}
    for i, (tag, duration, image) in enumerate(source):
        sheet.alpha_composite(image.resize((art_w, art_h), Image.Resampling.LANCZOS),
                              (i * frame_w + inset_x, inset_y))
        anims[tag] = {"frames": [{"duration": duration,
                                  "data": {"x": i * frame_w, "y": 0,
                                           "w": frame_w, "h": frame_h}}]}
    sheet.save(outputs[0])
    outputs[1].write_text(json.dumps({"anims": anims, "anchor_x": 0.5, "anchor_y": 1.0}, indent=2) + "\n")
    print(f"{out}: visible art {art_w}x{art_h}, bottom-anchored frame {frame_w}x{frame_h}")


if __name__ == "__main__":
    make_skill1_projectile_larger()
    make_marks_higher()
