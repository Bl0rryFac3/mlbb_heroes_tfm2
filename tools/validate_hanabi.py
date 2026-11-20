#!/usr/bin/env python3
"""Static validation for the Hanabi candidate (no Rust/game runtime required)."""
from __future__ import annotations
import json
import re
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
champion_path = ROOT / "champion/mlbb_hanabi.data_champion"
data = json.loads(champion_path.read_text(encoding="utf-8"))
assert data["id"] == "mlbb_hanabi"
assert data["sprite"] == "asset/mlbb_heroes/sprite/hanabi"
assert len(data["skill_icons"]) == 3
assert data["passive"]["passive_ref"] == "mlbb_heroes:hanabi_passive"

# Animation atlas bounds and timing structure.
for anim_path in [ROOT / "sprite/hanabi#anim.fanim", *sorted((ROOT / "effects").glob("hanabi_*#anim.fanim"))]:
    anim = json.loads(anim_path.read_text(encoding="utf-8"))
    sheet_path = anim_path.with_name(anim_path.name.replace("#anim.fanim", "#sheet.png"))
    image = Image.open(sheet_path)
    assert image.mode == "RGBA", (sheet_path, image.mode)
    for tag, timeline in anim["anims"].items():
        for frame in timeline["frames"]:
            rect = frame["data"]
            assert rect["w"] > 0 and rect["h"] > 0 and frame["duration"] > 0, (anim_path, tag, frame)
            assert rect["x"] >= 0 and rect["y"] >= 0
            assert rect["x"] + rect["w"] <= image.width, (anim_path, tag, frame, image.size)
            assert rect["y"] + rect["h"] <= image.height, (anim_path, tag, frame, image.size)
    assert 0.0 <= anim.get("anchor_x", .5) <= 1.0
    assert 0.0 <= anim.get("anchor_y", .5) <= 1.0

# V2 shield matches Hanabi's actor-cell size and has 15% lower processed alpha.
shield_path = ROOT / "effects/hanabi_equinox_shield#sheet.png"
shield = Image.open(shield_path)
assert shield.mode == "RGBA" and shield.size == (432, 72), (shield.mode, shield.size)
assert shield.getchannel("A").getextrema()[1] == 217, shield.getchannel("A").getextrema()
shield_anim = json.loads((ROOT / "effects/hanabi_equinox_shield#anim.fanim").read_text())
shield_frames = shield_anim["anims"]["loop"]["frames"]
assert len(shield_frames) == 6
assert all(frame["data"]["w"] == 72 and frame["data"]["h"] == 72 and frame["duration"] == .08
           for frame in shield_frames)

# Bloom stages use measured transparent gutters rather than equal-width source crops.
bloom_bounds = (0, 270, 550, 945, 1440, 1790, 2172)
raw_bloom = Image.open(ROOT / "source/hanabi/import_2026-10-28/hanabi-ultbloom.png").convert("RGBA")
assert raw_bloom.size == (2172, 724)
alpha = raw_bloom.getchannel("A")
for boundary in bloom_bounds[1:-1]:
    assert alpha.crop((boundary - 1, 0, boundary + 1, raw_bloom.height)).getbbox() is None, boundary
bloom_sheet = Image.open(ROOT / "effects/hanabi_ult_bloom#sheet.png")
assert bloom_sheet.mode == "RGBA" and bloom_sheet.size == (624, 104), bloom_sheet.size
bloom_anim = json.loads((ROOT / "effects/hanabi_ult_bloom#anim.fanim").read_text())
bloom_frames = bloom_anim["anims"]["bloom"]["frames"]
assert len(bloom_frames) == 6
assert all(frame["data"]["w"] == 104 and frame["data"]["h"] == 104
           and frame["duration"] == .07 for frame in bloom_frames)

# The persistent S2 mark is slightly smaller; its bottom anchor/timing are retained.
mark_sheet = Image.open(ROOT / "effects/hanabi_soul_mark#sheet.png")
assert mark_sheet.mode == "RGBA" and mark_sheet.size == (352, 128), mark_sheet.size
mark_anim = json.loads((ROOT / "effects/hanabi_soul_mark#anim.fanim").read_text())
assert mark_anim["anchor_x"] == .5 and mark_anim["anchor_y"] == 1.0
assert all(frame["data"]["w"] == 44 and frame["data"]["h"] == 128
           for timeline in mark_anim["anims"].values() for frame in timeline["frames"])

# Active icons must be actual transparent PNGs at the project's requested 100px size.
for icon in ["hanabi_skill1", "hanabi_skill2", "hanabi_ult"]:
    path = ROOT / f"icons/{icon}.png"
    im = Image.open(path)
    assert im.format == "PNG" and im.mode == "RGBA" and im.size == (100, 100), (path, im.format, im.mode, im.size)

# All data-bound runtime assets and tags resolve locally.
def asset_paths(section: str):
    for item in data.get(section, []):
        anim_path = item["anim"].removeprefix("asset/mlbb_heroes/")
        yield item["name"], item["tag"], ROOT / f"{anim_path}#anim.fanim", item

for section in ("view_projectiles", "view_effects", "view_buffs"):
    for name, tag, anim_path, item in asset_paths(section):
        assert anim_path.exists(), (section, name, anim_path)
        anim = json.loads(anim_path.read_text(encoding="utf-8"))
        assert tag in anim["anims"], (section, name, tag, anim_path)

# Native refs in the champion data must be registered by Hanabi and linked in lib.rs.
source = (ROOT / "src/hanabi.rs").read_text(encoding="utf-8")
lib = (ROOT / "src/lib.rs").read_text(encoding="utf-8")
for match in re.findall(r'"(mlbb_heroes:[a-z0-9_]+)"', json.dumps(data)):
    name = match.split(":", 1)[1]
    assert f'"{match}"' in source, f"missing registration: {match}"
assert "mod hanabi;" in lib and "hanabi::register(&mut module);" in lib

# Localization coverage for all five project locales and the four ability slots.
i18n = json.loads((ROOT / "text/champion.i18n").read_text(encoding="utf-8"))
for locale in ("en", "zh-hans", "zh-hant", "ko", "ja"):
    hero = i18n[locale]["description"]["mlbb_hanabi"]
    names = i18n[locale]["skill_name"]["mlbb_hanabi"]
    assert set(("name", "attack", "passive", "skill", "skill2", "ult")) <= set(hero)
    assert set(("passive", "skill1", "skill2", "ult")) <= set(names)

# Supplied image geometries are checked against the recorded source audit.
raw = ROOT / "source/hanabi/import_2026-10-28"
expected = {
    "hanabi_atlas.png": (1440, 576),
    "hanabi-basicprojectile.png": (2172, 724),
    "hanabi-higanabanault.png": (2172, 724),
    "hanabi-skill1shieldeffect.png": (2172, 724),
    "hanabi-skill2mark.png": (2172, 724),
    "hanabi-skill2projectile.png": (1220, 1289),
    "hanabi-ultbloom.png": (2172, 724),
}
for filename, size in expected.items():
    im = Image.open(raw / filename)
    assert im.mode == "RGBA" and im.size == size, (filename, im.mode, im.size)

# Animation and effect data bindings.
assert len(json.loads((ROOT / "effects/hanabi_soul_mark#anim.fanim").read_text())["anims"]["apply"]["frames"]) == 6
assert len(json.loads((ROOT / "effects/hanabi_soul_mark#anim.fanim").read_text())["anims"]["consume"]["frames"]) == 3
assert len(json.loads((ROOT / "effects/hanabi_soul_mark#anim.fanim").read_text())["anims"]["mark"]["frames"]) == 1
assert data["view_effects"][0]["repeat"] is True
assert data["view_effects"][0]["is_follow"] is True
print("Hanabi static validation passed: data, five locales, icons, source dimensions, animation bounds/tags, asset/native bindings, v2 shield, v3 measured bloom crops, and slightly smaller S2 mark.")
